//! Stable, generation-checked identifiers used by the retained UI world.

use core::fmt;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name {
            index: u32,
            generation: u32,
        }

        impl $name {
            pub const fn new(index: u32, generation: u32) -> Self {
                Self { index, generation }
            }
            pub const fn index(self) -> u32 {
                self.index
            }
            pub const fn generation(self) -> u32 {
                self.generation
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}:{}", self.index, self.generation)
            }
        }
    };
}

id_type!(UiNodeId);
id_type!(StyleId);
id_type!(TextureId);
id_type!(FontId);
