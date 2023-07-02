use ovis_core::{
    add_job_dependency, register_job, wgpu, EntityId, Error, JobId, JobKind, Resource,
    ResourceAccess, SceneState, SystemResources,
};
use ovis_macros::resource;

pub type Vec3 = glam::Vec3A;
pub type Quat = glam::Quat;
pub type Affine3A = glam::Affine3A;
pub type Mat4 = glam::Mat4;

#[resource(EntityComponent)]
pub struct LocalToParent(Affine3A);

impl std::ops::Deref for LocalToParent {
    type Target = Affine3A;

    fn deref(&self) -> &Self::Target {
        return &self.0;
    }
}

#[resource(EntityComponent)]
pub struct LocalToWorld(Affine3A);

#[resource(EntityComponent)]
pub type WorldToCamera = Affine3A;

#[resource(EntityComponent)]
pub type CameraToClip = Mat4;

#[resource(EntityComponent)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scaling: Vec3,
}

#[resource(EntityComponent)]
pub struct Camera {
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

pub type ActiveCamera = EntityId;

// #[job]
fn calculate_local_to_parent(transform: &Transform) -> LocalToParent {
    return LocalToParent(Affine3A::from_scale_rotation_translation(
        transform.scaling.into(),
        transform.rotation,
        transform.translation.into(),
    ));
}

fn calculate_local_to_world(
    local_to_parent: &LocalToParent,
    parent_local_to_world: &LocalToWorld,
) -> LocalToWorld {
    LocalToWorld(**local_to_parent)
    // parent_local_to_world.0
    // local_to_parent.
    // return LocalToWorld(local_to_parent.0 * parent_local_to_world.0);
}

#[resource(EntityComponent)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

// pub fn foo(x: &mut Mat4) {
// x = Mat4::perspective_lh(1.0, 1.0, 1.0, 1.0).into();
// x.perspective_lh();
// x.inner.

// ViewToClip::perspective_lh(1.0, 1.0, 1.0, 1.0);
// }

static mut CLEAR_SURFACE_ID: JobId = JobId::from_index_and_version(0, 0);
pub fn clear_surface(sr: &SystemResources, _s: &SceneState) -> Result<(), Error> {
    let viewport = sr.viewport().unwrap();

    let gpu = viewport.gpu();
    let mut encoder = gpu
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("") });
    let color_attachment = viewport
        .texture_view()
        .map(|view| wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.1,
                    g: 0.2,
                    b: 0.3,
                    a: 1.0,
                }),
                store: true,
            },
        });

    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("ClearSurface"),
        color_attachments: &[color_attachment],
        depth_stencil_attachment: None,
    });
    gpu.queue().submit(std::iter::once(encoder.finish()));

    Ok(())
}

static mut DRAW_TRIANGLES_ID: JobId = JobId::from_index_and_version(0, 0);
pub fn draw_triangles(sr: &SystemResources, s: &SceneState) -> Result<(), Error> {
    let viewport = sr.viewport().unwrap();

    let gpu = viewport.gpu();
    let mut encoder = gpu
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("") });

    let color_attachment = viewport
        .texture_view()
        .map(|view| wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: true,
            },
        });
    {
        let position_storage = s.resource_storage_mut::<Position>().unwrap(); // TODO: mut not necessary here
        position_storage.update_gpu_buffers();

        // for (id, p) in position_storage.iter() {
        //     println!("{}: ({}, {})", id, p.x, p.y);
        // }

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("DrawTriangles"),
            color_attachments: &[color_attachment],
            depth_stencil_attachment: None,
        });
        // render_pass.set_push_constants
        render_pass.set_pipeline(sr.pipeline().unwrap());
        render_pass.set_bind_group(0, viewport.gpu().system_bind_group(), &[]);
        render_pass.set_bind_group(1, s.resource_bind_group(viewport.gpu().index()), &[]);

        render_pass.draw(0..3, 1..2);
    }
    gpu.queue().submit(std::iter::once(encoder.finish()));

    Ok(())
}

pub fn load_runtime() {
    unsafe {
        Position::register();
        // POSITION_ID = register_entity_component::<Position>("ovis::runtime::Position");
        CLEAR_SURFACE_ID = register_job(JobKind::Update, clear_surface, &[]);
        DRAW_TRIANGLES_ID = register_job(
            JobKind::Update,
            draw_triangles,
            &[ResourceAccess::Read(POSITION_ID)],
        );
        add_job_dependency(DRAW_TRIANGLES_ID, CLEAR_SURFACE_ID);
    }
}
