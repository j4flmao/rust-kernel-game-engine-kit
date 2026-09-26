//! UI quotas and diagnostics. Limits are part of the public safety contract.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiConfig {
    pub max_nodes: usize,
    pub max_depth: usize,
    pub max_text_bytes: usize,
    pub max_glyphs: usize,
    pub max_clips: usize,
    pub max_paint_items: usize,
    pub max_batches: usize,
    pub max_commands: usize,
    pub max_upload_bytes: usize,
    pub max_layout_iterations: u8,
    pub dpi_scale: f32,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            max_nodes: 4096,
            max_depth: 128,
            max_text_bytes: 1 << 20,
            max_glyphs: 1 << 20,
            // One retained node may introduce one clip, plus the viewport clip.
            max_clips: 8192,
            max_paint_items: 8192,
            max_batches: 4096,
            max_commands: 4096,
            max_upload_bytes: 16 << 20,
            max_layout_iterations: 8,
            dpi_scale: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiError {
    CapacityExceeded,
    AllocationFailed,
    InvalidNode,
    InvalidParent,
    Cycle,
    DepthExceeded,
    InvalidValue,
    LayoutDidNotConverge,
    OutputTooSmall,
    IndexOverflow,
    ResourceUnavailable,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiDiagnostics {
    pub dirty_nodes: usize,
    pub layout_passes: usize,
    pub paint_items: usize,
    pub batches: usize,
    pub glyphs: usize,
    pub uploaded_bytes: usize,
    pub clipped_items: usize,
    pub overflow_fallbacks: usize,
}

impl UiDiagnostics {
    pub fn record_error(&mut self) {
        self.overflow_fallbacks = self.overflow_fallbacks.saturating_add(1);
    }
}
