//! Re-exported types referenced in macros.
#![doc(hidden)]

pub use core::clone::Clone;
pub use core::cmp::{Eq, PartialEq};
pub use core::convert::Into;
pub use core::default::Default;
pub use core::fmt::Debug;
pub use core::marker::{Copy, Send, Sync};
pub use core::option::Option::{self, None, Some};
pub use core::stringify;

pub use bevy_ecs::component::Component;
pub use bevy_ecs::entity::Entity;
pub use bevy_ecs::query::{QueryData, With};
pub use bevy_ecs::system::Query;
pub use bevy_ecs::world::{EntityRef, World};
