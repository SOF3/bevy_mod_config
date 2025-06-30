use alloc::boxed::Box;
use core::any::TypeId;
use core::marker::PhantomData;

use bevy_ecs::bundle::Bundle;
use bevy_ecs::component::Component;
use bevy_ecs::world::World;
use hashbrown::HashMap;
use serde::de::DeserializeOwned;
use serde::ser::SerializeMap;
use serde::{Deserializer, Serialize, Serializer};

use crate::{ConfigData, Manager, ScalarData, manager};

/// A [`Manager`] that serializes config data using Serde.
pub struct Serde<S: Serializer, D>
where
    for<'de> D: Deserializer<'de>,
{
    types: HashMap<TypeId, Box<dyn AnyHandler<S, D>>>,
}

impl<S: Serializer, D> Default for Serde<S, D>
where
    for<'de> D: Deserializer<'de>,
{
    fn default() -> Self { Serde { types: HashMap::new() } }
}

struct Handler<T> {
    _ph: PhantomData<T>,
}

trait AnyHandler<S: Serializer, D>: Send + Sync
where
    for<'de> D: Deserializer<'de>,
{
    fn serialize_all(&self, world: &mut World, ser: S::SerializeMap) -> Result<(), S::Error>;
}

impl<T, S: Serializer, D> AnyHandler<S, D> for Handler<T>
where
    T: Serialize + DeserializeOwned + Send + Sync,
    ScalarData<T>: Component,
    for<'de> D: Deserializer<'de>,
{
    fn serialize_all(&self, world: &mut World, mut ser: S::SerializeMap) -> Result<(), S::Error> {
        let mut query = world.query::<(&ConfigData, &ScalarData<T>)>();
        for (config_data, data) in query.iter(world) {
            let path = &config_data.ctx.path;
            ser.serialize_entry(path, &data.0)?;
        }
        Ok(())
    }
}

impl<S: Serializer + 'static, D: 'static> Manager for Serde<S, D> where for<'de> D: Deserializer<'de>
{}

impl<S, D, T> manager::Supports<T> for Serde<S, D>
where
    S: Serializer + 'static,
    for<'de> D: Deserializer<'de> + 'static,
    T: Serialize + DeserializeOwned + Send + Sync,
    ScalarData<T>: Component,
{
    fn new_entity_for_type(&mut self) -> impl Bundle {
        self.types
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(Handler::<T> { _ph: PhantomData }));
    }
}
