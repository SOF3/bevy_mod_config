#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::iter;
use core::num::NonZeroU64;

use bevy_ecs::component::Component;
use bevy_ecs::query::With;
use bevy_ecs::system::Query;
use bevy_ecs::world::{EntityRef, World};

mod impls;

pub mod manager;
pub use bevy_mod_config_macros::Config;
pub use manager::Manager;

pub mod __import;

mod app;
pub use app::{AppExt, ReadConfig};

/// Marks an entity as a scalar config field.
#[derive(Component)]
pub struct ConfigData {
    /// Context information passed to [`ConfigField::spawn_world`].
    pub ctx: SpawnContext,
    /// The generation of a field, used for change detection.
    pub generation: FieldGeneration,
}

/// Tracks the number of changes to a config field.
///
/// After each change, the new generation is greater than the previous one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldGeneration(NonZeroU64);

impl Default for FieldGeneration {
    fn default() -> Self {
        FieldGeneration(const{NonZeroU64::new(1).unwrap()})
    }
}

impl FieldGeneration {
    /// Increments the generation by one.
    pub fn next(self) -> Self {
        FieldGeneration(self.0.checked_add(1).expect("field generation overflow"))
    }
}

/// Context information of the config field from its referrers.
pub struct SpawnContext {
    /// The hierarchical path from the root config field.
    ///
    /// Uniquely identifies the config field statically.
    pub path: Vec<String>,
}

impl SpawnContext {
    /// Appends a path component to this context.
    #[must_use]
    pub fn join(&self, key: impl Into<String>) -> Self {
        SpawnContext { path: self.path.iter().cloned().chain(iter::once(key.into())).collect() }
    }
}

/// Field types that can be used in a [`Config`] struct/enum.
pub trait ConfigField: 'static {
    /// Remembers where the config data are stored in the world after spawning.
    type SpawnHandle: 'static + Send + Sync;

    /// The type returned when reading the config data from the world.
    type Reader<'a>;
    /// Type-specific metadata specified by the referrer.
    type Metadata: Default + 'static + Send + Sync;

    /// Reads config data for user consumption from a query of config data entities.
    fn read_world<'a>(
        query: &'a Query<EntityRef, With<ConfigData>>,
        spawn_handle: &Self::SpawnHandle,
    ) -> Self::Reader<'a>;
}

/// Determines how a [`ConfigField`] implementor interacts with a [`Manager`] type.
///
/// `T: ConfigField<M>` means that `T` can be used in applications
/// using a [`Manager`] `M`.
/// If `T` contains a scalar type `U`, the implementation should be written as
/// ```
/// # /*
/// impl<M: manager::Supports<U>> ConfigField<M> for T { ... }
/// # */
/// `
pub trait ConfigFieldFor<M>: ConfigField {
    /// Spawns entities in the world to store config data.
    ///
    /// Each spawned entity MUST have a [`ConfigData`] component
    /// AND attach the component bundle requested from [`Manager::new_entity`].
    /// The manager
    fn spawn_world(
        world: &mut World,
        ctx: SpawnContext,
        metadata: &Self::Metadata,
    ) -> Self::SpawnHandle;
}

/// Stores the typed data of a scalar config field.
///
/// In addition to direct use in [`ConfigField`] implementations,
/// this is also the conventional type used by [`Manager`]s to interact with the actual data
/// which they are monomorphized for in [`manager::Supports::new_entity_for_type`].
#[derive(Component)]
pub struct ScalarData<T>(pub T);

/// Implements [`ConfigField`] for a scalar (non-composite) type.
#[macro_export]
macro_rules! impl_scalar_config_field {
    ($ty:ty, $metadata:ty, $default_from_metadata:expr, $lt:lifetime => $mapped_ty:ty, $map_fn:expr $(,)?) => {
        impl $crate::ConfigField for $ty {
            type SpawnHandle = $crate::__import::Entity;
            type Reader<$lt> = $mapped_ty;
            type Metadata = $metadata;

            fn read_world<'a>(
                query: &'a $crate::__import::Query<
                    $crate::__import::EntityRef,
                    $crate::__import::With<$crate::ConfigData>,
                >,
                &spawn_handle: &$crate::__import::Entity,
            ) -> Self::Reader<'a> {
                let entity = query.get(spawn_handle).expect(
                    "entity managed by config field must remain active as long as the config \
                     handle is used",
                );
                let data = entity.get::<$crate::ScalarData<$ty>>().expect(
                    "entity must have been spawned with a ScalarData of the corresponding type",
                );
                $map_fn(&data.0)
            }
        }

        impl<M: $crate::manager::Supports<$ty>> $crate::ConfigFieldFor<M> for $ty {
            fn spawn_world(
                world: &mut $crate::__import::World,
                ctx: $crate::SpawnContext,
                metadata: &Self::Metadata,
            ) -> $crate::__import::Entity {
                let manager_comps =
                    world.resource_mut::<$crate::manager::Instance<M>>().new_entity::<$ty>();
                world
                    .spawn((
                        $crate::ConfigData { ctx, generation: $crate::__import::Default::default() },
                        $crate::ScalarData($default_from_metadata(metadata)),
                        manager_comps,
                    ))
                    .id()
            }
        }
    };
}
use impl_scalar_config_field as impl_scalar_config_field_;

/// Metadata type for [`ConfigField`] implementors derived from [`Config`].
#[derive(Default)]
pub struct StructMetadata;

/// Implemented by the discriminant type generated by [`Config`] when derived for enums.
///
/// [Manager]s that support enum config fields should blanket-implement
/// [`manager::Supports<T>`](manager::Supports) for all `T: EnumDiscriminant`.
pub trait EnumDiscriminant: ConfigField + Sized + Copy + 'static {
    /// Lists all variants of the enum.
    const VARIANTS: &'static [Self];

    /// Returns the index of the variant in [`variants`](Self::variants).
    fn into_usize(self) -> usize;

    /// Returns the enum variant name.
    fn name(self) -> &'static str;

    /// Returns the enum variant with the given name if any.
    fn from_name(name: &str) -> Option<Self>;
}
