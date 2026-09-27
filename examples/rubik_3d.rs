//! Native GPU 3D bring-up example for the renderer contract.
//!
//! Windows with a working Vulkan loader opens a real native window and submits
//! 27 instanced cubies through the kernel's bounded render-world extraction and
//! GPU instance upload path.  The first pass uses a direct instanced draw and
//! a shader-generated cube mesh; GPU culling and indirect command generation
//! are deliberately kept as the next renderer phase.
//!
//! ```text
//! cargo run --example rubik_3d --release
//! ```

// The Linux/MSRV build validates the headless frame contract while the native
// Windows path exercises the camera and picking helpers at runtime.
#![allow(dead_code)]

#[path = "rubik_3d/camera.rs"]
mod camera;
#[path = "rubik_3d/driver.rs"]
mod driver;
#[path = "rubik_3d/scene.rs"]
mod scene;
#[path = "rubik_3d/state.rs"]
mod state;

use driver::RubikDriver;
use scene::RubikScene;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 800;
const DEFAULT_MAX_FRAMES: u64 = 600;

fn main() {
    let driver = RubikDriver::new();
    let scene = RubikScene::new();
    let frame = scene
        .build_frame_for(driver.state(), driver.active_rotation())
        .expect("Rubik scene must satisfy bounded 3D contracts");
    println!(
        "rubik_3d: {} cubies, {} CPU batch(es), {} instance bytes",
        frame.instance_count,
        frame.batch_count,
        frame.upload.instance_bytes().len()
    );

    #[cfg(windows)]
    run_native(driver, scene);

    #[cfg(not(windows))]
    {
        println!("rubik_3d: native presentation is currently enabled on Windows; headless Linux validates the frame contract only");
    }
}

#[cfg(windows)]
fn run_native(mut driver: RubikDriver, mut scene: RubikScene) {
    use driver::{MoveFace, MoveRequest};
    use rust_kernel_game_engine_kit::platform::windows::{
        present::{PresentEngine, PresentHandles, PresentSize, PresentStatus},
        window::Win32Window,
    };
    use rust_kernel_game_engine_kit::subsystems::input::InputEvent;
    use rust_kernel_game_engine_kit::subsystems::window::{WindowEvent, WindowSize};
    use state::{Axis, RotationCommand};
    use std::time::Duration;

    let window = match Win32Window::create("Rust Kernel Rubik 3D", WIDTH as i32, HEIGHT as i32) {
        Ok(window) => window,
        Err(error) => {
            eprintln!("rubik_3d: window creation failed: {error}");
            return;
        }
    };
    window.show();
    let initial_size = window.client_size().unwrap_or(WindowSize {
        width: WIDTH,
        height: HEIGHT,
    });
    let handles = PresentHandles::win32(window.instance_handle(), window.raw_handle());
    let mut present = match PresentEngine::build(
        handles,
        PresentSize {
            width: initial_size.width,
            height: initial_size.height,
        },
        [0.025, 0.035, 0.065, 1.0],
    ) {
        Ok(present) => present,
        Err(error) => {
            eprintln!("rubik_3d: Vulkan present setup failed: {error}");
            eprintln!("rubik_3d: check Vulkan loader/driver availability");
            return;
        }
    };

    let mut window_events = Vec::with_capacity(8);
    let mut input_events = Vec::with_capacity(16);
    let mut camera = camera::OrbitCamera::default();
    let mut viewport_width = present.extent().width.max(1);
    let mut viewport_height = present.extent().height.max(1);
    camera.fit_to_viewport(viewport_width, viewport_height);
    let mut cursor = (0_i32, 0_i32);
    let mut drag = DragMode::None;
    let mut running = true;
    let max_frames = std::env::var("RKE_RUBIK_MAX_FRAMES")
        .ok()
        .and_then(|value| value.parse::<u64>().ok());
    let max_frames = max_frames.unwrap_or(DEFAULT_MAX_FRAMES);
    let mut frame_count = 0_u64;
    while running && frame_count < max_frames {
        window.poll_events(&mut window_events, &mut input_events);
        for event in &window_events {
            match event {
                WindowEvent::CloseRequested => running = false,
                WindowEvent::Resized(WindowSize { width, height }) => {
                    let size = window.client_size().unwrap_or(WindowSize {
                        width: (*width).max(1),
                        height: (*height).max(1),
                    });
                    if let Err(error) = present.recreate(PresentSize {
                        width: size.width,
                        height: size.height,
                    }) {
                        eprintln!("rubik_3d: swapchain recreation failed: {error}");
                        running = false;
                    } else {
                        // Surface capabilities may clamp the request.  The
                        // render viewport must follow the actual swapchain
                        // extent or maximize leaves an old frame in a corner.
                        viewport_width = present.extent().width.max(1);
                        viewport_height = present.extent().height.max(1);
                        camera.fit_to_viewport(viewport_width, viewport_height);
                    }
                }
                WindowEvent::Focused(_) => {}
            }
        }

        for event in &input_events {
            match *event {
                InputEvent::Key {
                    key: 0x1b,
                    pressed: true,
                } => running = false,
                InputEvent::Key {
                    key: 0x30,
                    pressed: true,
                } => camera = camera::OrbitCamera::default(),
                InputEvent::Key { key, pressed: true } => {
                    if key == 0x48 {
                        driver.shuffle(24);
                        continue;
                    }
                    if key == 0x5a {
                        let _ = driver.undo();
                        continue;
                    }
                    if key == 0x59 {
                        let _ = driver.redo();
                        continue;
                    }
                    if key == 0x54 {
                        driver.solve_history();
                        continue;
                    }
                    if key == 0x4e {
                        driver.reset();
                        continue;
                    }
                    let request = match key {
                        0x55 => Some(MoveRequest::normal(MoveFace::U)),
                        0x44 => Some(MoveRequest::normal(MoveFace::D)),
                        0x4c => Some(MoveRequest::normal(MoveFace::L)),
                        0x52 => Some(MoveRequest::normal(MoveFace::R)),
                        0x46 => Some(MoveRequest::normal(MoveFace::F)),
                        0x42 => Some(MoveRequest::normal(MoveFace::B)),
                        0x4d => Some(MoveRequest::normal(MoveFace::M)),
                        0x45 => Some(MoveRequest::normal(MoveFace::E)),
                        0x53 => Some(MoveRequest::normal(MoveFace::S)),
                        _ => None,
                    };
                    if let Some(request) = request {
                        let _ = driver.enqueue(request);
                    }
                }
                InputEvent::MouseMoved { x, y } => {
                    cursor = (x, y);
                    match &mut drag {
                        DragMode::Orbit { last_x, last_y } => {
                            camera.orbit(x - *last_x, y - *last_y);
                            *last_x = x;
                            *last_y = y;
                        }
                        DragMode::Layer {
                            coordinate,
                            normal,
                            start_x,
                            start_y,
                            applied,
                        } if !*applied
                            && (x - *start_x)
                                .unsigned_abs()
                                .max((y - *start_y).unsigned_abs())
                                >= 10 =>
                        {
                            let delta_x = x - *start_x;
                            let delta_y = y - *start_y;
                            let axis = normal
                                .iter()
                                .position(|component| *component != 0)
                                .unwrap_or(2);
                            let direction = camera.layer_drag_direction(*normal, delta_x, delta_y);
                            let axis = match axis {
                                0 => Axis::X,
                                1 => Axis::Y,
                                _ => Axis::Z,
                            };
                            if let Some(command) = RotationCommand::new(
                                axis,
                                coordinate[axis.index()] as i8,
                                direction as i8,
                            ) {
                                if driver.enqueue_command(command) {
                                    *applied = true;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                InputEvent::MouseButton {
                    button: 0,
                    pressed: true,
                } => {
                    let view = camera.view(
                        viewport_width as f32 / viewport_height.max(1) as f32,
                        viewport_width,
                        viewport_height,
                    );
                    scene.sync_state(driver.state());
                    if let Some(hit) = scene.pick_face(
                        cursor.0,
                        cursor.1,
                        view.view_projection,
                        viewport_width,
                        viewport_height,
                    ) {
                        drag = DragMode::Layer {
                            coordinate: hit.coordinate,
                            normal: hit.normal,
                            start_x: cursor.0,
                            start_y: cursor.1,
                            applied: false,
                        };
                    } else {
                        drag = DragMode::Orbit {
                            last_x: cursor.0,
                            last_y: cursor.1,
                        };
                    }
                }
                InputEvent::MouseButton {
                    button: 0,
                    pressed: false,
                } => drag = DragMode::None,
                InputEvent::MouseButton {
                    button: 1,
                    pressed: true,
                } => {
                    // Right-drag always orbits the whole cube, even when the
                    // pointer starts on a sticker. Left-drag remains the
                    // physical layer-turn gesture.
                    drag = DragMode::Orbit {
                        last_x: cursor.0,
                        last_y: cursor.1,
                    };
                }
                InputEvent::MouseButton {
                    button: 1,
                    pressed: false,
                } => drag = DragMode::None,
                _ => {}
            }
        }

        let view = camera.view(
            viewport_width as f32 / viewport_height.max(1) as f32,
            viewport_width,
            viewport_height,
        );
        let pass = rust_kernel_game_engine_kit::subsystems::renderer_3d::WorldPassConfig {
            view,
            clear_color: [0.025, 0.035, 0.065, 1.0],
            clear_depth: 1.0,
        };
        if let Err(error) = pass.validate() {
            eprintln!("rubik_3d: invalid world pass: {error}");
            break;
        }
        driver.advance(16_666_667);
        scene.sync_state(driver.state());
        let frame = match scene.build_frame_for(driver.state(), driver.active_rotation()) {
            Ok(next_frame) => next_frame,
            Err(error) => {
                eprintln!("rubik_3d: layer rebuild failed: {error}");
                break;
            }
        };
        if let Err(error) = present.set_world_upload(
            frame.upload.instance_bytes(),
            view.view_projection,
            &frame.draws,
        ) {
            eprintln!("rubik_3d: world upload rejected: {error}");
            break;
        }
        match present.present_frame() {
            Ok(PresentStatus::Presented) => {}
            Ok(PresentStatus::OutOfDate) => {
                if let Some(size) = window.client_size() {
                    if let Err(error) = present.recreate(PresentSize {
                        width: size.width,
                        height: size.height,
                    }) {
                        eprintln!("rubik_3d: out-of-date swapchain recreation failed: {error}");
                        running = false;
                    } else {
                        viewport_width = present.extent().width.max(1);
                        viewport_height = present.extent().height.max(1);
                        camera.fit_to_viewport(viewport_width, viewport_height);
                    }
                }
            }
            Err(error) => {
                eprintln!("rubik_3d: frame submission failed: {error}");
                running = false;
            }
        }
        window.paint_rubik_help(viewport_width as i32, viewport_height as i32);
        std::thread::sleep(Duration::from_millis(16));
        frame_count = frame_count.saturating_add(1);
    }
}

#[cfg(windows)]
enum DragMode {
    None,
    Orbit {
        last_x: i32,
        last_y: i32,
    },
    Layer {
        coordinate: [i32; 3],
        normal: [i32; 3],
        start_x: i32,
        start_y: i32,
        applied: bool,
    },
}
