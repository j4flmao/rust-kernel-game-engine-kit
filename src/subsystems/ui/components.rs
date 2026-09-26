//! Typed UI component values. No Vulkan or operating-system handles live here.

use super::id::{FontId, TextureId};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum UiLength {
    #[default]
    Auto,
    Points(f32),
    Percent(f32),
}

impl UiLength {
    pub fn resolve(self, available: f32) -> Option<f32> {
        if !available.is_finite() || available < 0.0 {
            return None;
        }
        match self {
            Self::Auto => None,
            Self::Points(value) if value.is_finite() && value >= 0.0 => Some(value),
            Self::Percent(value) if value.is_finite() && (0.0..=100.0).contains(&value) => {
                Some(available * value / 100.0)
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UiEdges {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl UiEdges {
    pub const fn all(value: f32) -> Self {
        Self {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }
    pub fn horizontal(self) -> f32 {
        self.left + self.right
    }
    pub fn vertical(self) -> f32 {
        self.top + self.bottom
    }
    pub fn sane(self) -> bool {
        [self.left, self.right, self.top, self.bottom]
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UiRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl UiRect {
    pub const fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }
    pub fn sane(self) -> bool {
        [
            self.x,
            self.y,
            self.width,
            self.height,
            self.right(),
            self.bottom(),
        ]
        .iter()
        .all(|value| value.is_finite())
            && self.width >= 0.0
            && self.height >= 0.0
    }
    pub fn right(self) -> f32 {
        self.x + self.width
    }
    pub fn bottom(self) -> f32 {
        self.y + self.height
    }
    pub fn contains(self, x: f32, y: f32) -> bool {
        self.sane() && x >= self.x && y >= self.y && x < self.right() && y < self.bottom()
    }
    pub fn intersect(self, other: Self) -> Self {
        let left = self.x.max(other.x);
        let top = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        Self {
            x: left,
            y: top,
            width: (right - left).max(0.0),
            height: (bottom - top).max(0.0),
        }
    }
    pub fn area(self) -> f32 {
        self.width * self.height
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiDirection {
    Row,
    #[default]
    Column,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiAlign {
    #[default]
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiOverflow {
    #[default]
    Visible,
    Clip,
    Scroll,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum UiPosition {
    #[default]
    Flow,
    Absolute {
        left: f32,
        top: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiStyle {
    pub width: UiLength,
    pub height: UiLength,
    pub min_width: f32,
    pub min_height: f32,
    pub max_width: f32,
    pub max_height: f32,
    pub margin: UiEdges,
    pub padding: UiEdges,
    pub gap: f32,
    pub direction: UiDirection,
    pub align: UiAlign,
    pub position: UiPosition,
    pub background: [f32; 4],
    pub gradient: Option<([f32; 4], [f32; 4])>,
    pub border: [f32; 4],
    pub border_width: f32,
    pub corner_radius: f32,
    pub overflow: UiOverflow,
    pub scroll_x: f32,
    pub scroll_y: f32,
}

impl Default for UiStyle {
    fn default() -> Self {
        Self {
            width: UiLength::Auto,
            height: UiLength::Auto,
            min_width: 0.0,
            min_height: 0.0,
            max_width: f32::MAX,
            max_height: f32::MAX,
            margin: UiEdges::default(),
            padding: UiEdges::default(),
            gap: 0.0,
            direction: UiDirection::Column,
            align: UiAlign::Stretch,
            position: UiPosition::Flow,
            background: [0.0, 0.0, 0.0, 0.0],
            gradient: None,
            border: [0.0, 0.0, 0.0, 0.0],
            border_width: 0.0,
            corner_radius: 0.0,
            overflow: UiOverflow::Visible,
            scroll_x: 0.0,
            scroll_y: 0.0,
        }
    }
}

impl UiStyle {
    pub fn sane(self) -> bool {
        self.margin.sane()
            && self.padding.sane()
            && self.gap.is_finite()
            && self.gap >= 0.0
            && self.min_width.is_finite()
            && self.min_height.is_finite()
            && self.max_width.is_finite()
            && self.max_height.is_finite()
            && self.min_width >= 0.0
            && self.min_height >= 0.0
            && self.max_width >= self.min_width
            && self.max_height >= self.min_height
            && self.border_width.is_finite()
            && self.border_width >= 0.0
            && match self.position {
                UiPosition::Flow => true,
                UiPosition::Absolute { left, top } => left.is_finite() && top.is_finite(),
            }
            && self.background.iter().all(|v| v.is_finite())
            && self.border.iter().all(|v| v.is_finite())
            && self.gradient.is_none_or(|(from, to)| {
                from.iter().chain(to.iter()).all(|value| value.is_finite())
            })
            && self.corner_radius.is_finite()
            && self.corner_radius >= 0.0
            && self.scroll_x.is_finite()
            && self.scroll_y.is_finite()
            && self.scroll_x >= 0.0
            && self.scroll_y >= 0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiNodeKind {
    Root,
    Panel,
    Text,
    Image,
    Custom,
}

/// Stable semantic role for accessibility adapters outside the kernel.
/// Platform accessibility APIs consume this metadata; they never cross into
/// the Vulkan or retained-tree ownership boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiAccessibilityRole {
    #[default]
    None,
    Button,
    Label,
    Image,
    TextField,
    Window,
    List,
    ListItem,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiText {
    value: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UiAccessibility {
    pub role: UiAccessibilityRole,
    pub label: Option<UiText>,
    pub hidden: bool,
}

impl UiText {
    pub fn try_new(value: &str, max_bytes: usize) -> Result<Self, super::budget::UiError> {
        if value.len() > max_bytes || !value.is_char_boundary(value.len()) {
            return Err(super::budget::UiError::CapacityExceeded);
        }
        let mut output = String::new();
        output
            .try_reserve_exact(value.len())
            .map_err(|_| super::budget::UiError::AllocationFailed)?;
        output.push_str(value);
        Ok(Self { value: output })
    }
    pub fn as_str(&self) -> &str {
        &self.value
    }
    pub fn byte_len(&self) -> usize {
        self.value.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiImage {
    pub texture: TextureId,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiInteraction {
    pub focusable: bool,
    pub disabled: bool,
    pub hit_test: bool,
}

impl Default for UiInteraction {
    fn default() -> Self {
        Self {
            focusable: false,
            disabled: false,
            hit_test: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiNodeContent {
    pub kind: UiNodeKind,
    pub style: UiStyle,
    pub visible: bool,
    pub z_index: i32,
    pub opacity: f32,
    pub text: Option<UiText>,
    pub image: Option<UiImage>,
    pub font: Option<FontId>,
    pub interaction: UiInteraction,
    pub accessibility: UiAccessibility,
}

impl UiNodeContent {
    pub fn new(kind: UiNodeKind) -> Self {
        Self {
            kind,
            style: UiStyle::default(),
            visible: true,
            z_index: 0,
            opacity: 1.0,
            text: None,
            image: None,
            font: None,
            interaction: UiInteraction::default(),
            accessibility: UiAccessibility::default(),
        }
    }
    pub fn sane(&self) -> bool {
        self.style.sane() && self.opacity.is_finite() && (0.0..=1.0).contains(&self.opacity)
    }
}
