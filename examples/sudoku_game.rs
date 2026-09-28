//! A small native UI game example: a color-coded Sudoku board.
//!
//! On Windows, the example uses a safe Win32 fallback UI by default. The
//! handwritten Vulkan path is experimental and requires
//! `RKE_ENABLE_NATIVE_VULKAN=experimental`.
//!
//! ```text
//! cargo run --example sudoku_game --release
//! ```
//!
//! The board is intentionally built from the kernel UI contracts only. Each
//! row/cell is a retained node, layout and paint are resolved by the UI
//! subsystem, and the renderer receives the same backend-neutral packet used
//! by headless tests. Light cells are givens, dark cells are editable slots,
//! and the accent color marks the selected 3x3 region.

use rust_kernel_game_engine_kit::kernel::{
    trace::FrameTracer, GamePlugin, GamePluginHost, Kernel, KernelContext, Subsystem,
};
use rust_kernel_game_engine_kit::platform;
#[cfg(windows)]
use rust_kernel_game_engine_kit::subsystems::input::InputEvent;
use rust_kernel_game_engine_kit::subsystems::input::InputSubsystem;
#[cfg(windows)]
use rust_kernel_game_engine_kit::subsystems::messages::{InputEventMessage, WindowResized};
use rust_kernel_game_engine_kit::subsystems::messages::{InputFrame, UiCommandMessage};
use rust_kernel_game_engine_kit::subsystems::renderer::RendererSubsystem;
use rust_kernel_game_engine_kit::subsystems::ui::{
    UiAlign, UiCommand, UiConfig, UiEdges, UiLength, UiNodeContent, UiNodeId, UiNodeKind,
    UiPosition, UiStyle, UiSubsystem, UiText,
};
#[cfg(windows)]
use rust_kernel_game_engine_kit::subsystems::window::WindowEvent;
use rust_kernel_game_engine_kit::subsystems::window::WindowSize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

#[path = "sudoku_game/game.rs"]
mod game;
#[path = "sudoku_game/input.rs"]
mod input;
#[path = "sudoku_game/layout.rs"]
mod layout;
#[path = "sudoku_game/model.rs"]
mod model;
#[path = "sudoku_game/native.rs"]
mod native;
#[path = "sudoku_game/paint.rs"]
mod paint;
#[path = "sudoku_game/ui.rs"]
mod ui;
use game::SudokuGame;
use input::{action_from_key, GameAction};
use layout::{DifficultyChoice, HitTarget, MenuAction, PlayingAction, Screen, TopAction};
use native::NativeInputBridge;
use paint::NativeSnapshot;
#[cfg(windows)]
use paint::Presenter;
use ui::{build_board, BoardUi};

const WIDTH: u32 = 900;
const HEIGHT: u32 = 840;
const HEADLESS_FRAMES: u64 = 60 * 5;
use model::*;

fn main() {
    let size = WindowSize::new(WIDTH, HEIGHT).expect("static game size is valid");
    let config = UiConfig {
        max_nodes: 256,
        max_depth: 32,
        max_clips: 256,
        max_paint_items: 512,
        max_batches: 256,
        max_commands: 256,
        max_upload_bytes: 256 * 1024,
        ..UiConfig::default()
    };

    let mut ui = UiSubsystem::try_new(config).expect("UI budgets are valid");
    let difficulty = difficulty_from_environment();
    let session = GameSession::new(difficulty);
    let snapshot = Arc::new(std::sync::Mutex::new(NativeSnapshot::from_session(
        Screen::MainMenu,
        &session,
        None,
        false,
        0,
    )));
    let board = build_board(&mut ui, config, session.puzzle);

    let mut kernel = Kernel::with_tracer(FrameTracer::new(4096, Duration::from_secs(3600)));
    let (renderer, native_window) = renderer_bootstrap(size);
    let interactive = native_window.is_some();
    let running = Arc::new(AtomicBool::new(true));
    // The example's native bridge owns the window subsystem slot so native
    // events enter the normal kernel message graph instead of bypassing it.
    kernel
        .register(Box::new(InputSubsystem::default()))
        .expect("register input");
    kernel
        .register(Box::new(ui))
        .expect("register UI subsystem");
    kernel
        .register(Box::new(renderer))
        .expect("register renderer");
    kernel
        .register(Box::new(GamePluginHost::new(SudokuGame::new(
            board,
            session,
            Arc::clone(&snapshot),
            Arc::clone(&running),
        ))))
        .expect("register Sudoku game");
    kernel
        .register(Box::new(NativeInputBridge::new(
            native_window,
            Arc::clone(&running),
            snapshot,
        )))
        .expect("register window bridge");
    kernel.init().expect("initialize Sudoku kernel");
    let frame_budget = if interactive {
        u64::MAX
    } else {
        HEADLESS_FRAMES
    };
    kernel
        .run_until(frame_budget, || running.load(Ordering::Acquire))
        .expect("run Sudoku frames");
    kernel.shutdown();

    let _ = platform::system_clock().now_ns();
}

#[cfg(windows)]
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

    {
        match Win32Window::create("Rust Kernel Sudoku", size.width as i32, size.height as i32) {
            Ok(window) => {
                window.show();
                let handles = PresentHandles::win32(window.instance_handle(), window.raw_handle());
                // The handwritten Vulkan ABI remains an explicit experiment;
                // a stale `RKE_ENABLE_NATIVE_VULKAN=1` must never make the
                // playable example crash on startup.
                let try_native_vulkan = std::env::var("RKE_ENABLE_NATIVE_VULKAN")
                    .map(|value| value == "experimental")
                    .unwrap_or(false);
                if try_native_vulkan {
                    match PresentEngine::build(
                        handles,
                        PresentSize {
                            width: size.width,
                            height: size.height,
                        },
                        [0.018, 0.027, 0.055, 1.0],
                    ) {
                        Ok(engine) => {
                            return (
                                RendererSubsystem::with_native(
                                    engine,
                                    PresentSize {
                                        width: size.width,
                                        height: size.height,
                                    },
                                    [0.018, 0.027, 0.055, 1.0],
                                ),
                                Some(window),
                            )
                        }
                        Err(error) => eprintln!("[sudoku] native Vulkan bootstrap failed: {error}"),
                    }
                }
                window.paint_main_menu();
                return (RendererSubsystem::new(), Some(window));
            }
            Err(error) => eprintln!("[sudoku] Win32 window creation failed: {error}"),
        }
    }
    (RendererSubsystem::new(), None)
}

#[cfg(not(windows))]
fn renderer_bootstrap(_size: WindowSize) -> (RendererSubsystem, Option<()>) {
    (RendererSubsystem::new(), None)
}

fn publish_status(
    ctx: &mut KernelContext<'_>,
    ui: rust_kernel_game_engine_kit::kernel::SubscriberId,
    node: UiNodeId,
    status: &str,
) {
    let _ = ctx.publish(
        ui,
        4,
        UiCommandMessage(UiCommand::SetText {
            node,
            text: UiText::try_new(status, 128).expect("bounded Sudoku status"),
        }),
    );
}

fn publish_visibility(
    ctx: &mut KernelContext<'_>,
    ui: rust_kernel_game_engine_kit::kernel::SubscriberId,
    node: UiNodeId,
    visible: bool,
) {
    let _ = ctx.publish(
        ui,
        4,
        UiCommandMessage(UiCommand::SetVisible { node, visible }),
    );
}

fn publish_cell(
    ctx: &mut KernelContext<'_>,
    ui: rust_kernel_game_engine_kit::kernel::SubscriberId,
    board: &BoardUi,
    session: &GameSession,
    row: usize,
    column: usize,
    selected: bool,
) {
    let mut label = String::with_capacity(9);
    if session.values[row][column] != 0 {
        label.push_str(digit_text(session.values[row][column]));
    } else {
        for (index, marked) in session.notes[row][column].iter().enumerate() {
            if *marked {
                label.push(char::from(b'1' + index as u8));
            }
        }
    }
    let _ = ctx.publish(
        ui,
        4,
        UiCommandMessage(UiCommand::SetText {
            node: board.labels[row][column],
            text: UiText::try_new(&label, 16).expect("bounded Sudoku cell label"),
        }),
    );
    publish_cell_style(
        ctx,
        ui,
        board.cells[row][column],
        session,
        row,
        column,
        selected,
    );
}

fn publish_cell_style(
    ctx: &mut KernelContext<'_>,
    ui: rust_kernel_game_engine_kit::kernel::SubscriberId,
    node: UiNodeId,
    session: &GameSession,
    row: usize,
    column: usize,
    selected: bool,
) {
    let _ = ctx.publish(
        ui,
        4,
        UiCommandMessage(UiCommand::SetStyle {
            node,
            style: cell_style(
                session.values[row][column],
                session.states[row][column],
                selected,
            ),
        }),
    );
}

fn cell_style(value: u8, state: CellState, selected: bool) -> UiStyle {
    UiStyle {
        width: UiLength::Points(62.0),
        height: UiLength::Points(62.0),
        background: if selected {
            [0.72, 0.84, 1.0, 1.0]
        } else if state == CellState::Error {
            [1.0, 0.82, 0.82, 1.0]
        } else if state == CellState::Hint {
            [0.82, 0.95, 0.84, 1.0]
        } else if value == 0 {
            [0.99, 0.99, 1.0, 1.0]
        } else {
            [0.89, 0.92, 0.96, 1.0]
        },
        border: [0.72, 0.78, 0.86, 1.0],
        border_width: 1.0,
        ..UiStyle::default()
    }
}

fn digit_text(value: u8) -> &'static str {
    match value {
        1 => "1",
        2 => "2",
        3 => "3",
        4 => "4",
        5 => "5",
        6 => "6",
        7 => "7",
        8 => "8",
        9 => "9",
        _ => "",
    }
}
