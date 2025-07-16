use std::sync::Arc;

use crate::texture;
use winit::window::Window;

pub struct GpuState {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub depth_attachment: texture::Texture,
    pub color_attachment: texture::Texture,
    pub window: Arc<winit::window::Window>,
}

impl GpuState {
    pub const COLOR_ATTACHMENT_LAYER_COUNT: u32 = 64;

    pub async fn new(window: Window) -> GpuState {
        let window = Arc::new(window);
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("Expected wgpu instance to create surface.");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let color_attachment = texture::Texture::create_color_texture_array(
            &device,
            config.width,
            config.height,
            Self::COLOR_ATTACHMENT_LAYER_COUNT,
            config.format,
            "Color Attachment Array",
        );
        let depth_attachment =
            texture::Texture::create_depth_texture(&device, &config, "Depth Attachment");

        Self {
            surface,
            device,
            queue,
            config,
            depth_attachment,
            color_attachment,
            window,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width.max(1);
            self.config.height = new_size.height.max(1);
            self.depth_attachment = texture::Texture::create_depth_texture(
                &self.device,
                &self.config,
                "Depth Attachment",
            );
            self.color_attachment = texture::Texture::create_color_texture_array(
                &self.device,
                self.config.width,
                self.config.height,
                Self::COLOR_ATTACHMENT_LAYER_COUNT,
                self.config.format,
                "Color Attachment Array",
            );
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn encoder(&self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            })
    }

    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        winit::dpi::PhysicalSize::new(self.config.width, self.config.height)
    }

    pub fn window(&self) -> &winit::window::Window {
        &self.window
    }
}
