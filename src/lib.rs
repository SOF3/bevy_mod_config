#![no_std]
#![warn(missing_docs, clippy::pedantic)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::num::NonZeroU64;

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::query::QueryData;
use bevy_ecs::world::{EntityRef, EntityWorldMut, World};

mod impls;
pub use impls::BareField;
mod query;
pub use query::QueryLike;
mod enum_;
pub use enum_::{EnumDiscriminant, EnumDiscriminantMetadata, EnumDiscriminantWrapper};
pub mod manager;
pub use bevy_mod_config_macros::Config;
pub use manager::Manager;

pub mod __import;

mod app;
pub use app::{AppExt, ReadConfig};

mod tree;
pub use tree::{
    ChildNodeList, ChildNodeOf, ConditionalRelevance, ConfigNode, RootNode, ScalarField,
};

/// Tracks the number of changes to a config field.
///
/// After each change, the new generation is greater than the previous one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldGeneration(NonZeroU64);

impl Default for FieldGeneration {
    fn default() -> Self { FieldGeneration(const { NonZeroU64::new(1).unwrap() }) }
}

impl FieldGeneration {
    /// Increments the generation by one.
    ///
    /// # Panics
    /// Panics if the generation overflows.
    #[must_use]
    pub fn next(self) -> Self {
        FieldGeneration(self.0.checked_add(1).expect("field generation overflow"))
    }
}

/// Context information of the config field from its referrers.
#[derive(Clone)]
pub struct SpawnContext {
    /// The hierarchical path from the root config field.
    ///
    /// Uniquely identifies the config field statically.
    pub path:       Vec<String>,
    /// The parent entity of the config field, if any.
    pub parent:     Option<Entity>,
    /// The [`ConditionalRelevance`] dependency of the config field, if any.
    pub dependency: Option<ConditionalRelevance>,
}

impl SpawnContext {
    /// Appends a path component to this context.
    #[must_use]
    pub fn join(
        &self,
        key: impl IntoIterator<Item = impl Into<String>>,
        parent: Option<Entity>,
    ) -> Self {
        SpawnContext {
            path: self
                .path
                .iter()
                .cloned()
                .chain(key.into_iter().map(Into::<String>::into))
                .collect(),
            parent,
            dependency: None,
        }
    }

    /// Adds a [`ConditionalRelevance`] dependency to this context.
    #[must_use]
    pub fn with_dependency(
        mut self,
        dependency: Entity,
        is_entity_relevant: fn(EntityRef) -> bool,
    ) -> Self {
        self.dependency = Some(ConditionalRelevance { dependency, is_entity_relevant });
        self
    }
}

/// The spawn handle of a config node.
pub trait SpawnHandle {
    /// The entity of the subtree root node.
    fn node(&self) -> Entity;
}

impl SpawnHandle for Entity {
    fn node(&self) -> Entity { *self }
}

/// Field types that can be used in a [`Config`] struct/enum.
pub trait ConfigField: 'static {
    /// Remembers where the config data are stored in the world after spawning.
    type SpawnHandle: SpawnHandle + 'static + Send + Sync;

    /// The type returned when reading the config data from the world.
    ///
    /// `'a` is the lifetime of the receiver in [`ReadConfig::read`].
    type Reader<'a>;
    /// The minimal components required to read the typed config fields under this field.
    ///
    /// For scalar fields, this is always `Option<&ScalarData<Self>>`.
    type ReadQueryData: QueryData;

    /// Type-specific metadata specified by the referrer.
    type Metadata: Default + 'static + Send + Sync;

    /// Type returned by [`ConfigField::changed`].
    ///
    /// The return type of this function is often opaque, but guarantees that:
    /// - It can be safely persisted in the world due to thread safety and static lifetime.
    /// - It can be [cloned](Clone) at a cheaper cost (than the original data, on average).
    /// - It can be compared for [equality](Eq) with the previous value
    ///   to determine whether the config data has changed.
    type Changed: Clone + Eq + 'static + Send + Sync;
    /// The minimal components required to compute whether the config data has changed.
    ///
    /// This is `()` for most types,
    /// but may contain enum discriminants for enum fields
    /// to determine which variant should be compared.
    type ChangedQueryData: QueryData;

    /// Reads config data for user consumption from a query of config data entities.
    fn read_world<'a>(
        query: impl QueryLike<
            Item = <<Self::ReadQueryData as QueryData>::ReadOnly as QueryData>::Item<'a>,
        >,
        spawn_handle: &Self::SpawnHandle,
    ) -> Self::Reader<'a>;

    /// Computes an [equivalence class](Eq) that represents whether the config data has changed.
    ///
    /// If the config data has been changed, the result returned by this function
    /// will be [unequal](PartialEq::ne) to the result obtained before the change.
    fn changed<'a>(
        query: impl QueryLike<
            Item = (
                &'a ConfigNode,
                <<Self::ChangedQueryData as QueryData>::ReadOnly as QueryData>::Item<'a>,
            ),
        >,
        spawn_handle: &Self::SpawnHandle,
    ) -> Self::Changed;
}

/// Determines how a [`ConfigField`] implementor interacts with a [`Manager`] type.
///
/// `T: ConfigField<M>` means that `T` can be used in applications
/// using a [`Manager`] `M`.
/// If `T` contains a scalar type `U`, the implementation should be written as
///
/// ```text
/// impl<M: manager::Supports<U>> ConfigField<M> for T { ... }
/// ```
pub trait ConfigFieldFor<M>: ConfigField {
    /// Spawns entities in the world to store config data.
    ///
    /// Each spawned entity MUST have a [`ConfigNode`] component
    /// AND attach the component bundle requested from [`Manager::new_entity`].
    fn spawn_world(
        world: &mut World,
        ctx: SpawnContext,
        metadata: Self::Metadata,
    ) -> Self::SpawnHandle;
}

/// Stores the typed data of a scalar config field.
///
/// In addition to direct use in [`ConfigField`] implementations,
/// this is also the conventional type used by [`Manager`]s to interact with the actual data
/// which they are monomorphized for in [`manager::Supports::new_entity_for_type`].
/// Managers generally only interact with scalar fields directly.
#[derive(Component)]
pub struct ScalarData<T>(pub T);

/// Stores the metadata of a scalar config field.
#[derive(Component)]
pub struct ScalarMetadata<T: ConfigField>(pub T::Metadata);

/// Implements [`ConfigField`] for a scalar (non-composite) type.
///
/// - `$ty`: the scalar type to implement [`ConfigField`] for.
///   This is the actual owned value to be persisted in the world.
///   Managers will see this type as a component [`ScalarData<$ty>`].
/// - `$metadata`: the metadata type for the scalar field.
/// - `$default_from_metadata`: a function to produce a default value of `$ty` from metadata.
///   Must implement `Fn($metadata) -> $ty`.
/// - `$lt`: an arbitrary lifetime parameter that may be used in `$mapped_ty`.
///   Just put an arbitrary lifetime parameter here, such as `'a`,
///   even if `$mapped_ty` does not use it.
/// - `$mapped_ty`: the type returned by [`ConfigField::read_world`].
///   This is the most user-friendly type used in readers,
///   e.g. `&str` for `String`, or the owned value for [`Copy`] types.
/// - `$map_fn`: a function that maps the scalar data to `$mapped_ty`.
#[macro_export]
macro_rules! impl_scalar_config_field {
    ($ty:ty, $metadata:ty, $default_from_metadata:expr, $lt:lifetime => $mapped_ty:ty, $map_fn:expr $(,)?) => {
        impl $crate::ConfigField for $ty {
            type SpawnHandle = $crate::__import::Entity;
            type Reader<$lt> = $mapped_ty;
            type ReadQueryData = $crate::__import::Option<&'static $crate::ScalarData<Self>>;
            type Metadata = $metadata;
            type Changed = $crate::FieldGeneration;
            type ChangedQueryData = ();

            fn read_world<'a>(
                query: impl $crate::QueryLike<Item = <<Self::ReadQueryData as $crate::__import::QueryData>::ReadOnly as $crate::__import::QueryData>::Item<'a>>,
                &spawn_handle: &$crate::__import::Entity,
            ) -> Self::Reader<'a> {
                let data = query.get(spawn_handle).expect(
                    "entity managed by config field must remain active as long as the config \
                     handle is used",
                );
                $map_fn(&data.as_ref().expect("scalar data component must remain valid with Self type").0)
            }

            fn changed<'a>(
                query: impl $crate::QueryLike<Item = (&'a $crate::ConfigNode, <<Self::ChangedQueryData as $crate::__import::QueryData>::ReadOnly as $crate::__import::QueryData>::Item<'a>)>,
                &spawn_handle: &$crate::__import::Entity,
            ) -> Self::Changed {
                let entity = query.get(spawn_handle).expect(
                    "entity managed by config field must remain active as long as the config \
                     handle is used",
                );
                entity.0.generation
            }
        }

        impl<M: $crate::manager::Supports<$ty>> $crate::ConfigFieldFor<M> for $ty {
            fn spawn_world(
                world: &mut $crate::__import::World,
                ctx: $crate::SpawnContext,
                metadata: Self::Metadata,
            ) -> $crate::__import::Entity {
                let manager_comps =
                    world.resource_mut::<$crate::manager::Instance<M>>().new_entity::<$ty>();
                let mut entity = world.spawn((
                        $crate::ScalarData::<Self>($default_from_metadata(&metadata)),
                        $crate::ScalarMetadata::<Self>(metadata),
                        manager_comps,
                ));
                $crate::init_config_node(&mut entity, ctx);
                entity.id()
            }
        }
    };
}
use impl_scalar_config_field as impl_scalar_config_field_;

/// Initializes a newly spawned config node entity with the required components from the context.
pub fn init_config_node(entity: &mut EntityWorldMut, ctx: SpawnContext) {
    entity.insert(ConfigNode { path: ctx.path, generation: FieldGeneration::default() });
    if let Some(parent) = ctx.parent {
        entity.insert(ChildNodeOf(parent));
    }
    if let Some(dependency) = ctx.dependency {
        entity.insert(dependency);
    }
}

/// Metadata type for [`ConfigField`] implementors derived from [`Config`].
#[derive(Default, Clone)]
pub struct StructMetadata;
