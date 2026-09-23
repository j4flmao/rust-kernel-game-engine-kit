use rust_kernel_game_engine_kit::kernel::{Kernel, KernelContext, Subsystem};
use rust_kernel_game_engine_kit::subsystems::{
    input::{InputEvent, InputSubsystem},
    messages::{InputFrame, WindowResized},
    window::{WindowEvent, WindowId, WindowSize, WindowSubsystem},
};
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

struct RendererProbe {
    resize: Arc<AtomicU32>,
}
impl Subsystem for RendererProbe {
    fn name(&self) -> &'static str {
        "renderer"
    }
    fn dependencies(&self) -> &'static [&'static str] {
        &["window"]
    }
    fn init(&mut self, _: &mut KernelContext<'_>) {}
    fn tick(&mut self, ctx: &mut KernelContext<'_>, _: u64) {
        for message in ctx.receive() {
            if let Ok(size) = message.downcast::<WindowResized>() {
                self.resize
                    .store(size.width ^ size.height, Ordering::Release);
            }
        }
    }
    fn shutdown(&mut self, _: &mut KernelContext<'_>) {}
}

struct PhysicsProbe {
    input: Arc<AtomicU32>,
}
impl Subsystem for PhysicsProbe {
    fn name(&self) -> &'static str {
        "physics"
    }
    fn dependencies(&self) -> &'static [&'static str] {
        &["input"]
    }
    fn init(&mut self, _: &mut KernelContext<'_>) {}
    fn tick(&mut self, ctx: &mut KernelContext<'_>, _: u64) {
        for message in ctx.receive() {
            if let Ok(frame) = message.downcast::<InputFrame>() {
                self.input
                    .store(frame.mouse_buttons as u32, Ordering::Release);
            }
        }
    }
    fn shutdown(&mut self, _: &mut KernelContext<'_>) {}
}

#[test]
fn window_and_input_publish_to_dependent_subsystems() {
    let mut window = WindowSubsystem::new(WindowId(1), WindowSize::new(640, 480).unwrap());
    window
        .window_mut()
        .push_event(WindowEvent::Resized(WindowSize::new(1280, 720).unwrap()));
    let mut input = InputSubsystem::default();
    input.input_mut().push(InputEvent::MouseButton {
        button: 3,
        pressed: true,
    });

    let resize = Arc::new(AtomicU32::new(0));
    let buttons = Arc::new(AtomicU32::new(0));
    let mut kernel = Kernel::new();
    kernel.register(Box::new(window)).unwrap();
    kernel.register(Box::new(input)).unwrap();
    kernel
        .register(Box::new(RendererProbe {
            resize: Arc::clone(&resize),
        }))
        .unwrap();
    kernel
        .register(Box::new(PhysicsProbe {
            input: Arc::clone(&buttons),
        }))
        .unwrap();
    kernel.init().unwrap();
    kernel.run(1).unwrap();
    assert_eq!(resize.load(Ordering::Acquire), 1280 ^ 720);
    assert_eq!(buttons.load(Ordering::Acquire), 1 << 3);
    kernel.shutdown();
}
