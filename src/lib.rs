use ovis_core::{
    add_job_dependency, register_entity_component, register_job, wgpu, EntityComponent, Error,
    JobId, JobKind, ResourceAccess, ResourceId, SceneState, SystemResources,
};
use ovis_macros::EntityComponent;

#[derive(EntityComponent)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

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
        POSITION_ID = register_entity_component::<Position>("ovis::runtime::Position");
        CLEAR_SURFACE_ID = register_job(JobKind::Update, clear_surface, &[]);
        DRAW_TRIANGLES_ID = register_job(
            JobKind::Update,
            draw_triangles,
            &[ResourceAccess::Read(POSITION_ID)],
        );
        add_job_dependency(DRAW_TRIANGLES_ID, CLEAR_SURFACE_ID);
    }
}
