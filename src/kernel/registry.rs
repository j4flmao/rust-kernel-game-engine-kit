//! The registry: name-keyed set of registered subsystems.
//!
//! The index of an entry doubles as the subsystem's [`SubscriberId`] on the
//! message bus: registration order fixes the id space for a kernel run.

use crate::kernel::error::KernelError;
use crate::kernel::Subsystem;

pub(crate) type Registered = (String, Box<dyn Subsystem>);

/// Owns the driver list. Kernel-core owned; subsystem authors never touch it.
#[derive(Default)]
pub(crate) struct Registry {
    pub(crate) entries: Vec<Registered>,
}

impl Registry {
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Registers a subsystem, detecting duplicate names at startup.
    pub(crate) fn register(&mut self, subsystem: Box<dyn Subsystem>) -> Result<usize, KernelError> {
        let name = subsystem.name().to_owned();
        if self.entries.iter().any(|(n, _)| *n == name) {
            return Err(KernelError::duplicate_name(name));
        }
        self.entries.push((name, subsystem));
        Ok(self.entries.len() - 1)
    }

    /// Resolves names to indices, failing hard on unknown dependencies.
    pub(crate) fn resolve_dependencies(&self) -> Result<Vec<Vec<usize>>, KernelError> {
        self.entries
            .iter()
            .map(|(name, subsystem)| {
                subsystem
                    .dependencies()
                    .iter()
                    .map(|dep| {
                        self.entries
                            .iter()
                            .position(|(n, _)| n == dep)
                            .ok_or_else(|| KernelError::unknown_dependency(name, dep))
                    })
                    .collect()
            })
            .collect()
    }
}
