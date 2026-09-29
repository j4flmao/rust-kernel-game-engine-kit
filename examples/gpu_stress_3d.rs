//! Interactive Minecraft-style voxel renderer.
//!
//! This is intentionally independent from the Rubik example. It builds a
//! height-mapped block world, uploads voxel instances, and presents it through
//! the native Windows Vulkan path.

#![allow(dead_code)]

#[path = "gpu_stress_3d/voxel_camera.rs"]
mod camera;
#[path = "gpu_stress_3d/voxel_driver.rs"]
mod driver;
#[path = "gpu_stress_3d/voxel_scene.rs"]
mod scene;
#[path = "gpu_stress_3d/voxel_state.rs"]
mod state;
#[path = "gpu_stress_3d/ui.rs"]
mod ui;

use driver::VoxelDriver;
use scene::VoxelScene;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 800;

fn main() {
    report_shader_bundle();
    let driver = VoxelDriver::new();
    let scene = VoxelScene::new();
    #[cfg(windows)]
    run_native(driver, scene);
    #[cfg(not(windows))]
    println!(
        "gpu_stress_3d: Linux validates voxel extraction; native window path is enabled on Windows"
    );
}

fn report_shader_bundle() {
    let shaders = [
        (
            "ui.vert",
            rust_kernel_game_engine_kit::platform::ui_shaders::UI_VERTEX_SPIRV,
        ),
        (
            "ui.frag",
            rust_kernel_game_engine_kit::platform::ui_shaders::UI_FRAGMENT_SPIRV,
        ),
        (
            "world.vert",
            rust_kernel_game_engine_kit::platform::ui_shaders::WORLD_VERTEX_SPIRV,
        ),
        (
            "world.frag",
            rust_kernel_game_engine_kit::platform::ui_shaders::WORLD_FRAGMENT_SPIRV,
        ),
        (
            "frustum_cull.comp",
            rust_kernel_game_engine_kit::platform::ui_shaders::FRUSTUM_CULL_SPIRV,
        ),
        (
            "build_indirect.comp",
            rust_kernel_game_engine_kit::platform::ui_shaders::BUILD_INDIRECT_SPIRV,
        ),
    ];
    let missing = shaders
        .iter()
        .filter(|(_, words)| words.is_empty())
        .map(|(name, _)| *name)
        .collect::<Vec<_>>();
    println!(
        "gpu_stress_3d: shaders={} words={} missing={:?}",
        shaders.len(),
        shaders.iter().map(|(_, words)| words.len()).sum::<usize>(),
        missing
    );
}

#[cfg(windows)]
fn run_native(mut driver: VoxelDriver, scene: VoxelScene) {
    use rust_kernel_game_engine_kit::platform::windows::{
        present::{PresentEngine, PresentHandles, PresentSize, PresentStatus},
        window::Win32Window,
    };
    use rust_kernel_game_engine_kit::subsystems::input::InputEvent;
    use rust_kernel_game_engine_kit::subsystems::window::{WindowEvent, WindowSize};
    use std::time::{Duration, Instant};

    let window = match Win32Window::create(ui::WINDOW_TITLE, WIDTH as i32, HEIGHT as i32) {
        Ok(window) => window,
        Err(error) => {
            eprintln!("gpu_stress_3d: window creation failed: {error}");
            return;
        }
    };
    window.show();
    let size = window.client_size().unwrap_or(WindowSize {
        width: WIDTH,
        height: HEIGHT,
    });
    let handles = PresentHandles::win32(window.instance_handle(), window.raw_handle());
    let mut present = match PresentEngine::build(
        handles,
        PresentSize {
            width: size.width,
            height: size.height,
        },
        [0.04, 0.07, 0.11, 1.0],
    ) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("gpu_stress_3d: Vulkan setup failed: {error}");
            return;
        }
    };
    let mut camera = camera::OrbitCamera::default();
    let mut viewport_width = present.extent().width.max(1);
    let mut viewport_height = present.extent().height.max(1);
    camera.fit_to_viewport(viewport_width, viewport_height);
    let max_frames = std::env::var("RKE_GPU_STRESS_MAX_FRAMES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(600_u64);
    let mut frame_count = 0_u64;
    let mut events = Vec::with_capacity(16);
    let mut inputs = Vec::with_capacity(32);
    let mut drag = None::<(i32, i32)>;
    let mut cursor = (0_i32, 0_i32);
    let started = Instant::now();
    let mut presented = 0_u64;

    while frame_count < max_frames {
        window.poll_events(&mut events, &mut inputs);
        let mut running = true;
        for event in &events {
            match event {
                WindowEvent::CloseRequested => running = false,
                WindowEvent::Resized(WindowSize { width, height }) => {
                    let target = window.client_size().unwrap_or(WindowSize {
                        width: (*width).max(1),
                        height: (*height).max(1),
                    });
                    if let Err(error) = present.recreate(PresentSize {
                        width: target.width,
                        height: target.height,
                    }) {
                        eprintln!("gpu_stress_3d: resize failed: {error}");
                        running = false;
                    }
                    viewport_width = present.extent().width.max(1);
                    viewport_height = present.extent().height.max(1);
                    camera.fit_to_viewport(viewport_width, viewport_height);
                }
                WindowEvent::Focused(_) => {}
            }
        }
        for input in &inputs {
            match *input {
                InputEvent::Key {
                    key: 0x1b,
                    pressed: true,
                } => running = false,
                InputEvent::Key {
                    key: 0x48,
                    pressed: true,
                } => driver.world.regenerate(),
                InputEvent::Key {
                    key: 0x4e,
                    pressed: true,
                } => {
                    driver.world.player.position = [0.0, 12.0, 8.0];
                    driver.world.player.velocity = [0.0; 3];
                    camera = camera::OrbitCamera::default();
                }
                InputEvent::Key { key: 0x57, pressed } => {
                    driver.forward = if pressed { -1.0 } else { 0.0 }
                }
                InputEvent::Key { key: 0x53, pressed } => {
                    driver.forward = if pressed { 1.0 } else { 0.0 }
                }
                InputEvent::Key { key: 0x41, pressed } => {
                    driver.strafe = if pressed { -1.0 } else { 0.0 }
                }
                InputEvent::Key { key: 0x44, pressed } => {
                    driver.strafe = if pressed { 1.0 } else { 0.0 }
                }
                InputEvent::Key {
                    key: 0x20,
                    pressed: true,
                } => driver.jump(),
                InputEvent::MouseMoved { x, y } => {
                    cursor = (x, y);
                    if let Some((last_x, last_y)) = drag.as_mut() {
                        camera.orbit(x - *last_x, y - *last_y);
                        *last_x = x;
                        *last_y = y;
                    }
                }
                InputEvent::MouseButton {
                    button: 0,
                    pressed: true,
                } => drag = Some(cursor),
                InputEvent::MouseButton {
                    button: 0,
                    pressed: false,
                } => drag = None,
                _ => {}
            }
        }
        driver.tick(1.0 / 60.0);
        let view = camera.view_at(
            viewport_width as f32 / viewport_height as f32,
            viewport_width,
            viewport_height,
            driver.world.player.position,
        );
        let frame = match scene.build_frame(&driver.world) {
            Ok(frame) => frame,
            Err(error) => {
                eprintln!("gpu_stress_3d: voxel upload rejected: {error}");
                break;
            }
        };
        if let Err(error) = present.set_world_upload(
            frame.upload.instance_bytes(),
            view.view_projection,
            &frame.draws,
        ) {
            eprintln!("gpu_stress_3d: frame allocation failed: {error}");
            break;
        }
        match present.present_frame() {
            Ok(PresentStatus::Presented) => presented += 1,
            Ok(PresentStatus::OutOfDate) => {}
            Err(error) => {
                eprintln!("gpu_stress_3d: present failed: {error}");
                break;
            }
        }
        window.paint_voxel_help(viewport_width as i32, viewport_height as i32);
        if !running {
            break;
        }
        frame_count += 1;
        std::thread::sleep(Duration::from_millis(16));
    }
    let elapsed = started.elapsed().as_secs_f64();
    println!(
        "gpu_stress_3d: frames={presented} average_fps={:.2}",
        presented as f64 / elapsed.max(0.001)
    );
}
