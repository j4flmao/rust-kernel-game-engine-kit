//! Bounded contracts and resource tables for opaque 3D world rendering.
//!
//! This module deliberately contains no Vulkan handles.  It is the safe
//! renderer-side boundary used by both the headless reference path and native
//! backends.  Native platform code may copy the validated ranges into GPU
//! resources, but it must not reinterpret gameplay-owned pointers or offsets.

use core::fmt;

const INITIAL_GENERATION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MeshHandle {
    index: u32,
    generation: u32,
}

impl MeshHandle {
    pub const INVALID: Self = Self {
        index: u32::MAX,
        generation: 0,
    };

    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    pub const fn index(self) -> u32 {
        self.index
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }

    pub const fn is_valid(self) -> bool {
        self.index != u32::MAX && self.generation != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MaterialHandle {
    index: u32,
    generation: u32,
}

impl MaterialHandle {
    pub const INVALID: Self = Self {
        index: u32::MAX,
        generation: 0,
    };

    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    pub const fn index(self) -> u32 {
        self.index
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }

    pub const fn is_valid(self) -> bool {
        self.index != u32::MAX && self.generation != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Renderer3dError {
    CapacityExceeded,
    AllocationFailed,
    InvalidHandle,
    InvalidVertexFormat,
    InvalidIndexData,
    InvalidBounds,
    IntegerOverflow,
    InvalidView,
    InvalidPass,
}

impl fmt::Display for Renderer3dError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::CapacityExceeded => "3D resource capacity exceeded",
            Self::AllocationFailed => "3D resource allocation failed",
            Self::InvalidHandle => "invalid or stale 3D resource handle",
            Self::InvalidVertexFormat => "invalid mesh vertex format",
            Self::InvalidIndexData => "invalid mesh index data",
            Self::InvalidBounds => "invalid mesh bounds",
            Self::IntegerOverflow => "3D resource size overflow",
            Self::InvalidView => "invalid 3D render view",
            Self::InvalidPass => "invalid 3D render pass",
        };
        f.write_str(message)
    }
}

impl std::error::Error for Renderer3dError {}

/// A validated, backend-neutral mesh range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshRecord {
    pub vertex_offset: u64,
    pub vertex_bytes: u64,
    pub vertex_stride: u32,
    pub vertex_count: u32,
    pub index_offset: u64,
    pub index_bytes: u64,
    pub index_count: u32,
    pub bounds: [f32; 4],
}

#[derive(Clone, Copy, Debug)]
struct MeshSlot {
    generation: u32,
    record: Option<MeshRecord>,
}

/// Bounded immutable mesh storage owned by the renderer.
pub struct MeshTable {
    slots: Vec<MeshSlot>,
    free: Vec<u32>,
    vertex_data: Vec<u8>,
    index_data: Vec<u8>,
    max_meshes: usize,
    max_vertex_bytes: usize,
    max_index_bytes: usize,
}

impl MeshTable {
    pub fn try_with_capacity(
        max_meshes: usize,
        max_vertex_bytes: usize,
        max_index_bytes: usize,
    ) -> Result<Self, Renderer3dError> {
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(max_meshes)
            .map_err(|_| Renderer3dError::AllocationFailed)?;
        let mut vertex_data = Vec::new();
        vertex_data
            .try_reserve_exact(max_vertex_bytes)
            .map_err(|_| Renderer3dError::AllocationFailed)?;
        let mut index_data = Vec::new();
        index_data
            .try_reserve_exact(max_index_bytes)
            .map_err(|_| Renderer3dError::AllocationFailed)?;
        Ok(Self {
            slots,
            free: Vec::new(),
            vertex_data,
            index_data,
            max_meshes,
            max_vertex_bytes,
            max_index_bytes,
        })
    }

    pub fn with_capacity(
        max_meshes: usize,
        max_vertex_bytes: usize,
        max_index_bytes: usize,
    ) -> Self {
        Self::try_with_capacity(max_meshes, max_vertex_bytes, max_index_bytes)
            .expect("3D mesh table allocation failed")
    }

    pub const fn len(&self) -> usize {
        self.slots.len() - self.free.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub const fn max_meshes(&self) -> usize {
        self.max_meshes
    }

    pub const fn vertex_bytes_len(&self) -> usize {
        self.vertex_data.len()
    }

    pub const fn index_bytes_len(&self) -> usize {
        self.index_data.len()
    }

    pub const fn max_vertex_bytes(&self) -> usize {
        self.max_vertex_bytes
    }

    pub const fn max_index_bytes(&self) -> usize {
        self.max_index_bytes
    }

    pub fn try_insert(
        &mut self,
        vertex_bytes: &[u8],
        vertex_stride: u32,
        index_bytes: &[u8],
        bounds: [f32; 4],
    ) -> Result<MeshHandle, Renderer3dError> {
        validate_mesh_input(vertex_bytes, vertex_stride, index_bytes, bounds)?;
        if self.len() >= self.max_meshes {
            return Err(Renderer3dError::CapacityExceeded);
        }
        let vertex_end = self
            .vertex_data
            .len()
            .checked_add(vertex_bytes.len())
            .ok_or(Renderer3dError::IntegerOverflow)?;
        let index_end = self
            .index_data
            .len()
            .checked_add(index_bytes.len())
            .ok_or(Renderer3dError::IntegerOverflow)?;
        if vertex_end > self.max_vertex_bytes || index_end > self.max_index_bytes {
            return Err(Renderer3dError::CapacityExceeded);
        }

        let record = MeshRecord {
            vertex_offset: u64::try_from(self.vertex_data.len())
                .map_err(|_| Renderer3dError::IntegerOverflow)?,
            vertex_bytes: u64::try_from(vertex_bytes.len())
                .map_err(|_| Renderer3dError::IntegerOverflow)?,
            vertex_stride,
            vertex_count: u32::try_from(vertex_bytes.len() / vertex_stride as usize)
                .map_err(|_| Renderer3dError::IntegerOverflow)?,
            index_offset: u64::try_from(self.index_data.len())
                .map_err(|_| Renderer3dError::IntegerOverflow)?,
            index_bytes: u64::try_from(index_bytes.len())
                .map_err(|_| Renderer3dError::IntegerOverflow)?,
            index_count: u32::try_from(index_bytes.len() / core::mem::size_of::<u32>())
                .map_err(|_| Renderer3dError::IntegerOverflow)?,
            bounds,
        };

        let slot_index = if let Some(index) = self.free.pop() {
            index
        } else {
            let index =
                u32::try_from(self.slots.len()).map_err(|_| Renderer3dError::IntegerOverflow)?;
            self.slots
                .try_reserve(1)
                .map_err(|_| Renderer3dError::AllocationFailed)?;
            self.slots.push(MeshSlot {
                generation: INITIAL_GENERATION,
                record: None,
            });
            index
        };
        let slot = self
            .slots
            .get_mut(slot_index as usize)
            .ok_or(Renderer3dError::InvalidHandle)?;
        // The backing vectors were reserved to their hard maxima during
        // construction, and the checked end offsets above prove these copies
        // cannot grow them. No fallible operation remains after slot reuse.
        self.vertex_data.extend_from_slice(vertex_bytes);
        self.index_data.extend_from_slice(index_bytes);
        slot.record = Some(record);
        Ok(MeshHandle::new(slot_index, slot.generation))
    }

    pub fn get(&self, handle: MeshHandle) -> Result<MeshRecord, Renderer3dError> {
        let slot = self.valid_slot(handle)?;
        slot.record.ok_or(Renderer3dError::InvalidHandle)
    }

    pub fn vertex_bytes(&self, handle: MeshHandle) -> Result<&[u8], Renderer3dError> {
        let record = self.get(handle)?;
        let start =
            usize::try_from(record.vertex_offset).map_err(|_| Renderer3dError::IntegerOverflow)?;
        let len =
            usize::try_from(record.vertex_bytes).map_err(|_| Renderer3dError::IntegerOverflow)?;
        let end = start
            .checked_add(len)
            .ok_or(Renderer3dError::IntegerOverflow)?;
        self.vertex_data
            .get(start..end)
            .ok_or(Renderer3dError::InvalidHandle)
    }

    pub fn index_bytes(&self, handle: MeshHandle) -> Result<&[u8], Renderer3dError> {
        let record = self.get(handle)?;
        let start =
            usize::try_from(record.index_offset).map_err(|_| Renderer3dError::IntegerOverflow)?;
        let len =
            usize::try_from(record.index_bytes).map_err(|_| Renderer3dError::IntegerOverflow)?;
        let end = start
            .checked_add(len)
            .ok_or(Renderer3dError::IntegerOverflow)?;
        self.index_data
            .get(start..end)
            .ok_or(Renderer3dError::InvalidHandle)
    }

    pub fn remove(&mut self, handle: MeshHandle) -> Result<MeshRecord, Renderer3dError> {
        if !handle.is_valid() {
            return Err(Renderer3dError::InvalidHandle);
        }
        let index = usize::try_from(handle.index).map_err(|_| Renderer3dError::IntegerOverflow)?;
        let slot = self
            .slots
            .get(index)
            .ok_or(Renderer3dError::InvalidHandle)?;
        if slot.generation != handle.generation || slot.record.is_none() {
            return Err(Renderer3dError::InvalidHandle);
        }
        self.free
            .try_reserve(1)
            .map_err(|_| Renderer3dError::AllocationFailed)?;
        let record = {
            let slot = self
                .slots
                .get_mut(index)
                .ok_or(Renderer3dError::InvalidHandle)?;
            let record = slot.record.take().ok_or(Renderer3dError::InvalidHandle)?;
            slot.generation = next_generation(slot.generation);
            record
        };
        self.free.push(handle.index);
        Ok(record)
    }

    fn valid_slot(&self, handle: MeshHandle) -> Result<&MeshSlot, Renderer3dError> {
        if !handle.is_valid() {
            return Err(Renderer3dError::InvalidHandle);
        }
        let slot = self
            .slots
            .get(handle.index as usize)
            .ok_or(Renderer3dError::InvalidHandle)?;
        if slot.generation != handle.generation || slot.record.is_none() {
            return Err(Renderer3dError::InvalidHandle);
        }
        Ok(slot)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterialRecord {
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
}

#[derive(Clone, Copy, Debug)]
struct MaterialSlot {
    generation: u32,
    record: Option<MaterialRecord>,
}

/// Bounded material table.  Textures are intentionally separate resources;
/// this first contract only covers scalar material parameters.
pub struct MaterialTable {
    slots: Vec<MaterialSlot>,
    free: Vec<u32>,
    max_materials: usize,
}

impl MaterialTable {
    pub fn try_with_capacity(max_materials: usize) -> Result<Self, Renderer3dError> {
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(max_materials)
            .map_err(|_| Renderer3dError::AllocationFailed)?;
        Ok(Self {
            slots,
            free: Vec::new(),
            max_materials,
        })
    }

    pub fn with_capacity(max_materials: usize) -> Self {
        Self::try_with_capacity(max_materials).expect("3D material table allocation failed")
    }

    pub const fn len(&self) -> usize {
        self.slots.len() - self.free.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub const fn max_materials(&self) -> usize {
        self.max_materials
    }

    pub fn try_insert(
        &mut self,
        record: MaterialRecord,
    ) -> Result<MaterialHandle, Renderer3dError> {
        if self.len() >= self.max_materials
            || !record.base_color.iter().all(|value| value.is_finite())
            || !record.metallic.is_finite()
            || !record.roughness.is_finite()
            || !(0.0..=1.0).contains(&record.metallic)
            || !(0.0..=1.0).contains(&record.roughness)
        {
            return Err(Renderer3dError::CapacityExceeded);
        }
        let slot_index = if let Some(index) = self.free.pop() {
            index
        } else {
            let index =
                u32::try_from(self.slots.len()).map_err(|_| Renderer3dError::IntegerOverflow)?;
            self.slots
                .try_reserve(1)
                .map_err(|_| Renderer3dError::AllocationFailed)?;
            self.slots.push(MaterialSlot {
                generation: INITIAL_GENERATION,
                record: None,
            });
            index
        };
        let slot = self
            .slots
            .get_mut(slot_index as usize)
            .ok_or(Renderer3dError::InvalidHandle)?;
        slot.record = Some(record);
        Ok(MaterialHandle::new(slot_index, slot.generation))
    }

    pub fn get(&self, handle: MaterialHandle) -> Result<MaterialRecord, Renderer3dError> {
        if !handle.is_valid() {
            return Err(Renderer3dError::InvalidHandle);
        }
        let slot = self
            .slots
            .get(handle.index as usize)
            .ok_or(Renderer3dError::InvalidHandle)?;
        if slot.generation != handle.generation {
            return Err(Renderer3dError::InvalidHandle);
        }
        slot.record.ok_or(Renderer3dError::InvalidHandle)
    }

    pub fn remove(&mut self, handle: MaterialHandle) -> Result<MaterialRecord, Renderer3dError> {
        if !handle.is_valid() {
            return Err(Renderer3dError::InvalidHandle);
        }
        let index = usize::try_from(handle.index).map_err(|_| Renderer3dError::IntegerOverflow)?;
        let slot = self
            .slots
            .get(index)
            .ok_or(Renderer3dError::InvalidHandle)?;
        if slot.generation != handle.generation || slot.record.is_none() {
            return Err(Renderer3dError::InvalidHandle);
        }
        self.free
            .try_reserve(1)
            .map_err(|_| Renderer3dError::AllocationFailed)?;
        let record = {
            let slot = self
                .slots
                .get_mut(index)
                .ok_or(Renderer3dError::InvalidHandle)?;
            let record = slot.record.take().ok_or(Renderer3dError::InvalidHandle)?;
            slot.generation = next_generation(slot.generation);
            record
        };
        self.free.push(handle.index);
        Ok(record)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderView {
    pub view_projection: [f32; 16],
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub near_plane: f32,
    pub far_plane: f32,
}

impl RenderView {
    pub fn validate(self) -> Result<Self, Renderer3dError> {
        if self.viewport_width == 0
            || self.viewport_height == 0
            || !self.view_projection.iter().all(|value| value.is_finite())
            || !self.near_plane.is_finite()
            || !self.far_plane.is_finite()
            || self.near_plane <= 0.0
            || self.far_plane <= self.near_plane
        {
            return Err(Renderer3dError::InvalidView);
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldPassConfig {
    pub view: RenderView,
    pub clear_color: [f32; 4],
    pub clear_depth: f32,
}

impl WorldPassConfig {
    pub fn validate(self) -> Result<Self, Renderer3dError> {
        self.view.validate()?;
        if !self.clear_color.iter().all(|value| value.is_finite())
            || !self.clear_depth.is_finite()
            || !(0.0..=1.0).contains(&self.clear_depth)
        {
            return Err(Renderer3dError::InvalidPass);
        }
        Ok(self)
    }
}

fn validate_mesh_input(
    vertex_bytes: &[u8],
    vertex_stride: u32,
    index_bytes: &[u8],
    bounds: [f32; 4],
) -> Result<(), Renderer3dError> {
    if vertex_stride < 12
        || !vertex_stride.is_multiple_of(4)
        || vertex_bytes.is_empty()
        || !vertex_bytes.len().is_multiple_of(vertex_stride as usize)
    {
        return Err(Renderer3dError::InvalidVertexFormat);
    }
    if index_bytes.is_empty()
        || !index_bytes
            .len()
            .is_multiple_of(core::mem::size_of::<u32>())
    {
        return Err(Renderer3dError::InvalidIndexData);
    }
    if !bounds.iter().all(|value| value.is_finite()) || bounds[3] < 0.0 {
        return Err(Renderer3dError::InvalidBounds);
    }
    Ok(())
}

fn next_generation(generation: u32) -> u32 {
    generation
        .checked_add(1)
        .filter(|value| *value != 0)
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERTEX: [u8; 12] = [0; 12];
    const INDEX: [u8; 4] = [0; 4];

    #[test]
    fn mesh_ranges_are_bounded_and_handle_checked() {
        let mut table = MeshTable::with_capacity(1, 12, 4);
        let handle = table
            .try_insert(&VERTEX, 12, &INDEX, [0.0, 0.0, 0.0, 1.0])
            .unwrap();
        assert_eq!(table.get(handle).unwrap().vertex_count, 1);
        assert!(matches!(
            table.try_insert(&VERTEX, 12, &INDEX, [0.0, 0.0, 0.0, 1.0]),
            Err(Renderer3dError::CapacityExceeded)
        ));
        table.remove(handle).unwrap();
        assert!(matches!(
            table.get(handle),
            Err(Renderer3dError::InvalidHandle)
        ));
    }

    #[test]
    fn invalid_mesh_data_never_enters_the_table() {
        let mut table = MeshTable::with_capacity(2, 64, 64);
        assert!(matches!(
            table.try_insert(&VERTEX[..11], 12, &INDEX, [0.0, 0.0, 0.0, 1.0]),
            Err(Renderer3dError::InvalidVertexFormat)
        ));
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn capacity_rejection_keeps_mesh_state_unchanged() {
        let mut table = MeshTable::with_capacity(1, VERTEX.len(), INDEX.len());
        assert!(matches!(
            table.try_insert(&VERTEX, 12, &[0; 8], [0.0, 0.0, 0.0, 1.0]),
            Err(Renderer3dError::CapacityExceeded)
        ));
        assert_eq!(table.len(), 0);
        assert_eq!(table.vertex_bytes_len(), 0);
        assert_eq!(table.index_bytes_len(), 0);
    }

    #[test]
    fn material_generation_rejects_stale_handles() {
        let mut table = MaterialTable::with_capacity(1);
        let first = table
            .try_insert(MaterialRecord {
                base_color: [1.0, 0.0, 0.0, 1.0],
                metallic: 0.0,
                roughness: 0.5,
            })
            .unwrap();
        table.remove(first).unwrap();
        let second = table
            .try_insert(MaterialRecord {
                base_color: [0.0, 1.0, 0.0, 1.0],
                metallic: 0.0,
                roughness: 0.5,
            })
            .unwrap();
        assert_ne!(first.generation(), second.generation());
        assert!(matches!(
            table.get(first),
            Err(Renderer3dError::InvalidHandle)
        ));
    }

    #[test]
    fn view_and_pass_validate_finite_bounds() {
        let view = RenderView {
            view_projection: [1.0; 16],
            viewport_width: 1280,
            viewport_height: 720,
            near_plane: 0.1,
            far_plane: 100.0,
        };
        assert!(WorldPassConfig {
            view,
            clear_color: [0.02, 0.03, 0.05, 1.0],
            clear_depth: 1.0,
        }
        .validate()
        .is_ok());
        assert!(matches!(
            RenderView {
                viewport_width: 0,
                ..view
            }
            .validate(),
            Err(Renderer3dError::InvalidView)
        ));
    }
}
