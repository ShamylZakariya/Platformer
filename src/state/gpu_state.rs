use crate::texture;
use winit::window::Window;

pub struct GpuState {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub depth_attachment: texture::Texture,
    pub color_attachment: texture::Texture,
}

impl GpuState {
    pub const COLOR_ATTACHMENT_LAYER_COUNT: u32 = 64;

    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
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

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: *surface
                .get_supported_formats(&adapter)
                .first()
                .expect("Unable to find a surface compatible with the adapter"),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
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
        }
    }

    pub fn resize(&mut self, _window: &Window, new_size: winit::dpi::PhysicalSize<u32>) {
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
}
