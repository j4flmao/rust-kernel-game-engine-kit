use rust_kernel_game_engine_kit::kernel::Kernel;
use rust_kernel_game_engine_kit::subsystems::input::{InputEvent, InputSubsystem};
use rust_kernel_game_engine_kit::subsystems::window::{
    WindowEvent, WindowId, WindowSize, WindowSubsystem,
};

#[test]
fn headless_window_and_input_run_through_kernel() {
    let size = WindowSize::new(1280, 720).expect("valid test window size");
    let mut window = WindowSubsystem::new(WindowId(1), size);
    window.window_mut().push_event(WindowEvent::Focused(false));

    let mut input = InputSubsystem::default();
    input.input_mut().push(InputEvent::Key {
        key: 32,
        pressed: true,
    });

    let mut kernel = Kernel::new();
    kernel
        .register(Box::new(input))
        .expect("input registration");
    kernel
        .register(Box::new(window))
        .expect("window registration");
    kernel.init().expect("headless phase must initialize");
    kernel.run(1).expect("headless phase must tick");
    kernel.shutdown();
}
