//! Bounded UI resource readiness and generation tracking.

use super::budget::{UiConfig, UiError};
use super::id::{FontId, TextureId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiAssetState {
    Missing,
    Loading,
    Ready,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiTextureResource {
    pub id: TextureId,
    pub state: UiAssetState,
    pub generation: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiFontResource {
    pub id: FontId,
    pub state: UiAssetState,
    pub generation: u32,
}

pub struct UiResourceTable {
    textures: Vec<UiTextureResource>,
    fonts: Vec<UiFontResource>,
    config: UiConfig,
}

impl UiResourceTable {
    pub fn try_new(config: UiConfig) -> Result<Self, UiError> {
        let mut textures = Vec::new();
        let mut fonts = Vec::new();
        textures
            .try_reserve_exact(config.max_nodes)
            .map_err(|_| UiError::AllocationFailed)?;
        fonts
            .try_reserve_exact(config.max_nodes)
            .map_err(|_| UiError::AllocationFailed)?;
        Ok(Self {
            textures,
            fonts,
            config,
        })
    }
    pub fn register_texture(&mut self, id: TextureId, state: UiAssetState) -> Result<(), UiError> {
        if let Some(resource) = self.textures.iter_mut().find(|entry| entry.id == id) {
            resource.state = state;
            resource.generation = resource
                .generation
                .checked_add(1)
                .ok_or(UiError::IndexOverflow)?;
            return Ok(());
        }
        if self.textures.len() >= self.config.max_nodes {
            return Err(UiError::CapacityExceeded);
        }
        self.textures.push(UiTextureResource {
            id,
            state,
            generation: 1,
        });
        Ok(())
    }
    pub fn set_texture_state(&mut self, id: TextureId, state: UiAssetState) -> Result<(), UiError> {
        let resource = self
            .textures
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or(UiError::ResourceUnavailable)?;
        resource.state = state;
        resource.generation = resource
            .generation
            .checked_add(1)
            .ok_or(UiError::IndexOverflow)?;
        Ok(())
    }
    pub fn texture(&self, id: TextureId) -> Option<UiTextureResource> {
        self.textures.iter().find(|entry| entry.id == id).copied()
    }

    pub fn texture_ready(&self, id: TextureId) -> bool {
        self.texture(id)
            .is_some_and(|resource| resource.state == UiAssetState::Ready)
    }
    pub fn register_font(&mut self, id: FontId, state: UiAssetState) -> Result<(), UiError> {
        if let Some(resource) = self.fonts.iter_mut().find(|entry| entry.id == id) {
            resource.state = state;
            resource.generation = resource
                .generation
                .checked_add(1)
                .ok_or(UiError::IndexOverflow)?;
            return Ok(());
        }
        if self.fonts.len() >= self.config.max_nodes {
            return Err(UiError::CapacityExceeded);
        }
        self.fonts.push(UiFontResource {
            id,
            state,
            generation: 1,
        });
        Ok(())
    }
    pub fn font(&self, id: FontId) -> Option<UiFontResource> {
        self.fonts.iter().find(|entry| entry.id == id).copied()
    }

    pub fn set_font_state(&mut self, id: FontId, state: UiAssetState) -> Result<(), UiError> {
        let resource = self
            .fonts
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or(UiError::ResourceUnavailable)?;
        resource.state = state;
        resource.generation = resource
            .generation
            .checked_add(1)
            .ok_or(UiError::IndexOverflow)?;
        Ok(())
    }

    pub fn font_ready(&self, id: FontId) -> bool {
        self.font(id)
            .is_some_and(|resource| resource.state == UiAssetState::Ready)
    }

    pub fn evict_texture(&mut self, id: TextureId) -> Result<u32, UiError> {
        let resource = self
            .textures
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or(UiError::ResourceUnavailable)?;
        resource.state = UiAssetState::Missing;
        resource.generation = resource
            .generation
            .checked_add(1)
            .ok_or(UiError::IndexOverflow)?;
        Ok(resource.generation)
    }

    pub fn evict_font(&mut self, id: FontId) -> Result<u32, UiError> {
        let resource = self
            .fonts
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or(UiError::ResourceUnavailable)?;
        resource.state = UiAssetState::Missing;
        resource.generation = resource
            .generation
            .checked_add(1)
            .ok_or(UiError::IndexOverflow)?;
        Ok(resource.generation)
    }
}
