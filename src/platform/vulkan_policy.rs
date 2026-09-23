//! Backend-independent Vulkan queue selection policy.

pub const QUEUE_GRAPHICS: u32 = 0x0000_0001;
pub const QUEUE_COMPUTE: u32 = 0x0000_0002;
pub const QUEUE_TRANSFER: u32 = 0x0000_0004;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueueFamilyInfo {
    pub index: u32,
    pub flags: u32,
    pub queue_count: u32,
}

impl QueueFamilyInfo {
    pub fn supports(&self, required: u32) -> bool {
        self.queue_count > 0 && self.flags & required == required
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueueSelection {
    pub graphics: u32,
    pub compute: u32,
    pub transfer: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueueCreateRequest {
    pub family_index: u32,
    pub queue_count: u32,
}

pub fn build_queue_create_requests(selection: QueueSelection) -> [QueueCreateRequest; 3] {
    let mut requests = [
        QueueCreateRequest {
            family_index: selection.graphics,
            queue_count: 1,
        },
        QueueCreateRequest {
            family_index: selection.compute,
            queue_count: 1,
        },
        QueueCreateRequest {
            family_index: selection.transfer,
            queue_count: 1,
        },
    ];
    for index in 1..requests.len() {
        if let Some(previous) = requests[..index]
            .iter()
            .position(|request| request.family_index == requests[index].family_index)
        {
            requests[previous].queue_count = requests[previous].queue_count.saturating_add(1);
            requests[index].queue_count = 0;
        }
    }
    requests
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueSelectionError;

impl core::fmt::Display for QueueSelectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Vulkan queue families do not satisfy graphics/compute/transfer requirements")
    }
}
impl core::error::Error for QueueSelectionError {}

fn first_preferred(
    families: &[QueueFamilyInfo],
    required: u32,
    avoid: u32,
) -> Option<&QueueFamilyInfo> {
    families
        .iter()
        .find(|family| family.supports(required) && family.flags & avoid == 0)
        .or_else(|| families.iter().find(|family| family.supports(required)))
}

pub fn select_queue_families(
    families: &[QueueFamilyInfo],
) -> Result<QueueSelection, QueueSelectionError> {
    let graphics = families
        .iter()
        .find(|family| family.supports(QUEUE_GRAPHICS));
    let compute = first_preferred(families, QUEUE_COMPUTE, QUEUE_GRAPHICS);
    let transfer = first_preferred(families, QUEUE_TRANSFER, QUEUE_GRAPHICS | QUEUE_COMPUTE);
    match (graphics, compute, transfer) {
        (Some(graphics), Some(compute), Some(transfer)) => Ok(QueueSelection {
            graphics: graphics.index,
            compute: compute.index,
            transfer: transfer.index,
        }),
        _ => Err(QueueSelectionError),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_first_deterministic_family_for_each_role() {
        let families = [
            QueueFamilyInfo {
                index: 0,
                flags: QUEUE_GRAPHICS | QUEUE_COMPUTE | QUEUE_TRANSFER,
                queue_count: 1,
            },
            QueueFamilyInfo {
                index: 1,
                flags: QUEUE_TRANSFER,
                queue_count: 1,
            },
        ];
        assert_eq!(
            select_queue_families(&families).unwrap(),
            QueueSelection {
                graphics: 0,
                compute: 0,
                transfer: 1
            }
        );
    }

    #[test]
    fn rejects_missing_required_family() {
        let families = [QueueFamilyInfo {
            index: 0,
            flags: QUEUE_GRAPHICS,
            queue_count: 1,
        }];
        assert_eq!(select_queue_families(&families), Err(QueueSelectionError));
    }

    #[test]
    fn prefers_dedicated_compute_and_transfer_families() {
        let families = [
            QueueFamilyInfo {
                index: 0,
                flags: QUEUE_GRAPHICS | QUEUE_COMPUTE | QUEUE_TRANSFER,
                queue_count: 1,
            },
            QueueFamilyInfo {
                index: 1,
                flags: QUEUE_COMPUTE,
                queue_count: 1,
            },
            QueueFamilyInfo {
                index: 2,
                flags: QUEUE_TRANSFER,
                queue_count: 1,
            },
        ];
        assert_eq!(
            select_queue_families(&families).unwrap(),
            QueueSelection {
                graphics: 0,
                compute: 1,
                transfer: 2
            }
        );
    }

    #[test]
    fn queue_create_requests_deduplicate_shared_families() {
        let requests = build_queue_create_requests(QueueSelection {
            graphics: 2,
            compute: 2,
            transfer: 7,
        });
        assert_eq!(
            requests[0],
            QueueCreateRequest {
                family_index: 2,
                queue_count: 2
            }
        );
        assert_eq!(requests[1].queue_count, 0);
        assert_eq!(
            requests[2],
            QueueCreateRequest {
                family_index: 7,
                queue_count: 1
            }
        );
    }
}
