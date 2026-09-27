//! Bounded retained UI tree with generational handles and deterministic order.

use super::budget::{UiConfig, UiError};
use super::components::{UiNodeContent, UiRect};
use super::id::UiNodeId;

#[derive(Clone, Debug)]
struct UiNode {
    id: UiNodeId,
    parent: Option<UiNodeId>,
    children: Vec<UiNodeId>,
    content: UiNodeContent,
    rect: UiRect,
    dirty: bool,
}

pub struct UiTree {
    slots: Vec<Option<UiNode>>,
    generations: Vec<u32>,
    free: Vec<u32>,
    root: UiNodeId,
    count: usize,
    config: UiConfig,
}

impl UiTree {
    pub fn try_new(config: UiConfig) -> Result<Self, UiError> {
        if config.max_nodes == 0 || config.max_depth == 0 {
            return Err(UiError::InvalidValue);
        }
        let root = UiNodeId::new(0, 1);
        let root_node = UiNode {
            id: root,
            parent: None,
            children: Vec::new(),
            content: UiNodeContent::new(super::components::UiNodeKind::Root),
            rect: UiRect::zero(),
            dirty: true,
        };
        let mut slots = Vec::new();
        let mut generations = Vec::new();
        slots
            .try_reserve_exact(config.max_nodes)
            .map_err(|_| UiError::AllocationFailed)?;
        generations
            .try_reserve_exact(config.max_nodes)
            .map_err(|_| UiError::AllocationFailed)?;
        slots.push(Some(root_node));
        generations.push(1);
        Ok(Self {
            slots,
            generations,
            free: Vec::new(),
            root,
            count: 1,
            config,
        })
    }

    pub fn root(&self) -> UiNodeId {
        self.root
    }
    pub fn len(&self) -> usize {
        self.count
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub fn config(&self) -> UiConfig {
        self.config
    }

    fn valid_slot(&self, id: UiNodeId) -> bool {
        self.generations.get(id.index() as usize).copied() == Some(id.generation())
            && self
                .slots
                .get(id.index() as usize)
                .and_then(Option::as_ref)
                .is_some()
    }

    pub fn contains(&self, id: UiNodeId) -> bool {
        self.valid_slot(id)
    }

    pub fn content(&self, id: UiNodeId) -> Result<&UiNodeContent, UiError> {
        self.slots
            .get(id.index() as usize)
            .and_then(Option::as_ref)
            .filter(|node| node.id == id)
            .map(|node| &node.content)
            .ok_or(UiError::InvalidNode)
    }

    pub fn content_mut(&mut self, id: UiNodeId) -> Result<&mut UiNodeContent, UiError> {
        self.slots
            .get_mut(id.index() as usize)
            .and_then(Option::as_mut)
            .filter(|node| node.id == id)
            .map(|node| &mut node.content)
            .ok_or(UiError::InvalidNode)
    }

    pub fn parent(&self, id: UiNodeId) -> Result<Option<UiNodeId>, UiError> {
        self.slots
            .get(id.index() as usize)
            .and_then(Option::as_ref)
            .filter(|node| node.id == id)
            .map(|node| node.parent)
            .ok_or(UiError::InvalidNode)
    }

    pub fn children(&self, id: UiNodeId) -> Result<&[UiNodeId], UiError> {
        self.slots
            .get(id.index() as usize)
            .and_then(Option::as_ref)
            .filter(|node| node.id == id)
            .map(|node| node.children.as_slice())
            .ok_or(UiError::InvalidNode)
    }

    pub fn rect(&self, id: UiNodeId) -> Result<UiRect, UiError> {
        self.slots
            .get(id.index() as usize)
            .and_then(Option::as_ref)
            .filter(|node| node.id == id)
            .map(|node| node.rect)
            .ok_or(UiError::InvalidNode)
    }

    pub fn set_rect(&mut self, id: UiNodeId, rect: UiRect) -> Result<(), UiError> {
        if !rect.sane() {
            return Err(UiError::InvalidValue);
        }
        let node = self.node_mut(id)?;
        node.rect = rect;
        node.dirty = false;
        Ok(())
    }

    pub fn is_dirty(&self, id: UiNodeId) -> Result<bool, UiError> {
        Ok(self.node(id)?.dirty)
    }

    pub fn dirty_count(&self) -> usize {
        self.slots
            .iter()
            .filter_map(Option::as_ref)
            .filter(|node| node.dirty)
            .count()
    }

    fn mark_ancestors_dirty(&mut self, mut id: UiNodeId) -> Result<(), UiError> {
        loop {
            let parent = {
                let node = self.node_mut(id)?;
                node.dirty = true;
                node.parent
            };
            match parent {
                Some(parent) => id = parent,
                None => return Ok(()),
            }
        }
    }

    fn node(&self, id: UiNodeId) -> Result<&UiNode, UiError> {
        self.slots
            .get(id.index() as usize)
            .and_then(Option::as_ref)
            .filter(|node| node.id == id)
            .ok_or(UiError::InvalidNode)
    }

    fn node_mut(&mut self, id: UiNodeId) -> Result<&mut UiNode, UiError> {
        self.slots
            .get_mut(id.index() as usize)
            .and_then(Option::as_mut)
            .filter(|node| node.id == id)
            .ok_or(UiError::InvalidNode)
    }

    fn depth(&self, mut id: UiNodeId) -> Result<usize, UiError> {
        let mut depth = 0usize;
        while let Some(parent) = self.node(id)?.parent {
            depth = depth.checked_add(1).ok_or(UiError::IndexOverflow)?;
            if depth >= self.config.max_depth {
                return Err(UiError::DepthExceeded);
            }
            id = parent;
        }
        Ok(depth)
    }

    pub fn create(
        &mut self,
        parent: UiNodeId,
        content: UiNodeContent,
    ) -> Result<UiNodeId, UiError> {
        if !self.valid_slot(parent) {
            return Err(UiError::InvalidParent);
        }
        if !content.sane() {
            return Err(UiError::InvalidValue);
        }
        if content
            .text
            .as_ref()
            .is_some_and(|text| text.byte_len() > self.config.max_text_bytes)
        {
            return Err(UiError::CapacityExceeded);
        }
        let parent_depth = self.depth(parent)?;
        if parent_depth.checked_add(1).ok_or(UiError::IndexOverflow)? >= self.config.max_depth {
            return Err(UiError::DepthExceeded);
        }
        if self.count >= self.config.max_nodes {
            return Err(UiError::CapacityExceeded);
        }

        let (index, generation) = if let Some(index) = self.free.pop() {
            let generation = self.generations[index as usize];
            (index, generation)
        } else {
            let index = u32::try_from(self.slots.len()).map_err(|_| UiError::IndexOverflow)?;
            self.slots
                .try_reserve(1)
                .map_err(|_| UiError::AllocationFailed)?;
            self.generations
                .try_reserve(1)
                .map_err(|_| UiError::AllocationFailed)?;
            self.slots.push(None);
            self.generations.push(1);
            (index, 1)
        };
        let id = UiNodeId::new(index, generation);
        let mut node = UiNode {
            id,
            parent: Some(parent),
            children: Vec::new(),
            content,
            rect: UiRect::zero(),
            dirty: true,
        };
        node.children
            .try_reserve(4)
            .map_err(|_| UiError::AllocationFailed)?;
        self.node_mut(parent)?
            .children
            .try_reserve(1)
            .map_err(|_| UiError::AllocationFailed)?;
        self.node_mut(parent)?.children.push(id);
        self.slots[index as usize] = Some(node);
        self.count += 1;
        Ok(id)
    }

    pub fn set_content(&mut self, id: UiNodeId, content: UiNodeContent) -> Result<(), UiError> {
        if !content.sane() {
            return Err(UiError::InvalidValue);
        }
        if content
            .text
            .as_ref()
            .is_some_and(|text| text.byte_len() > self.config.max_text_bytes)
        {
            return Err(UiError::CapacityExceeded);
        }
        let node = self.node_mut(id)?;
        node.content = content;
        self.mark_ancestors_dirty(id)
    }

    pub fn reparent(&mut self, id: UiNodeId, new_parent: UiNodeId) -> Result<(), UiError> {
        if id == self.root || !self.valid_slot(new_parent) {
            return Err(UiError::InvalidParent);
        }
        let mut cursor = new_parent;
        loop {
            if cursor == id {
                return Err(UiError::Cycle);
            }
            match self.parent(cursor)? {
                Some(parent) => cursor = parent,
                None => break,
            }
        }
        if self
            .depth(new_parent)?
            .checked_add(1)
            .ok_or(UiError::IndexOverflow)?
            >= self.config.max_depth
        {
            return Err(UiError::DepthExceeded);
        }
        let old_parent = self.parent(id)?.ok_or(UiError::InvalidParent)?;
        // Reserve before unlinking so allocation failure leaves the tree
        // structurally unchanged.
        self.node_mut(new_parent)?
            .children
            .try_reserve(1)
            .map_err(|_| UiError::AllocationFailed)?;
        self.node_mut(old_parent)?
            .children
            .retain(|child| *child != id);
        self.node_mut(new_parent)?.children.push(id);
        let node = self.node_mut(id)?;
        node.parent = Some(new_parent);
        self.mark_ancestors_dirty(id)
    }

    pub fn destroy(&mut self, id: UiNodeId) -> Result<(), UiError> {
        if id == self.root {
            return Err(UiError::InvalidNode);
        }
        let mut order = Vec::new();
        order
            .try_reserve(self.config.max_depth)
            .map_err(|_| UiError::AllocationFailed)?;
        self.collect_subtree(id, &mut order)?;
        self.free
            .try_reserve(order.len())
            .map_err(|_| UiError::AllocationFailed)?;
        for current in &order {
            self.generations[current.index() as usize]
                .checked_add(1)
                .ok_or(UiError::IndexOverflow)?;
        }
        let parent = self.parent(id)?.ok_or(UiError::InvalidNode)?;
        self.node_mut(parent)?.children.retain(|child| *child != id);
        for current in order.into_iter().rev() {
            let index = current.index() as usize;
            self.slots[index] = None;
            // Every generation was checked above before any structural
            // mutation, so this addition cannot overflow.
            self.generations[index] += 1;
            self.free.push(current.index());
            self.count = self.count.saturating_sub(1);
        }
        Ok(())
    }

    fn collect_subtree(&self, id: UiNodeId, out: &mut Vec<UiNodeId>) -> Result<(), UiError> {
        out.push(id);
        for child in self.children(id)? {
            self.collect_subtree(*child, out)?;
        }
        Ok(())
    }

    pub fn preorder(&self, output: &mut Vec<UiNodeId>) -> Result<(), UiError> {
        output.clear();
        output
            .try_reserve(self.count.saturating_sub(output.capacity()))
            .map_err(|_| UiError::AllocationFailed)?;
        self.collect_subtree(self.root, output)
    }
}
