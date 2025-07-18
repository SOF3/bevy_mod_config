//! Utilities for config management.
//!
//! See [`Manager`] for more information.

use core::ops::{Deref, DerefMut};

use bevy_ecs::bundle::Bundle;
use bevy_ecs::resource::Resource;

#[cfg(feature = "egui")]
pub mod egui;
#[cfg(feature = "egui")]
pub use egui::Egui;

#[cfg(feature = "serde")]
pub mod serde;
#[cfg(feature = "serde")]
pub use serde::Serde;

/// Stateful hooks attached to config fields.
///
/// A manager is invoked when a scalar config field is spawned in the world,
/// allowing it to attach behavior to the entity.
///
/// Tuples of managers are also managers;
/// each manager would be invoked in order when a new field entity is spawned.
/// This allows using multiple managers in the same app.
pub trait Manager: Sized + Send + Sync + 'static {
    /// Returns a component bundle that tracks entity management.
    ///
    /// This is particulraly useful for attaching vtable pointers to a component
    /// so that the manager can later traverse the config tree
    /// without knowing the type of each field at compile time.
    fn new_entity<T>(&mut self) -> impl Bundle
    where
        Self: Supports<T>,
    {
        self.new_entity_for_type()
    }
}

/// Marks that a [`Manager`] type supports handling config fields of scalar type `T`.
pub trait Supports<T>: Manager {
    /// Returns a component bundle that tracks entity management for the scalar type `T`.
    fn new_entity_for_type(&mut self) -> impl Bundle;
}

/// Stores the manager instances from the world.
///
/// `M` must be the exact manager type passed into [`init_config`](crate::AppExt::init_config).
#[derive(Resource)]
pub struct Instance<M: Manager> {
    /// The manager instance.
    pub instance: M,
}

impl<M: Manager> Deref for Instance<M> {
    type Target = M;

    fn deref(&self) -> &M { &self.instance }
}

impl<M: Manager> DerefMut for Instance<M> {
    fn deref_mut(&mut self) -> &mut M { &mut self.instance }
}

macro_rules! impl_manager {
    ($(($n:tt, $M:ident)),*) => {
        impl<$($M),*> Manager for ($($M,)*)
        where
            $($M: Manager),*
        {}

        impl<T, $($M: Send + Sync + 'static),*> Supports<T> for ($($M,)*)
        where
            $($M: Supports<T>),*
        {
            fn new_entity_for_type(&mut self) -> impl Bundle {
                #[allow(clippy::unused_unit)]
                (
                    $(
                        self.$n.new_entity_for_type(),
                    )*
                )
            }
        }
    };
}

variadics_please::all_tuples_enumerated!(impl_manager, 0, 8, T);
