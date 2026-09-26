//! Deferred UI mutations applied at a deterministic frame boundary.

use super::budget::UiError;
use super::components::{UiAccessibility, UiNodeContent, UiStyle, UiText};
use super::id::UiNodeId;
use super::tree::UiTree;

#[derive(Clone, Debug, PartialEq)]
pub enum UiCommand {
    Create {
        parent: UiNodeId,
        content: UiNodeContent,
    },
    Destroy(UiNodeId),
    Reparent {
        node: UiNodeId,
        parent: UiNodeId,
    },
    SetStyle {
        node: UiNodeId,
        style: UiStyle,
    },
    SetText {
        node: UiNodeId,
        text: UiText,
    },
    SetVisible {
        node: UiNodeId,
        visible: bool,
    },
    SetZIndex {
        node: UiNodeId,
        z_index: i32,
    },
    SetAccessibility {
        node: UiNodeId,
        accessibility: UiAccessibility,
    },
}

pub struct UiCommandBuffer {
    commands: Vec<UiCommand>,
    capacity: usize,
}

impl UiCommandBuffer {
    pub fn try_with_capacity(capacity: usize) -> Result<Self, UiError> {
        let mut commands = Vec::new();
        commands
            .try_reserve_exact(capacity)
            .map_err(|_| UiError::AllocationFailed)?;
        Ok(Self { commands, capacity })
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self::try_with_capacity(capacity).expect("UI command allocation failed")
    }
    pub fn try_push(&mut self, command: UiCommand) -> Result<(), UiError> {
        if self.commands.len() >= self.capacity {
            return Err(UiError::CapacityExceeded);
        }
        self.commands.push(command);
        Ok(())
    }
    pub fn len(&self) -> usize {
        self.commands.len()
    }
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    pub fn apply(&mut self, tree: &mut UiTree) -> usize {
        let mut applied = 0usize;
        for command in self.commands.drain(..) {
            let result = match command {
                UiCommand::Create { parent, content } => tree.create(parent, content).map(|_| ()),
                UiCommand::Destroy(node) => tree.destroy(node),
                UiCommand::Reparent { node, parent } => tree.reparent(node, parent),
                UiCommand::SetStyle { node, style } if style.sane() => {
                    tree.content_mut(node).map(|content| {
                        content.style = style;
                    })
                }
                UiCommand::SetStyle { .. } => Err(UiError::InvalidValue),
                UiCommand::SetText { node, text }
                    if text.byte_len() <= tree.config().max_text_bytes =>
                {
                    tree.content_mut(node).map(|content| {
                        content.text = Some(text);
                    })
                }
                UiCommand::SetText { .. } => Err(UiError::CapacityExceeded),
                UiCommand::SetVisible { node, visible } => tree.content_mut(node).map(|content| {
                    content.visible = visible;
                }),
                UiCommand::SetZIndex { node, z_index } => tree.content_mut(node).map(|content| {
                    content.z_index = z_index;
                }),
                UiCommand::SetAccessibility {
                    node,
                    accessibility,
                } => tree.content_mut(node).map(|content| {
                    content.accessibility = accessibility;
                }),
            };
            if result.is_ok() {
                applied = applied.saturating_add(1);
            }
        }
        applied
    }
}
