use cgmath::prelude::*;
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalPosition, event::*, window::Window};

use crate::camera;
use crate::sprite;
use crate::sprite::{DrawSprite, Vertex};
use crate::texture;

// ---------------------------------------------------------------------------------------------------------------------

fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_descs: &[wgpu::VertexBufferDescriptor],
    vs_src: wgpu::ShaderModuleSource,
    fs_src: wgpu::ShaderModuleSource,
) -> wgpu::RenderPipeline {
    let vs_module = device.create_shader_module(vs_src);
    let fs_module = device.create_shader_module(fs_src);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&layout),
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &vs_module,
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &fs_module,
            entry_point: "main",
        }),
        rasterization_state: Some(wgpu::RasterizationStateDescriptor {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::Back,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
            clamp_depth: false,
        }),
        primitive_topology: wgpu::PrimitiveTopology::TriangleList,
        color_states: &[wgpu::ColorStateDescriptor {
            format: color_format,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL,
        }],
        depth_stencil_state: depth_format.map(|format| wgpu::DepthStencilStateDescriptor {
            format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilStateDescriptor::default(),
        }),
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint32,
            vertex_buffers: vertex_descs,
        },
    })
}

// --------------------------------------------------------------------------------------------------------------------

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct CameraUniforms {
    // use vec4 for 16-byte spacing requirement
    view_position: cgmath::Vector4<f32>,
    view_proj: cgmath::Matrix4<f32>,
}

unsafe impl bytemuck::Pod for CameraUniforms {}
unsafe impl bytemuck::Zeroable for CameraUniforms {}

impl CameraUniforms {
    fn new() -> Self {
        Self {
            view_position: Zero::zero(),
            view_proj: cgmath::Matrix4::identity(),
        }
    }

    fn update_view_proj(&mut self, camera: &camera::Camera, projection: &camera::Projection) {
        self.view_position = camera.position.to_homogeneous(); // converts to vec4
        self.view_proj = projection.calc_matrix() * camera.calc_matrix();
    }
}

// --------------------------------------------------------------------------------------------------------------------

pub struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    depth_texture: texture::Texture,

    camera: camera::Camera,
    projection: camera::Projection,
    camera_controller: camera::CameraController,
    last_mouse_pos: PhysicalPosition<f64>,
    mouse_pressed: bool,

    camera_uniforms: CameraUniforms,
    camera_uniform_buffer: wgpu::Buffer,
    camera_uniform_bind_group: wgpu::BindGroup,

    sprites: sprite::SpriteCollection,
}

impl State {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    shader_validation: true,
                },
                None,
            )
            .await
            .unwrap();

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let depth_texture =
            texture::Texture::create_depth_texture(&device, &sc_desc, "depth_texture");

        let material_bind_group_layout = sprite::SpriteMaterial::bind_group_layout(&device);

        let camera = camera::Camera::new((0.0, 5.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));
        let projection =
            camera::Projection::new(sc_desc.width, sc_desc.height, cgmath::Deg(45.0), 0.1, 100.0);
        let camera_controller = camera::CameraController::new(4.0, 0.4);

        let mut camera_uniforms = CameraUniforms::new();
        camera_uniforms.update_view_proj(&camera, &projection);

        let camera_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniforms]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let camera_uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer {
                        dynamic: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("uniform_bind_group_layout"),
            });

        let camera_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(camera_uniform_buffer.slice(..)),
            }],
            label: Some("uniform_bind_group"),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &material_bind_group_layout,
                    &camera_uniform_bind_group_layout,
                ],
                label: Some("Render Pipeline Layout"),
                push_constant_ranges: &[],
            });

        let render_pipeline = create_render_pipeline(
            &device,
            &render_pipeline_layout,
            sc_desc.format,
            Some(texture::Texture::DEPTH_FORMAT),
            &[sprite::SpriteVertex::desc()],
            wgpu::include_spirv!("shaders/sprite.vs.spv"),
            wgpu::include_spirv!("shaders/sprite.fs.spv"),
        );

        let sprites = {
            let mat = {
                let diffuse_bytes = include_bytes!("../res/cobble-diffuse.png");
                let diffuse_texture = texture::Texture::from_bytes(
                    &device,
                    &queue,
                    diffuse_bytes,
                    "res/cobble-diffuse",
                    false,
                )
                .unwrap();
                sprite::SpriteMaterial::new(
                    &device,
                    "Sprite Material",
                    diffuse_texture,
                    &material_bind_group_layout,
                )
            };
            let sb1 =
                sprite::SpriteDesc::new(0.0, 0.0, 10.0, 10.0, 0.0, [1.0, 0.5, 0.5, 1.0].into());
            let sb2 =
                sprite::SpriteDesc::new(10.0, 0.0, 15.0, 5.0, 0.0, [0.5, 0.5, 1.0, 1.0].into());
            let sm = sprite::SpriteMesh::new(&vec![sb1, sb2], 0, &device, "Sprite Mesh");
            sprite::SpriteCollection::new(vec![sm], vec![mat])
        };

        Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            render_pipeline,
            depth_texture,
            size,

            camera,
            camera_controller,
            projection,

            camera_uniforms: camera_uniforms,
            camera_uniform_buffer: camera_uniform_buffer,
            camera_uniform_bind_group: camera_uniform_bind_group,

            sprites,

            last_mouse_pos: (0, 0).into(),
            mouse_pressed: false,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.depth_texture =
            texture::Texture::create_depth_texture(&self.device, &self.sc_desc, "depth_texture");
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
        self.projection.resize(new_size.width, new_size.height);
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(key),
                        state,
                        ..
                    },
                ..
            } => self.camera_controller.process_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.mouse_pressed = *state == ElementState::Pressed;
                true
            }
            WindowEvent::CursorMoved { position, .. } => {
                let mouse_dx = position.x - self.last_mouse_pos.x;
                let mouse_dy = position.y - self.last_mouse_pos.y;
                self.last_mouse_pos = *position;
                if self.mouse_pressed {
                    self.camera_controller.process_mouse(mouse_dx, mouse_dy);
                }
                true
            }
            _ => false,
        }
    }

    pub fn update(&mut self, dt: std::time::Duration) {
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera_uniforms
            .update_view_proj(&self.camera, &self.projection);
        self.queue.write_buffer(
            &self.camera_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniforms]),
        );
    }

    pub fn render(&mut self) {
        let frame = self
            .swap_chain
            .get_current_frame()
            .expect("Timeout getting texture")
            .output;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
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
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw_sprite_collection(&self.sprites, &self.camera_uniform_bind_group);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }
}
