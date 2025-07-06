use alloc::string::String;
use core::any::{TypeId, type_name};

use bevy_app::App;
use bevy_ecs::query::With;
use bevy_ecs::resource::Resource;
use bevy_ecs::system::{Query, Res, SystemParam};
use hashbrown::HashSet;

use crate::{ConfigData, ConfigField, ConfigFieldFor, Manager, SpawnContext, manager};

/// Extension trait for [App] to initialize config systems.
pub trait AppExt {
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
    fn init_config<M: Manager, C: ConfigFieldFor<M>>(
        &mut self,
        key: impl Into<String>,
    ) -> &mut Self
    where
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
    fn init_config<M: Manager, C: ConfigFieldFor<M>>(&mut self, key: impl Into<String>) -> &mut Self
    where
        C::Metadata: Default,
    {
        if let Some(&ManagerType { id, name, .. }) = self.world().get_resource() {
            if id != TypeId::of::<M>() {
                panic!(
                    "Use of multiple different config managers in the same app is not allowed: \
                     {name} vs {}",
                    type_name::<M>()
                );
            }
        } else {
            self.insert_resource(ManagerType {
                id:        TypeId::of::<M>(),
                name:      type_name::<M>(),
                root_keys: HashSet::new(),
            });
            self.insert_resource(manager::Instance { instance: M::default() });
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

        let spawn_handle = C::spawn_world(
            self.world_mut(),
            SpawnContext { path: [key].into() },
            &Default::default(),
        );

        if self.world().get_resource::<RootField<C>>().is_some() {
            panic!(
                "Cannot initialize multiple root config fields of the same type in the same app: \
                 {}",
                type_name::<C>()
            );
        }

        self.insert_resource(RootField::<C> { spawn_handle });

        self
    }
}

#[derive(SystemParam)]
pub struct ReadConfig<'w, 's, C: ConfigField> {
    query:      Query<'w, 's, <C as ConfigField>::ReadQueryData, With<ConfigData>>,
    root_field: Res<'w, RootField<C>>,
}

impl<C: ConfigField> ReadConfig<'_, '_, C> {
    /// Reads the config field from the world.
    pub fn read(&self) -> C::Reader<'_> {
        C::read_world(&self.query, &self.root_field.spawn_handle)
    }
}
