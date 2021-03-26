use cgmath::*;
use winit::window::Window;

use crate::{texture::Texture, Options};

use super::{app_state::AppContext, gpu_state};

// ---------------------------------------------------------------------------------------------------------------------

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct LcdUniformData {
    pixels_per_unit: Vector2<f32>,
    palette_shift: f32,
}

unsafe impl bytemuck::Pod for LcdUniformData {}
unsafe impl bytemuck::Zeroable for LcdUniformData {}

impl Default for LcdUniformData {
    fn default() -> Self {
        Self {
            pixels_per_unit: vec2(1.0, 1.0),
            palette_shift: 0.0,
        }
    }
}

impl LcdUniformData {
    pub fn set_sprite_size_px(&mut self, pixels_per_unit: Vector2<f32>) -> &mut Self {
        self.pixels_per_unit = pixels_per_unit;
        self
    }

    pub fn set_palette_shift(&mut self, palette_shift: f32) -> &mut Self {
        self.palette_shift = palette_shift.clamp(-1.0, 1.0);
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
}

impl LcdFilter {
    pub fn new(gpu: &mut gpu_state::GpuState, _options: &Options, tonemap: Texture) -> Self {
        let textures_bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("LcdFilter Bind Group Layout"),
                    entries: &[
                        // Color attachment
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStage::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                        // Tonemap
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStage::FRAGMENT,
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
                            visibility: wgpu::ShaderStage::FRAGMENT,
                            ty: wgpu::BindingType::Sampler {
                                comparison: false,
                                filtering: false,
                            },
                            count: None,
                        },
                    ],
                });

        let textures_bind_group =
            Self::create_textures_bind_group(&gpu, &textures_bind_group_layout, &tonemap.view);

        let uniforms = LcdUniforms::new(&gpu.device);

        let pipeline = Self::create_render_pipeline(
            &gpu.device,
            gpu.sc_desc.format,
            &textures_bind_group_layout,
            &uniforms.bind_group_layout,
        );

        Self {
            textures_bind_group_layout,
            textures_bind_group,
            pipeline,
            tonemap,
            uniforms,
        }
    }

    fn create_textures_bind_group(
        gpu: &gpu_state::GpuState,
        layout: &wgpu::BindGroupLayout,
        tonemap: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&gpu.color_attachment.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&tonemap),
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
        let vs_src = wgpu::include_spirv!("../shaders/lcd.vs.spv");
        let fs_src = wgpu::include_spirv!("../shaders/lcd.fs.spv");

        let vs_module = device.create_shader_module(&vs_src);
        let fs_module = device.create_shader_module(&fs_src);

        // no uniforms for LcdFilter shaders
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("LcdFilter Render Pipeline Layout"),
            bind_group_layouts: &[&textures_bind_group_layout, &uniforms_bind_group_layout],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("LcdFilter Render Pipeline"),
            layout: Some(&layout),

            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: "main",
                buffers: &[],
            },

            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: color_format,
                    alpha_blend: wgpu::BlendState::REPLACE,
                    color_blend: wgpu::BlendState::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),

            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: wgpu::CullMode::None,
                polygon_mode: wgpu::PolygonMode::Fill,
            },

            depth_stencil: None,

            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        })
    }

    pub fn resize(
        &mut self,
        _window: &Window,
        _new_size: winit::dpi::PhysicalSize<u32>,
        gpu: &gpu_state::GpuState,
    ) {
        self.textures_bind_group = Self::create_textures_bind_group(
            gpu,
            &self.textures_bind_group_layout,
            &self.tonemap.view,
        );
    }

    pub fn update(&mut self, _dt: std::time::Duration, ctx: &mut AppContext) {
        self.uniforms.data.set_palette_shift(1_f32);
        self.uniforms.write(&mut ctx.gpu.queue);
    }

    pub fn render(
        &mut self,
        _window: &Window,
        _gpu: &mut gpu_state::GpuState,
        frame: &wgpu::SwapChainFrame,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("LcdFilter Render Pass"),
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &frame.output.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.textures_bind_group, &[]);
        render_pass.set_bind_group(1, &self.uniforms.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
