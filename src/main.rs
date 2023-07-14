mod scene;

use anyhow::{anyhow, Context};
use bytes::Buf;
use glam::{Vec2, uvec2, vec2};
use inox2d::formats::inp::parse_inp;
use inox2d::{model::Model, render::wgpu::Renderer};
use log::{debug, info};
use wgpu::CompositeAlphaMode;
use winit::event::{KeyboardInput, Event, WindowEvent, VirtualKeyCode, ElementState};
use winit::event_loop::ControlFlow;
use winit::platform::web::WindowExtWebSys;
use winit::window::Window;
use winit::{event_loop::EventLoop, window::WindowBuilder};

use crate::scene::ExampleSceneController;

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Info));
    console_error_panic_hook::set_once();

    wasm_bindgen_futures::spawn_local(runwrap());
}

async fn runwrap() {
    match run().await {
        Ok(_) => info!("app shutdown"),
        Err(e) => log::error!("error: {}", e),
    }
}

async fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::new();
    let window = try_create_window(&event_loop)?;

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let surface = unsafe { instance.create_surface(&window) }?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .ok_or(anyhow!("no wgpu adapter found"))?;

    info!("wgpu adapter: {:?}", adapter.get_info());

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER,
                limits: wgpu::Limits::default(),
                label: None,
            },
            None,
        )
        .await?;

    info!("device features: {:?}", device.features());

    // Fallback to first alpha mode if PreMultiplied is not supported
    let alpha_modes = surface.get_capabilities(&adapter).alpha_modes;
    let alpha_mode = if alpha_modes.contains(&CompositeAlphaMode::PreMultiplied) {
        CompositeAlphaMode::PreMultiplied
    } else {
        alpha_modes[0]
    };

    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8Unorm,
        width: window.inner_size().width,
        height: window.inner_size().height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode,
        view_formats: Vec::new(),
    };
    surface.configure(&device, &config);

    info!("wgpu surface initialized");

    info!("loading puppet");
    let res = reqwest::Client::new()
        .get(format!("{}/assets/puppet.inp", base_url()))
        .send()
        .await?;

    let model = inox2d::formats::inp::parse_inp(res.bytes().await?.reader())?;
    info!("== Puppet Meta ==\n{}", &model.puppet.meta);
    debug!("== Nodes ==\n{}", &model.puppet.nodes);
    if model.vendors.is_empty() {
        info!("(No Vendor Data)\n");
    } else {
        info!("== Vendor Data ==");
        for vendor in &model.vendors {
            debug!("{vendor}");
        }
    }

    let mut renderer = Renderer::new(
        &device,
        &queue,
        wgpu::TextureFormat::Bgra8Unorm,
        &model,
        uvec2(window.inner_size().width, window.inner_size().height),
    );
    renderer.camera.scale = Vec2::splat(0.15);
    let mut scene_ctrl = ExampleSceneController::new(&renderer.camera, 0.5);
    let mut puppet = model.puppet;

    event_loop.run(move |event, _, control_flow| match event {
        Event::RedrawRequested(_) => {
            scene_ctrl.update(&mut renderer.camera);

            puppet.begin_set_params();
            let t = scene_ctrl.current_elapsed();
            //puppet.set_param("Head:: Yaw-Pitch", vec2(t.cos(), t.sin()));
            puppet.end_set_params();

            let output = surface.get_current_texture().unwrap();
            let view = (output.texture).create_view(&wgpu::TextureViewDescriptor::default());

            renderer.render(&queue, &device, &puppet, &view);
            output.present();
        }
        Event::WindowEvent { ref event, .. } => match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    },
                ..
            } => *control_flow = ControlFlow::Exit,
            WindowEvent::Resized(size) => {
                // Reconfigure the surface with the new size
                config.width = size.width;
                config.height = size.height;
                surface.configure(&device, &config);

                // Update the renderer's internal viewport
                renderer.resize(uvec2(size.width, size.height));

                // On macos the window needs to be redrawn manually after resizing
                window.request_redraw();
            }
            _ => scene_ctrl.interact(&window, event, &renderer.camera),
        },
        Event::MainEventsCleared => {
            // RedrawRequested will only trigger once, unless we manually
            // request it.
            window.request_redraw();
        }
        _ => {}
    });
    Ok(())
}

fn try_create_window(event: &EventLoop<()>) -> anyhow::Result<Window> {
    let window = WindowBuilder::new()
        .with_resizable(false)
        .with_inner_size(winit::dpi::PhysicalSize::<u32>::new(1280, 720))
        .build(event)?;

    web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| doc.body())
        .and_then(|body| {
            body.append_child(&web_sys::Element::from(window.canvas()))
                .ok()
        })
        .context("couldn't append canvas to document body")?;

    return Ok(window);
}

pub fn base_url() -> String {
    web_sys::window().unwrap().location().origin().unwrap()
}
