//! Kernel-level error types. Hand-written — no external error crates.

use core::fmt;

use crate::kernel::bus::BusError;

/// Startup errors. An unmet or cyclic dependency is a hard startup failure,
/// never a silent skip.
#[derive(Debug, PartialEq, Eq)]
pub enum KernelError {
    /// A subsystem referred to a dependency that was never registered.
    UnknownDependency {
        subsystem: String,
        dependency: String,
    },
    /// The dependency graph contains a cycle; simulation order is undefined.
    DependencyCycle(Vec<String>),
    /// Two subsystems registered under the same name.
    DuplicateName(String),
    /// Message bus could not be brought up (e.g. routing setup failed).
    Bus(BusError),
    /// Runtime action attempted before `Kernel::init` succeeded.
    NotInitialized,
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownDependency {
                subsystem,
                dependency,
            } => write!(
                f,
                "subsystem '{subsystem}' depends on '{dependency}' which is not registered"
            ),
            Self::DependencyCycle(path) => {
                write!(f, "dependency cycle detected: {}", path.join(" -> "))
            }
            Self::DuplicateName(name) => write!(f, "duplicate subsystem name '{name}'"),
            Self::NotInitialized => write!(f, "kernel used before successful init"),
            Self::Bus(err) => write!(f, "message bus error: {err}"),
        }
    }
}

impl core::error::Error for KernelError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Bus(err) => Some(err),
            _ => None,
        }
    }
}

impl From<BusError> for KernelError {
    fn from(err: BusError) -> Self {
        Self::Bus(err)
    }
}

impl KernelError {
    /// Reports whether the failure is a topology/routing setup problem.
    pub fn is_configuration(&self) -> bool {
        !matches!(self, Self::Bus(_))
    }

    pub(crate) fn unknown_dependency(subsystem: &str, dependency: &str) -> Self {
        Self::UnknownDependency {
            subsystem: subsystem.to_owned(),
            dependency: dependency.to_owned(),
        }
    }

    pub(crate) fn cycle(path: Vec<String>) -> Self {
        Self::DependencyCycle(path)
    }

    pub(crate) fn duplicate_name(name: impl Into<String>) -> Self {
        Self::DuplicateName(name.into())
    }
}
