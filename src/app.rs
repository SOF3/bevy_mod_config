use alloc::string::String;
use core::any::{TypeId, type_name};

use bevy_app::App;
use bevy_ecs::resource::Resource;
use bevy_ecs::system::{Local, Query, Res, SystemParam};
use hashbrown::HashSet;

use crate::{
    ConfigField, ConfigFieldFor, ConfigNode, Manager, RootNode, SpawnContext, SpawnHandle, manager,
};

/// Extension trait for [App] to initialize config systems.
pub trait AppExt {
    /// Initializes a root config type `C` in the app
    /// using the default manager constructor.
    ///
    /// See [`App::init_config_with`] for more information.
    fn init_config<M, C>(&mut self, key: impl Into<String>) -> &mut Self
    where
        M: Manager + Default,
        C: ConfigFieldFor<M>,
        C::Metadata: Default,
    {
        self.init_config_with::<M, C>(key, M::default)
    }

    /// Initializes a root config type `C` in the app.
    ///
    /// This method may be called multiple times on the same app
    /// to initialize different config types,
    /// with the following requirements that would lead to a panic:
    ///
    /// # Panics
    /// - `M` must be **the same** for all calls.
    /// - `C` must be **unique** for each call.
    /// - `key` must be **unique** for each call.
    ///
    /// To ensure the same manager type is used across your game,
    /// it is recommended to reuse a type alias for the desired manager type.
    fn init_config_with<M, C>(
        &mut self,
        key: impl Into<String>,
        init: impl FnOnce() -> M,
    ) -> &mut Self
    where
        M: Manager,
        C: ConfigFieldFor<M>,
        C::Metadata: Default;
}

#[derive(Resource)]
struct ManagerType {
    id:        TypeId,
    name:      &'static str,
    root_keys: HashSet<String>,
}

#[derive(Resource)]
struct RootField<C: ConfigField> {
    spawn_handle: C::SpawnHandle,
}

impl AppExt for App {
    fn init_config_with<M, C>(
        &mut self,
        key: impl Into<String>,
        init: impl FnOnce() -> M,
    ) -> &mut Self
    where
        M: Manager,
        C: ConfigFieldFor<M>,
        C::Metadata: Default,
    {
        if let Some(&ManagerType { id, name, .. }) = self.world().get_resource() {
            assert!(
                id == TypeId::of::<M>(),
                "Use of multiple different config managers in the same app is not allowed: {name} \
                 vs {}",
                type_name::<M>()
            );
        } else {
            self.insert_resource(ManagerType {
                id:        TypeId::of::<M>(),
                name:      type_name::<M>(),
                root_keys: HashSet::new(),
            });
            self.insert_resource(manager::Instance { instance: init() });
        }

        let key = key.into();
        let key_exists = self
            .world_mut()
            .get_resource_mut::<ManagerType>()
            .expect("just checked")
            .root_keys
            .replace(key.clone());
        if let Some(key) = key_exists {
            panic!("Cannot reuse config key {key:?} in the same app");
        }

        assert!(
            self.world().get_resource::<RootField<C>>().is_none(),
            "Cannot initialize multiple root config fields of the same type in the same app: {}",
            type_name::<C>()
        );

        let spawn_handle = C::spawn_world(
            self.world_mut(),
            SpawnContext { path: [key].into(), parent: None, dependency: None },
            Default::default(),
        );

        self.world_mut().entity_mut(spawn_handle.node()).insert(RootNode);
        self.insert_resource(RootField::<C> { spawn_handle });

        self
    }
}

/// Access to a tree of config fields from a root config type `C`
/// that was passed into [`App::init_config`].
#[derive(SystemParam)]
pub struct ReadConfig<'w, 's, C: ConfigField> {
    read_query:    Query<'w, 's, <C as ConfigField>::ReadQueryData>,
    changed_query: Query<'w, 's, (&'static ConfigNode, <C as ConfigField>::ChangedQueryData)>,
    root_field:    Res<'w, RootField<C>>,
}

impl<C: ConfigField> ReadConfig<'_, '_, C> {
    /// Reads the config field from the world.
    #[must_use]
    pub fn read(&self) -> C::Reader<'_> {
        C::read_world(&self.read_query, &self.root_field.spawn_handle)
    }

    /// Returns a value that changes when the config field is modified.
    ///
    /// See [`ConfigField::Changed`] for details.
    #[must_use]
    pub fn changed(&self) -> C::Changed {
        C::changed(&self.changed_query, &self.root_field.spawn_handle)
    }
}

/// Access to a tree of config fields from a root config type `C`,
/// and maintains a local state to track changes since the last check.
#[derive(SystemParam)]
pub struct ReadConfigChange<'w, 's, C: ConfigField> {
    last_value:  Local<'s, Option<<C as ConfigField>::Changed>>,
    read_config: ReadConfig<'w, 's, C>,
}

impl<C: ConfigField> ReadConfigChange<'_, '_, C> {
    /// Reads the config field from the world.
    #[must_use]
    pub fn read(&self) -> C::Reader<'_> { self.read_config.read() }

    /// Returns whether the config field has changed since the last check.
    pub fn consume_change(&mut self) -> bool {
        let changed = self.read_config.changed();
        if self.last_value.as_ref().is_none_or(|v| *v != changed) {
            *self.last_value = Some(changed);
            true
        } else {
            false
        }
    }
}
