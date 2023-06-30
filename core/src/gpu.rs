pub struct Gpu {
    index: usize,
    adapter: wgpu::Adapter,
    shader_module: wgpu::ShaderModule,
    device: wgpu::Device,
    queue: wgpu::Queue,
    entity_buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl Gpu {
    pub async fn new(wgpu_instance: &wgpu::Instance, index: usize) -> Self {
        let adapter = wgpu_instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let shader_module = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let entity_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Entity Buffer"),
            size: 4,
            usage: wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("System Resources Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("System Resources Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &entity_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        return Gpu {
            index,
            device,
            queue,
            adapter,
            shader_module,
            entity_buffer,
            bind_group_layout,
            bind_group,
        };
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    pub fn shader_module(&self) -> &wgpu::ShaderModule {
        &self.shader_module
    }

    pub fn system_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn system_bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
