use cgmath::prelude::*;
// use imgui::*;
// use imgui_winit_support::WinitPlatform;
use wgpu::util::DeviceExt;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyboardInput, MouseButton, WindowEvent},
    window::Window,
};

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
            // Since we're rendering sprites, we don't care about backface culling
            front_face: wgpu::FrontFace::Cw,
            cull_mode: wgpu::CullMode::None,
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

#[derive(Copy, Clone, Debug)]
struct UiDisplayState {
    camera_position: cgmath::Point3<f32>,
    button_press_count: u32,
}

impl Default for UiDisplayState {
    fn default() -> Self {
        UiDisplayState {
            camera_position: [0.0, 0.0, 0.0].into(),
            button_press_count: 0,
        }
    }
}

struct UiActionState {
    did_press_button: bool,
}

impl Default for UiActionState {
    fn default() -> Self {
        UiActionState {
            did_press_button: false,
        }
    }
}

// --------------------------------------------------------------------------------------------------------------------

pub struct State {
    // Basic mechanism
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: winit::dpi::PhysicalSize<u32>,
    depth_texture: texture::Texture,

    // Camera and input state
    camera: camera::Camera,
    projection: camera::Projection,
    camera_controller: camera::CameraController,
    last_mouse_pos: PhysicalPosition<f64>,
    mouse_pressed: bool,
    camera_uniforms: CameraUniforms,
    camera_uniform_buffer: wgpu::Buffer,
    camera_uniform_bind_group: wgpu::BindGroup,

    // Sprite rendering
    sprite_render_pipeline: wgpu::RenderPipeline,
    sprites: sprite::SpriteCollection,

    // Imgui
    winit_platform: imgui_winit_support::WinitPlatform,
    imgui: imgui::Context,
    imgui_renderer: imgui_wgpu::Renderer,
    ui_display_state: UiDisplayState,
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

        // Buyild camera, and camera uniform storage

        let camera = camera::Camera::new((0.0, 0.0, 0.0), (0.0, 0.0, 1.0));
        let projection =
            camera::Projection::new(sc_desc.width, sc_desc.height, cgmath::Deg(45.), 0.1, 100.0);
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

        // Build the sprite render pipeline

        let material_bind_group_layout = sprite::SpriteMaterial::bind_group_layout(&device);

        let sprite_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &material_bind_group_layout,
                    &camera_uniform_bind_group_layout,
                ],
                label: Some("Render Pipeline Layout"),
                push_constant_ranges: &[],
            });

        let sprite_render_pipeline = create_render_pipeline(
            &device,
            &sprite_render_pipeline_layout,
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
                sprite::SpriteDesc::new(0.0, 0.0, 10.0, 10.0, 10.0, [1.0, 0.5, 0.5, 1.0].into());
            let sb2 = sprite::SpriteDesc::new(
                -10.0,
                -10.0,
                10.0,
                10.0,
                10.0,
                [0.5, 0.5, 1.0, 1.0].into(),
            );
            let sm = sprite::SpriteMesh::new(&vec![sb1, sb2], 0, &device, "Sprite Mesh");
            sprite::SpriteCollection::new(vec![sm], vec![mat])
        };

        // set up imgui

        let hidpi_factor = window.scale_factor();
        let mut imgui = imgui::Context::create();
        let mut winit_platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
        winit_platform.attach_window(
            imgui.io_mut(),
            &window,
            imgui_winit_support::HiDpiMode::Default,
        );
        imgui.set_ini_filename(None);

        let font_size = (13.0 * hidpi_factor) as f32;
        imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        imgui
            .fonts()
            .add_font(&[imgui::FontSource::DefaultFontData {
                config: Some(imgui::FontConfig {
                    oversample_h: 1,
                    pixel_snap_h: true,
                    size_pixels: font_size,
                    ..Default::default()
                }),
            }]);

        let imgui_renderer = imgui_wgpu::RendererConfig::new()
            .set_texture_format(sc_desc.format)
            .build(&mut imgui, &device, &queue);

        Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            depth_texture,
            size,

            camera,
            camera_controller,
            projection,
            last_mouse_pos: (0, 0).into(),
            mouse_pressed: false,

            camera_uniforms,
            camera_uniform_buffer,
            camera_uniform_bind_group,

            sprite_render_pipeline,
            sprites,

            winit_platform,
            imgui,
            imgui_renderer,
            ui_display_state: UiDisplayState::default(),
        }
    }

    pub fn event(&mut self, window: &Window, event: &winit::event::Event<()>) {
        self.winit_platform
            .handle_event(self.imgui.io_mut(), &window, &event);
    }

    pub fn resize(&mut self, _window: &Window, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.depth_texture =
            texture::Texture::create_depth_texture(&self.device, &self.sc_desc, "depth_texture");
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
        self.projection.resize(new_size.width, new_size.height);
    }

    pub fn input(&mut self, _window: &Window, event: &WindowEvent) -> bool {
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

    pub fn update(&mut self, window: &Window, dt: std::time::Duration) {
        self.imgui.io_mut().update_delta_time(dt);

        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera_uniforms
            .update_view_proj(&self.camera, &self.projection);
        self.queue.write_buffer(
            &self.camera_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniforms]),
        );

        self.update_ui_display_state(window, dt)
    }

    fn update_ui_display_state(&mut self, _window: &Window, _dt: std::time::Duration) {
        self.ui_display_state.camera_position = self.camera.position;
    }

    pub fn render(&mut self, window: &Window) {
        let frame = self
            .swap_chain
            .get_current_frame()
            .expect("Timeout getting texture");

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        //
        // Render Sprites; this is first pass so we clear color/depth
        //

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.output.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
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

            render_pass.set_pipeline(&self.sprite_render_pipeline);
            render_pass.draw_sprite_collection(&self.sprites, &self.camera_uniform_bind_group);
        }

        //
        //  ImGUI
        //

        {
            self.winit_platform
                .prepare_frame(self.imgui.io_mut(), window)
                .expect("Failed to prepare frame");

            let ui = self.imgui.frame();

            //
            // Update imgui content
            //

            let mut ui_action_state = UiActionState::default();
            let ui_display_state = self.ui_display_state;
            imgui::Window::new(imgui::im_str!("Hello"))
                .size([280.0, 128.0], imgui::Condition::FirstUseEver)
                .build(&ui, || {
                    ui.text(imgui::im_str!(
                        "camera: ({:.2},{:.2},{:.2})",
                        ui_display_state.camera_position.x,
                        ui_display_state.camera_position.y,
                        ui_display_state.camera_position.z
                    ));

                    if ui.button(imgui::im_str!("Button"), [200.0, 24.0]) {
                        ui_action_state.did_press_button = true;
                    }

                    ui.text(imgui::im_str!(
                        "Button presses: {}",
                        ui_display_state.button_press_count
                    ));
                });

            if ui_action_state.did_press_button {
                self.ui_display_state.button_press_count += 1;
            }

            //
            // Render imgui content
            //

            self.winit_platform.prepare_render(&ui, &window);

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.output.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Do not clear
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            self.imgui_renderer
                .render(ui.render(), &self.queue, &self.device, &mut render_pass)
                .expect("Imgui render failed");
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }
}
