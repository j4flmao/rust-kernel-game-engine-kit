//! Deterministic focus, capture, and hit testing for UI input.

use super::components::UiRect;
use super::id::UiNodeId;
use super::tree::UiTree;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiInputEvent {
    PointerMove {
        x: i32,
        y: i32,
    },
    PointerButton {
        button: u8,
        pressed: bool,
        x: i32,
        y: i32,
    },
    Key {
        key: u32,
        pressed: bool,
    },
    TextByte {
        byte: u8,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiInputResult {
    pub target: Option<UiNodeId>,
    pub focused: Option<UiNodeId>,
    pub consumed: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiFocusCapture {
    focused: Option<UiNodeId>,
    captured: Option<UiNodeId>,
}

impl UiFocusCapture {
    pub fn focused(&self) -> Option<UiNodeId> {
        self.focused
    }
    pub fn captured(&self) -> Option<UiNodeId> {
        self.captured
    }
    pub fn clear_stale(&mut self, tree: &UiTree) {
        if self.focused.is_some_and(|id| !tree.contains(id)) {
            self.focused = None;
        }
        if self.captured.is_some_and(|id| !tree.contains(id)) {
            self.captured = None;
        }
    }
    pub fn route(&mut self, tree: &UiTree, event: UiInputEvent) -> UiInputResult {
        self.clear_stale(tree);
        let pointer = match event {
            UiInputEvent::PointerMove { x, y } | UiInputEvent::PointerButton { x, y, .. } => {
                Some((x as f32, y as f32))
            }
            _ => None,
        };
        let target = self
            .captured
            .or_else(|| pointer.and_then(|(x, y)| hit_test(tree, x, y)));
        if let UiInputEvent::PointerButton { pressed: true, .. } = event {
            if target.is_some() {
                self.captured = target;
                self.focused = target.filter(|id| {
                    tree.content(*id)
                        .map(|content| content.interaction.focusable)
                        .unwrap_or(false)
                });
            }
        }
        if let UiInputEvent::PointerButton { pressed: false, .. } = event {
            self.captured = None;
        }
        UiInputResult {
            target,
            focused: self.focused,
            consumed: target.is_some() || self.focused.is_some(),
        }
    }
}

fn hit_test(tree: &UiTree, x: f32, y: f32) -> Option<UiNodeId> {
    let mut order = Vec::new();
    tree.preorder(&mut order).ok()?;
    let mut best: Option<(i32, usize, UiNodeId)> = None;
    for (order_index, id) in order.into_iter().enumerate() {
        let content = match tree.content(id) {
            Ok(content) => content,
            Err(_) => continue,
        };
        let rect: UiRect = match tree.rect(id) {
            Ok(rect) => rect,
            Err(_) => continue,
        };
        if !content.visible
            || !content.interaction.hit_test
            || content.interaction.disabled
            || !rect.contains(x, y)
            || !ancestors_allow_hit(tree, id, x, y)
        {
            continue;
        }
        let candidate = (content.z_index, order_index, id);
        if best
            .as_ref()
            .is_none_or(|current| (candidate.0, candidate.1) >= (current.0, current.1))
        {
            best = Some(candidate);
        }
    }
    best.map(|(_, _, id)| id)
}

fn ancestors_allow_hit(tree: &UiTree, node: UiNodeId, x: f32, y: f32) -> bool {
    let mut cursor = tree.parent(node).ok().flatten();
    while let Some(id) = cursor {
        let content = match tree.content(id) {
            Ok(content) => content,
            Err(_) => return false,
        };
        if !content.visible {
            return false;
        }
        if !matches!(
            content.style.overflow,
            super::components::UiOverflow::Visible
        ) {
            let rect = match tree.rect(id) {
                Ok(rect) => rect,
                Err(_) => return false,
            };
            if !rect.contains(x, y) {
                return false;
            }
        }
        cursor = tree.parent(id).ok().flatten();
    }
    true
}
