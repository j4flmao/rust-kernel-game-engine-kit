//! Composition root: the ONLY place concrete subsystems and the platform
//! clock are instantiated and wired.
//!
//! Wire subsystems -> init kernel -> run frames -> shutdown. Exit codes:
//! 0 success, 1 configuration error, 2 runtime error.

use std::process::ExitCode;
use std::sync::Arc;

use rust_kernel_game_engine_kit::kernel::Kernel;
use rust_kernel_game_engine_kit::platform;
use rust_kernel_game_engine_kit::subsystems::hello::{Hello, HelloCounters};
use rust_kernel_game_engine_kit::subsystems::tock::{Tock, TockCounters};
use rust_kernel_game_engine_kit::subsystems::{
    audio::AudioSubsystem,
    input::InputSubsystem,
    physics::PhysicsSubsystem,
    renderer::RendererSubsystem,
    scripting::ScriptingSubsystem,
    ui::UiSubsystem,
    window::{WindowId, WindowSize, WindowSubsystem},
};

const FRAMES: u64 = 60 * 2;

#[cfg(target_os = "windows")]
fn renderer_bootstrap(
    size: WindowSize,
) -> (
    RendererSubsystem,
    Option<rust_kernel_game_engine_kit::platform::windows::window::Win32Window>,
) {
    use rust_kernel_game_engine_kit::platform::windows::{
        present::{PresentEngine, PresentHandles, PresentSize},
        window::Win32Window,
    };
    let enabled = std::env::var_os("RUST_KERNEL_NATIVE_VULKAN").is_some();
    if enabled {
        match Win32Window::create("Rust Kernel Engine", size.width as i32, size.height as i32) {
            Ok(window) => {
                window.show();
                let handles = PresentHandles::win32(window.instance_handle(), window.raw_handle());
                match PresentEngine::build(
                    handles,
                    PresentSize {
                        width: size.width,
                        height: size.height,
                    },
                    [0.03, 0.05, 0.08, 1.0],
                ) {
                    Ok(engine) => {
                        return (
                            RendererSubsystem::with_native(
                                engine,
                                PresentSize {
                                    width: size.width,
                                    height: size.height,
                                },
                                [0.03, 0.05, 0.08, 1.0],
                            ),
                            Some(window),
                        )
                    }
                    Err(error) => eprintln!(
                        "[kit] native Vulkan unavailable, using headless renderer: {error}"
                    ),
                }
            }
            Err(error) => {
                eprintln!("[kit] native Win32 window unavailable, using headless renderer: {error}")
            }
        }
    }
    (RendererSubsystem::new(), None)
}

#[cfg(not(target_os = "windows"))]
fn renderer_bootstrap(size: WindowSize) -> (RendererSubsystem, Option<()>) {
    let _ = size;
    (RendererSubsystem::new(), None)
}

fn main() -> ExitCode {
    // Platform clock lives behind the PAL trait; nothing else sees the OS.
    let clock = platform::system_clock();
    eprintln!("[kit] monotonic clock: {} ns since epoch", clock.now_ns());

    // Shared counters survive the subsystem objects being moved into the
    // kernel, so the root can report results after shutdown.
    let hello_counters = Arc::new(HelloCounters::default());
    let tock_counters = Arc::new(TockCounters::default());

    let mut kernel = Kernel::new();
    if let Err(err) = kernel.register(Box::new(Hello::new(hello_counters.clone()))) {
        eprintln!("[kit] registration error: {err}");
        return ExitCode::from(1);
    }
    if let Err(err) = kernel.register(Box::new(Tock::new(tock_counters.clone()))) {
        eprintln!("[kit] registration error: {err}");
        return ExitCode::from(1);
    }
    let size = WindowSize::new(1280, 720).expect("static window size is valid");
    let (renderer, _native_window_guard) = renderer_bootstrap(size);
    for subsystem in [
        Box::new(WindowSubsystem::new(WindowId(1), size))
            as Box<dyn rust_kernel_game_engine_kit::kernel::Subsystem>,
        Box::new(InputSubsystem::default()),
        Box::new(UiSubsystem::default()),
        Box::new(renderer),
        Box::new(PhysicsSubsystem::default()),
        Box::new(AudioSubsystem::default()),
        Box::new(ScriptingSubsystem::default()),
    ] {
        if let Err(err) = kernel.register(subsystem) {
            eprintln!("[kit] registration error: {err}");
            return ExitCode::from(1);
        }
    }

    if let Err(err) = kernel.init() {
        eprintln!("[kit] init error: {err}");
        return ExitCode::from(1);
    }

    if let Err(err) = kernel.run(FRAMES) {
        eprintln!("[kit] run error: {err}");
        return ExitCode::from(2);
    }

    kernel.shutdown();

    let (ticks, pulses, echos) = hello_counters.summary();
    eprintln!(
        "[kit] ran {FRAMES} frames: hello ticks={ticks}, pulses sent={pulses}, echos={echos}"
    );
    let (pulses, last_seq) = tock_counters.summary();
    eprintln!("[kit] tock received {pulses} pulses, last seq={last_seq}");

    ExitCode::SUCCESS
}
