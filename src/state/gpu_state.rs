use crate::texture;
use winit::window::Window;

pub struct GpuState {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub sc_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub depth_attachment: texture::Texture,
    pub color_attachment: texture::Texture,
}

impl GpuState {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let depth_texture =
            texture::Texture::create_depth_texture(&device, &sc_desc, "depth_attachment");

        let color_texture =
            texture::Texture::create_color_texture(&device, &sc_desc, "color_attachment");

        Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            size,
            depth_attachment: depth_texture,
            color_attachment: color_texture,
        }
    }

    pub fn resize(&mut self, _window: &Window, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width.max(1);
        self.sc_desc.height = new_size.height.max(1);
        self.depth_attachment =
            texture::Texture::create_depth_texture(&self.device, &self.sc_desc, "depth_texture");
        self.color_attachment =
            texture::Texture::create_color_texture(&self.device, &self.sc_desc, "color_attachment");

        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    pub fn encoder(&self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            })
    }
}
