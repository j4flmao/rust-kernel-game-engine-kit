//! Interactive Minecraft-style voxel renderer.
//!
//! This is intentionally independent from the Rubik example. It builds a
//! height-mapped block world, uploads voxel instances, and presents it through
//! the native Windows and Linux Vulkan paths.

#![allow(dead_code)]

#[path = "gpu_stress_3d/voxel_camera.rs"]
mod camera;
#[path = "gpu_stress_3d/voxel_driver.rs"]
mod driver;
#[path = "gpu_stress_3d/voxel_scene.rs"]
mod scene;
#[path = "gpu_stress_3d/voxel_state.rs"]
mod state;
#[path = "gpu_stress_3d/voxel_stream.rs"]
mod stream;
#[path = "gpu_stress_3d/ui.rs"]
mod ui;

use driver::VoxelDriver;
use stream::VoxelScene;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 800;

fn main() {
    report_shader_bundle();
    #[cfg(any(windows, target_os = "linux"))]
    run_native(VoxelDriver::new(), VoxelScene::new());
    #[cfg(not(any(windows, target_os = "linux")))]
    println!(
        "gpu_stress_3d: shader inventory only on this platform; native window path is enabled on Windows"
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

#[cfg(any(windows, target_os = "linux"))]
fn run_native(mut driver: VoxelDriver, mut scene: VoxelScene) {
    #[cfg(target_os = "linux")]
    use rust_kernel_game_engine_kit::platform::linux::{
        present::{PresentEngine, PresentHandles, PresentSize, PresentStatus},
        x11::X11Library,
    };
    #[cfg(windows)]
    use rust_kernel_game_engine_kit::platform::windows::{
        present::{PresentEngine, PresentHandles, PresentSize, PresentStatus},
        window::Win32Window,
    };
    use rust_kernel_game_engine_kit::subsystems::input::InputEvent;
    use rust_kernel_game_engine_kit::subsystems::window::{WindowEvent, WindowSize};
    use std::time::{Duration, Instant};

    #[cfg(windows)]
    let window = match Win32Window::create(ui::WINDOW_TITLE, WIDTH as i32, HEIGHT as i32) {
        Ok(window) => window,
        Err(error) => {
            eprintln!("gpu_stress_3d: window creation failed: {error}");
            return;
        }
    };
    #[cfg(target_os = "linux")]
    let library = match X11Library::load() {
        Ok(value) => value,
        Err(error) => {
            eprintln!("gpu_stress_3d: {error}");
            return;
        }
    };
    #[cfg(target_os = "linux")]
    let mut window = match library.open_display() {
        Ok(value) => value,
        Err(error) => {
            eprintln!("gpu_stress_3d: {error}");
            return;
        }
    };
    #[cfg(windows)]
    window.show();
    let size = window.client_size().unwrap_or(WindowSize {
        width: WIDTH,
        height: HEIGHT,
    });
    #[cfg(windows)]
    let handles = PresentHandles::win32(window.instance_handle(), window.raw_handle());
    #[cfg(target_os = "linux")]
    let handles = PresentHandles::x11(window.display_handle(), window.window());
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
    let mut last_tick = Instant::now();
    let mut movement = [false; 4];
    let mut uploaded_world = None;
    let target_fps = std::env::var("RKE_GPU_STRESS_TARGET_FPS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(60)
        .min(1000);
    let frame_budget =
        (target_fps != 0).then(|| Duration::from_secs_f64(1.0 / f64::from(target_fps)));
    let mut stats_started = Instant::now();
    let mut stats_presented = 0_u64;
    println!("gpu_stress_3d: target_fps={target_fps} (0 disables CPU pacing; presentation may still synchronize)");
    for line in ui::HELP_LINES {
        println!("{line}");
    }

    while frame_count < max_frames {
        let frame_started = Instant::now();
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
                WindowEvent::Focused(false) => {
                    movement = [false; 4];
                    driver.forward = 0.0;
                    driver.strafe = 0.0;
                    drag = None;
                }
                WindowEvent::Focused(true) => {}
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
                    movement[0] = pressed;
                }
                InputEvent::Key { key: 0x53, pressed } => {
                    movement[1] = pressed;
                }
                InputEvent::Key { key: 0x41, pressed } => {
                    movement[2] = pressed;
                }
                InputEvent::Key { key: 0x44, pressed } => {
                    movement[3] = pressed;
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
        if !running {
            break;
        }
        driver.forward = i32::from(movement[1]) as f32 - i32::from(movement[0]) as f32;
        driver.strafe = i32::from(movement[3]) as f32 - i32::from(movement[2]) as f32;
        let now = Instant::now();
        driver.tick((now - last_tick).as_secs_f32().min(0.05));
        last_tick = now;
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
        if frame_count.is_multiple_of(120) {
            println!(
                "voxel: blocks={} exposed_faces={} greedy_quads={} draws={} upload_bytes={} chunks={} pending={}",
                frame.block_count,
                frame.exposed_faces,
                frame.upload.instance_bytes().len() / 80,
                frame.batch_count,
                frame.upload.instance_bytes().len(),
                frame.resident_chunks, frame.pending_chunks
            );
        }
        let revision = frame.revision;
        let update = if uploaded_world == Some(revision) {
            present.set_world_camera(view.view_projection)
        } else {
            present.set_world_upload(
                frame.upload.instance_bytes(),
                view.view_projection,
                &frame.draws,
            )
        };
        if let Err(error) = update {
            eprintln!("gpu_stress_3d: frame allocation failed: {error}");
            break;
        }
        uploaded_world = Some(revision);
        match present.present_frame() {
            Ok(PresentStatus::Presented) => presented += 1,
            Ok(PresentStatus::OutOfDate) => {
                if let Err(error) = present.recreate(PresentSize {
                    width: viewport_width,
                    height: viewport_height,
                }) {
                    eprintln!("gpu_stress_3d: swapchain recovery failed: {error}");
                    break;
                }
            }
            Err(error) => {
                eprintln!("gpu_stress_3d: present failed: {error}");
                break;
            }
        }
        #[cfg(windows)]
        window.paint_voxel_help(viewport_width as i32, viewport_height as i32);
        if stats_started.elapsed() >= Duration::from_millis(500) {
            let seconds = stats_started.elapsed().as_secs_f64();
            let fps = (presented - stats_presented) as f64 / seconds;
            let uploads = present.world_upload_stats();
            window.set_title(&format!(
                "Voxel | {fps:.1} FPS | {} blocks | {} quads | {} draws | {} uploads / {} KiB | chunks {} pending {}",
                frame.block_count,
                frame.upload.instance_bytes().len() / 80,
                frame.batch_count,
                uploads.transfers,
                uploads.bytes / 1024, frame.resident_chunks, frame.pending_chunks
            ));
            stats_started = Instant::now();
            stats_presented = presented;
        }
        if !running {
            break;
        }
        frame_count += 1;
        if let Some(remaining) =
            frame_budget.and_then(|budget| budget.checked_sub(frame_started.elapsed()))
        {
            std::thread::sleep(remaining);
        }
    }
    let elapsed = started.elapsed().as_secs_f64();
    let uploads = present.world_upload_stats();
    println!(
        "gpu_stress_3d: geometry_transfers={} transferred_bytes={}",
        uploads.transfers, uploads.bytes
    );
    println!(
        "gpu_stress_3d: frames={presented} average_fps={:.2}",
        presented as f64 / elapsed.max(0.001)
    );
}
