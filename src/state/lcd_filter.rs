use std::rc::Rc;
use winit::window::Window;

use crate::{texture::Texture, Options};

use super::{app_state::AppContext, gpu_state};

pub struct LcdFilter {
    pipeline: wgpu::RenderPipeline,
}

impl LcdFilter {
    pub fn new(gpu: &mut gpu_state::GpuState, _options: &Options, _tonemap: Rc<Texture>) -> Self {
        Self {
            pipeline: LcdFilter::create_render_pipeline(&gpu.device, gpu.sc_desc.format),
        }
    }

    fn create_render_pipeline(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        let vs_src = wgpu::include_spirv!("../shaders/lcd.vs.spv");
        let fs_src = wgpu::include_spirv!("../shaders/lcd.fs.spv");

        let vs_module = device.create_shader_module(&vs_src);
        let fs_module = device.create_shader_module(&fs_src);

        // no uniforms for LcdFilter shaders
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("LcdFilter Render Pipeline Layout"),
            bind_group_layouts: &[],
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

    pub fn resize(&mut self, _window: &Window, _new_size: winit::dpi::PhysicalSize<u32>) {}

    pub fn update(&mut self, _dt: std::time::Duration, _ctx: &mut AppContext) {}

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
        render_pass.draw(0..4, 0..1);
    }
}
