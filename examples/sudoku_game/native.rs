use super::*;

#[cfg(windows)]
type NativeWindow = rust_kernel_game_engine_kit::platform::windows::window::Win32Window;
#[cfg(not(windows))]
type NativeWindow = ();

pub(crate) struct NativeInputBridge {
    running: Arc<AtomicBool>,
    snapshot: Arc<std::sync::Mutex<NativeSnapshot>>,
    #[cfg(windows)]
    window: Option<NativeWindow>,
    #[cfg(windows)]
    window_events: Vec<WindowEvent>,
    #[cfg(windows)]
    input_events: Vec<InputEvent>,
    #[cfg(windows)]
    presenter: Option<Presenter>,
    #[cfg(windows)]
    last_painted: Option<NativeSnapshot>,
}

impl NativeInputBridge {
    pub(crate) fn new(
        window: Option<NativeWindow>,
        running: Arc<AtomicBool>,
        snapshot: Arc<std::sync::Mutex<NativeSnapshot>>,
    ) -> Self {
        #[cfg(windows)]
        {
            let presenter = window
                .as_ref()
                .map(|window| Presenter::new(window.raw_handle()));
            return Self {
                window,
                running,
                snapshot,
                window_events: Vec::with_capacity(16),
                input_events: Vec::with_capacity(32),
                presenter,
                last_painted: None,
            };
        }
        #[cfg(not(windows))]
        {
            let _ = window;
            Self { running, snapshot }
        }
    }
}

impl Subsystem for NativeInputBridge {
    fn name(&self) -> &'static str {
        "window"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &[]
    }

    fn init(&mut self, _ctx: &mut KernelContext<'_>) {}

    fn tick(&mut self, ctx: &mut KernelContext<'_>, _dt_ns: u64) {
        #[cfg(windows)]
        if let Some(window) = &self.window {
            window.poll_events(&mut self.window_events, &mut self.input_events);
            if let Some(input) = ctx.resolve("input") {
                for event in self.input_events.drain(..) {
                    let _ = ctx.publish(input, 4, InputEventMessage(event));
                }
            }
            if std::env::var_os("RKE_DISABLE_GDI").is_none() {
                if let (Some(presenter), Ok(snapshot)) = (&self.presenter, self.snapshot.lock()) {
                    if self.last_painted.as_ref() != Some(&*snapshot) {
                        presenter.paint(&snapshot);
                        self.last_painted = Some(*snapshot);
                    }
                }
            }
            for event in self.window_events.drain(..) {
                if let WindowEvent::Resized(size) = event {
                    let message = WindowResized {
                        width: size.width,
                        height: size.height,
                    };
                    if let Some(ui) = ctx.resolve("ui") {
                        let _ = ctx.publish(ui, 1, message);
                    }
                    if let Some(renderer) = ctx.resolve("renderer") {
                        let _ = ctx.publish(renderer, 1, message);
                    }
                } else if matches!(event, WindowEvent::CloseRequested) {
                    self.running.store(false, Ordering::Release);
                }
            }
            // Bound the native example loop to a responsive frame cadence;
            // without this, the kernel correctly runs as fast as possible but
            // starves Win32's compositor and makes input feel laggy.
            std::thread::sleep(Duration::from_millis(8));
        }
    }

    fn shutdown(&mut self, _ctx: &mut KernelContext<'_>) {}
}
