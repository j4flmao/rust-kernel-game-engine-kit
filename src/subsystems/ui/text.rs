//! Deterministic font/glyph contract with a bounded fallback provider.

use super::budget::{UiConfig, UiError};
use super::id::FontId;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glyph {
    pub codepoint: u32,
    pub advance: f32,
    pub atlas_x: u16,
    pub atlas_y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GlyphRun {
    glyphs: Vec<Glyph>,
    width: f32,
    height: f32,
}

impl GlyphRun {
    pub fn glyphs(&self) -> &[Glyph] {
        &self.glyphs
    }
    pub fn width(&self) -> f32 {
        self.width
    }
    pub fn height(&self) -> f32 {
        self.height
    }
}

pub trait GlyphProvider {
    fn shape(&self, font: FontId, text: &str, max_glyphs: usize) -> Result<GlyphRun, UiError>;

    fn shape_wrapped(
        &self,
        font: FontId,
        text: &str,
        max_glyphs: usize,
        max_width: f32,
    ) -> Result<GlyphRun, UiError> {
        if !max_width.is_finite() || max_width <= 0.0 {
            return Err(UiError::InvalidValue);
        }
        self.shape(font, text, max_glyphs)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FallbackGlyphProvider {
    pub advance: f32,
    pub line_height: f32,
}

impl Default for FallbackGlyphProvider {
    fn default() -> Self {
        Self {
            advance: 8.0,
            line_height: 16.0,
        }
    }
}

impl GlyphProvider for FallbackGlyphProvider {
    fn shape(&self, _font: FontId, text: &str, max_glyphs: usize) -> Result<GlyphRun, UiError> {
        if !self.advance.is_finite()
            || self.advance <= 0.0
            || self.advance > f32::from(u16::MAX)
            || !self.line_height.is_finite()
            || self.line_height <= 0.0
            || self.line_height > f32::from(u16::MAX)
        {
            return Err(UiError::InvalidValue);
        }
        let count = text.chars().count();
        if count > max_glyphs {
            return Err(UiError::CapacityExceeded);
        }
        let mut glyphs = Vec::new();
        glyphs
            .try_reserve_exact(count)
            .map_err(|_| UiError::AllocationFailed)?;
        for (index, ch) in text.chars().enumerate() {
            glyphs.push(Glyph {
                codepoint: ch as u32,
                advance: self.advance,
                atlas_x: (index % 256) as u16,
                atlas_y: (index / 256) as u16,
                width: self.advance as u16,
                height: self.line_height as u16,
            });
        }
        Ok(GlyphRun {
            glyphs,
            width: self.advance * count as f32,
            height: self.line_height,
        })
    }

    fn shape_wrapped(
        &self,
        _font: FontId,
        text: &str,
        max_glyphs: usize,
        max_width: f32,
    ) -> Result<GlyphRun, UiError> {
        if !self.advance.is_finite()
            || self.advance <= 0.0
            || self.advance > f32::from(u16::MAX)
            || !self.line_height.is_finite()
            || self.line_height <= 0.0
            || self.line_height > f32::from(u16::MAX)
            || !max_width.is_finite()
            || max_width <= 0.0
        {
            return Err(UiError::InvalidValue);
        }
        let count = text.chars().count();
        if count > max_glyphs {
            return Err(UiError::CapacityExceeded);
        }
        let columns = (max_width / self.advance).floor() as usize;
        if columns == 0 {
            return Err(UiError::InvalidValue);
        }
        let mut glyphs = Vec::new();
        glyphs
            .try_reserve_exact(count)
            .map_err(|_| UiError::AllocationFailed)?;
        let mut column = 0usize;
        let mut line = 0usize;
        let mut width = 0.0f32;
        for (index, ch) in text.chars().enumerate() {
            if ch == '\n' || column >= columns {
                line = line.checked_add(1).ok_or(UiError::IndexOverflow)?;
                column = 0;
                if ch == '\n' {
                    continue;
                }
            }
            glyphs.push(Glyph {
                codepoint: ch as u32,
                advance: self.advance,
                atlas_x: (index % 256) as u16,
                atlas_y: (index / 256) as u16,
                width: self.advance as u16,
                height: self.line_height as u16,
            });
            column = column.saturating_add(1);
            width = width.max(column as f32 * self.advance);
        }
        let lines = line.saturating_add(1);
        Ok(GlyphRun {
            glyphs,
            width,
            height: lines as f32 * self.line_height,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasSlot {
    pub generation: u32,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

pub struct GlyphAtlas {
    slots: Vec<Option<AtlasSlot>>,
    capacity: usize,
    next: usize,
}

impl GlyphAtlas {
    pub fn try_new(config: UiConfig) -> Result<Self, UiError> {
        let capacity = config.max_glyphs.min(65_536);
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(capacity)
            .map_err(|_| UiError::AllocationFailed)?;
        slots.resize(capacity, None);
        Ok(Self {
            slots,
            capacity,
            next: 0,
        })
    }
    pub fn allocate(&mut self, width: u16, height: u16) -> Result<AtlasSlot, UiError> {
        if width == 0 || height == 0 || self.capacity == 0 {
            return Err(UiError::InvalidValue);
        }
        let index = self.next % self.capacity;
        self.next = self.next.checked_add(1).ok_or(UiError::IndexOverflow)?;
        let generation = u32::try_from(self.next).map_err(|_| UiError::IndexOverflow)?;
        let slot = AtlasSlot {
            generation,
            x: (index % 256) as u16 * 64,
            y: (index / 256) as u16 * 64,
            width,
            height,
        };
        self.slots[index] = Some(slot);
        Ok(slot)
    }

    pub fn evict_generation(&mut self, generation: u32) -> usize {
        let mut evicted = 0usize;
        for slot in &mut self.slots {
            if slot.is_some_and(|value| value.generation == generation) {
                *slot = None;
                evicted = evicted.saturating_add(1);
            }
        }
        evicted
    }

    pub fn occupied(&self) -> usize {
        self.slots.iter().filter(|slot| slot.is_some()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_provider_wraps_without_exceeding_width() {
        let provider = FallbackGlyphProvider::default();
        let run = provider
            .shape_wrapped(FontId::new(1, 1), "abcdefgh", 32, 24.0)
            .unwrap();
        assert_eq!(run.glyphs().len(), 8);
        assert_eq!(run.height(), 48.0);
        assert!(run.width() <= 24.0);
    }

    #[test]
    fn atlas_eviction_invalidates_generation() {
        let mut atlas = GlyphAtlas::try_new(UiConfig {
            max_glyphs: 2,
            ..UiConfig::default()
        })
        .unwrap();
        let first = atlas.allocate(8, 8).unwrap();
        assert_eq!(atlas.occupied(), 1);
        assert_eq!(atlas.evict_generation(first.generation), 1);
        assert_eq!(atlas.occupied(), 0);
    }

    #[test]
    fn fallback_provider_rejects_unrepresentable_atlas_metrics() {
        let provider = FallbackGlyphProvider {
            advance: 65_536.0,
            line_height: 16.0,
        };
        assert_eq!(
            provider.shape(FontId::new(1, 1), "x", 1),
            Err(UiError::InvalidValue)
        );
    }
}
