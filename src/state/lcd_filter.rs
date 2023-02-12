use cgmath::*;
use winit::window::Window;

use crate::{texture::Texture, Options};

use super::{
    app_state::AppContext,
    game_state,
    gpu_state::{self},
};

// ---------------------------------------------------------------------------------------------------------------------

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct LcdUniformData {
    camera_position: Point2<f32>,
    viewport_size: Vector2<f32>,
    pixels_per_unit: Vector2<f32>,
    pixel_effect_alpha: f32,
    shadow_effect_alpha: f32,
    color_attachment_layer_index: u32,
    color_attachment_layer_count: u32,
    color_attachment_history_count: u32,
    padding_: u32,
}

unsafe impl bytemuck::Pod for LcdUniformData {}
unsafe impl bytemuck::Zeroable for LcdUniformData {}

impl Default for LcdUniformData {
    fn default() -> Self {
        Self {
            camera_position: point2(0.0, 0.0),
            viewport_size: vec2(1.0, 1.0),
            pixels_per_unit: vec2(1.0, 1.0),
            pixel_effect_alpha: 1.0,
            shadow_effect_alpha: 1.0,
            color_attachment_layer_index: 0,
            color_attachment_layer_count: 1,
            color_attachment_history_count: 0,
            padding_: 0,
        }
    }
}

impl LcdUniformData {
    pub fn set_pixel_effect_alpha(&mut self, pixel_effect_alpha: f32) -> &mut Self {
        self.pixel_effect_alpha = pixel_effect_alpha;
        self
    }

    pub fn set_shadow_effect_alpha(&mut self, shadow_effect_alpha: f32) -> &mut Self {
        self.shadow_effect_alpha = shadow_effect_alpha;
        self
    }

    pub fn set_camera_position(&mut self, camera_position: Point2<f32>) -> &mut Self {
        self.camera_position = camera_position;
        self
    }

    pub fn set_viewport_size(&mut self, viewport_size: Vector2<f32>) -> &mut Self {
        self.viewport_size = viewport_size;
        self
    }

    pub fn set_pixels_per_unit(&mut self, pixels_per_unit: Vector2<f32>) -> &mut Self {
        self.pixels_per_unit = pixels_per_unit;
        self
    }

    pub fn set_color_attachment_layer_index(&mut self, index: u32) -> &mut Self {
        self.color_attachment_layer_index = index;
        self
    }

    pub fn set_color_attachment_layer_count(&mut self, count: u32) -> &mut Self {
        self.color_attachment_layer_count = count;
        self
    }

    pub fn set_color_attachment_history_count(&mut self, count: u32) -> &mut Self {
        self.color_attachment_history_count = count;
        self
    }
}

pub type LcdUniforms = crate::util::UniformWrapper<LcdUniformData>;

// ---------------------------------------------------------------------------------------------------------------------

pub struct LcdFilter {
    textures_bind_group_layout: wgpu::BindGroupLayout,
    textures_bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    tonemap: Texture,
    uniforms: LcdUniforms,
    lcd_hysteresis: Option<std::time::Duration>,
    frames_available_for_hysteresis: usize,
}

impl LcdFilter {
    pub const DEFAULT_HYSTERESIS: std::time::Duration = std::time::Duration::from_millis(65);

    pub fn new(gpu: &mut gpu_state::GpuState, options: &Options, tonemap: Texture) -> Self {
        let textures_bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("LcdFilter Bind Group Layout"),
                    entries: &[
                        // Color attachment
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2Array,
                            },
                            count: None,
                        },
                        // Tonemap
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                        // Sampler
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                            count: None,
                        },
                    ],
                });

        let textures_bind_group =
            Self::create_textures_bind_group(gpu, &textures_bind_group_layout, &tonemap.view);

        let uniforms = LcdUniforms::new(&gpu.device);

        let pipeline = Self::create_render_pipeline(
            &gpu.device,
            gpu.config.format,
            &textures_bind_group_layout,
            &uniforms.bind_group_layout,
        );

        let lcd_hysteresis = (!options.no_hysteresis).then(|| Self::DEFAULT_HYSTERESIS);

        Self {
            textures_bind_group_layout,
            textures_bind_group,
            pipeline,
            tonemap,
            uniforms,
            lcd_hysteresis,
            frames_available_for_hysteresis: 0,
        }
    }

    pub fn set_lcd_hysteresis(&mut self, hysteresis: Option<std::time::Duration>) {
        self.lcd_hysteresis = hysteresis;
    }

    pub fn lcd_hysteresis(&self) -> Option<std::time::Duration> {
        self.lcd_hysteresis
    }

    fn create_textures_bind_group(
        gpu: &gpu_state::GpuState,
        layout: &wgpu::BindGroupLayout,
        tonemap: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&gpu.color_attachment.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(tonemap),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&gpu.color_attachment.sampler),
                },
            ],
            label: Some("LcdFilter Bind Group"),
        })
    }

    fn create_render_pipeline(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        textures_bind_group_layout: &wgpu::BindGroupLayout,
        uniforms_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let lcd_wgsl = wgpu::include_wgsl!("../shaders/lcd.wgsl");
        let shader = device.create_shader_module(lcd_wgsl);

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("LcdFilter Render Pipeline Layout"),
            bind_group_layouts: &[textures_bind_group_layout, uniforms_bind_group_layout],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("LcdFilter Render Pipeline"),
            layout: Some(&layout),

            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "lcd_vs_main",
                buffers: &[],
            },

            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "lcd_fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),

            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },

            depth_stencil: None,

            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },

            multiview: None,
        })
    }

    pub fn resize(
        &mut self,
        _window: &Window,
        _new_size: winit::dpi::PhysicalSize<u32>,
        gpu: &gpu_state::GpuState,
    ) {
        // new color buffer means we lose our history sample range
        self.frames_available_for_hysteresis = 0;
        self.textures_bind_group = Self::create_textures_bind_group(
            gpu,
            &self.textures_bind_group_layout,
            &self.tonemap.view,
        );
    }

    pub fn update(&mut self, ctx: &mut AppContext, game: &game_state::GameState) {
        // Determine an appropriate alpha for pixel effects - as window gets
        // smaller the effect needs to fade out, since it looks busy on small windows.
        // NOTE: min_high_freq and max_high_freq were determined via experimentation
        let pixel_effect_alpha = {
            let frequency = (game.camera_controller.projection.scale() * game.pixels_per_unit.x)
                / ctx.gpu.config.width as f32;

            let min_high_freq = 0.2;
            let max_high_freq = 0.5;
            let falloff =
                ((frequency - min_high_freq) / (max_high_freq - min_high_freq)).clamp(0.0, 1.0);
            1.0 - (falloff * falloff)
        };

        let layer_count = ctx.gpu.color_attachment.layer_array_views.len() as u32;
        let current_layer = ctx.frame_idx % layer_count;
        let history_count = self
            .lcd_hysteresis
            .map_or_else(
                || 1,
                |hysteresis| {
                    (hysteresis.as_secs_f32() / ctx.real_delta_time.as_secs_f32()).ceil() as u32
                },
            )
            .min(self.frames_available_for_hysteresis as u32)
            .max(1_u32);

        self.uniforms
            .data
            .set_pixel_effect_alpha(pixel_effect_alpha)
            .set_camera_position(game.camera_controller.camera.position().xy())
            .set_pixels_per_unit(game.pixels_per_unit)
            .set_viewport_size(game.camera_controller.projection.viewport_size())
            .set_color_attachment_layer_index(current_layer)
            .set_color_attachment_layer_count(layer_count)
            .set_color_attachment_history_count(history_count);

        self.uniforms.write(&mut ctx.gpu.queue);
    }

    pub fn render(
        &mut self,
        _window: &Window,
        gpu: &mut gpu_state::GpuState,
        output: &wgpu::SurfaceTexture,
        encoder: &mut wgpu::CommandEncoder,
        _frame_index: usize,
    ) {
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("LcdFilter Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.textures_bind_group, &[]);
        render_pass.set_bind_group(1, &self.uniforms.bind_group, &[]);
        render_pass.draw(0..3, 0..1); //FSQ

        self.frames_available_for_hysteresis = (self.frames_available_for_hysteresis + 1)
            .min(gpu.color_attachment.layer_array_views.len());
    }
}
