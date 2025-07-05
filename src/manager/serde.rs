use alloc::string::String;
use alloc::vec::Vec;
use core::any::TypeId;
use core::fmt;
use core::marker::PhantomData;

use bevy_ecs::bundle::Bundle;
use bevy_ecs::entity::Entity;
use bevy_ecs::query::With;
use bevy_ecs::world::{EntityRef, EntityWorldMut, World};
use hashbrown::HashMap;
use serde::de::{DeserializeOwned, MapAccess};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{ConfigData, EnumDiscriminant, EnumDiscriminantWrapper, Manager, ScalarData, manager};

pub trait Adapter: Send + Sync + 'static {
    type Typed: for<'a> TypedAdapter<
            SerContext<'a> = <Self::SerInput<'a> as Serializer>::SerializeMap,
            SerError<'a> = <Self::SerInput<'a> as Serializer>::Error,
        >;
    fn for_type<T: SerdeScalar>(&mut self) -> Self::Typed;

    type SerInput<'a>: Serializer;

    type DeInput<'de>: Deserializer<'de>;
    type DeKey<'de>: fmt::Debug + Deserialize<'de>;
    fn index_map_by_de_key<'de, 'map, V>(
        &self,
        map: &'map HashMap<Vec<String>, V>,
        key: Self::DeKey<'de>,
    ) -> Option<&'map V>;
}

pub trait TypedAdapter: Send + Sync + 'static {
    type SerContext<'a>: SerializeMap;
    type SerError<'a>;
    fn serialize_once<'a>(
        &self,
        entity: EntityRef,
        path: &[String],
        ser: &mut Self::SerContext<'a>,
    ) -> Result<(), Self::SerError<'a>>;

    fn deserialize_map_value<'de, M: MapAccess<'de>>(
        &self,
        entity: EntityWorldMut,
        map: &mut M,
    ) -> Result<(), M::Error>;
}

/// A [`Manager`] that serializes config data using Serde.
#[derive(Clone)]
pub struct Serde<A: Adapter> {
    adapter: A,
    types:   HashMap<TypeId, Typed<A::Typed>>,
}

type ScannedKey = (Vec<String>, Entity);

#[derive(Clone)]
struct Typed<A> {
    adapter:   A,
    scan_keys: fn(&mut World, &mut Vec<ScannedKey>),
}

impl<A: Adapter + Default> Default for Serde<A> {
    fn default() -> Self { Serde { adapter: A::default(), types: HashMap::new() } }
}

impl<A: Adapter> Serde<A> {
    fn keys_with_types(&self, world: &mut World) -> Vec<(ScannedKey, &Typed<A::Typed>)> {
        let mut keys_with_types = Vec::new();
        let types: Vec<_> = self.types.values().collect();

        let mut keys_buf = Vec::new();

        for typed in types {
            (typed.scan_keys)(world, &mut keys_buf);
            for key in keys_buf.drain(..) {
                keys_with_types.push((key, typed));
            }
        }

        keys_with_types
    }

    pub fn serialize_all<'a>(
        &self,
        world: &mut World,
        input: A::SerInput<'a>,
    ) -> Result<<A::SerInput<'a> as Serializer>::Ok, <A::SerInput<'a> as Serializer>::Error> {
        let mut keys = self.keys_with_types(world);
        keys.sort_by(|((path1, _), _), ((path2, _), _)| path1.cmp(path2));

        let mut map_ser = input.serialize_map(Some(keys.len()))?;
        for ((path, entity), typed) in keys {
            typed.adapter.serialize_once(world.entity(entity), &path, &mut map_ser)?;
        }
        map_ser.end()
    }

    pub fn deserialize<'de>(
        &self,
        world: &mut World,
        input: A::DeInput<'de>,
    ) -> Result<(), <A::DeInput<'de> as Deserializer<'de>>::Error> {
        let keys: HashMap<_, _> = self
            .keys_with_types(world)
            .into_iter()
            .map(|((path, entity), typed)| (path, (entity, typed)))
            .collect();

        let visitor = Visitor { adapter: &self.adapter, keys, world };
        input.deserialize_map(visitor)
    }
}

struct Visitor<'a, A: Adapter> {
    adapter: &'a A,
    keys:    HashMap<Vec<String>, (Entity, &'a Typed<A::Typed>)>,
    world:   &'a mut World,
}

impl<'de, A: Adapter> serde::de::Visitor<'de> for Visitor<'_, A> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> alloc::fmt::Result {
        formatter.write_str("a map")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        while let Some(key) = map.next_key::<A::DeKey<'de>>()? {
            if let Some(&(entity_id, typed)) = self.adapter.index_map_by_de_key(&self.keys, key) {
                let entity = self.world.entity_mut(entity_id);
                typed.adapter.deserialize_map_value(entity, &mut map)?;
            }
        }
        Ok(())
    }
}

impl<A: Adapter + Default> Manager for Serde<A> {}

impl<A, T> manager::Supports<T> for Serde<A>
where
    A: Adapter + Default,
    T: SerdeScalar,
{
    fn new_entity_for_type(&mut self) -> impl Bundle {
        self.types.entry(TypeId::of::<T>()).or_insert_with(|| Typed {
            adapter:   self.adapter.for_type::<T>(),
            scan_keys: |world, keys| {
                let mut query =
                    world.query_filtered::<(Entity, &ConfigData), With<ScalarData<T>>>();
                for (entity, config_data) in query.iter(world) {
                    keys.push((config_data.ctx.path.clone(), entity));
                }
            },
        });
    }
}

#[cfg(feature = "serde_json")]
mod json {
    extern crate std;
    use alloc::boxed::Box;
    use alloc::string::String;
    use alloc::vec::Vec;
    use core::any::Any;
    use std::io::{self, BufReader, BufWriter};

    use bevy_ecs::world::{EntityRef, EntityWorldMut, World};
    use hashbrown::HashMap;
    use serde::de::{Error as _, MapAccess};
    use serde::ser::SerializeMap as _;
    use serde_json::value::RawValue;

    use crate::ScalarData;

    pub type Json = super::Serde<JsonAdapter>;

    #[derive(Default, Clone)]
    pub struct JsonAdapter;

    pub trait AnyWrite: io::Write + Any {}
    impl<T: io::Write + Any> AnyWrite for T {}

    pub trait AnyRead: io::Read + Any {}
    impl<T: io::Read + Any> AnyRead for T {}

    type Writer = BufWriter<Box<dyn AnyWrite>>;
    type Reader = serde_json::de::IoRead<BufReader<Box<dyn AnyRead>>>;

    #[derive(Clone)]
    pub struct TypedVtable {
        #[expect(
            clippy::type_complexity,
            reason = "HRTBs will make it even more complex to extract out"
        )]
        ser: fn(
            EntityRef,
            &[String],
            &mut <&mut serde_json::Serializer<Writer> as serde::Serializer>::SerializeMap,
        ) -> serde_json::Result<()>,
        de:  fn(EntityWorldMut, &RawValue) -> Result<(), serde_json::Error>,
    }

    impl super::Adapter for JsonAdapter {
        type Typed = TypedVtable;
        fn for_type<T: super::SerdeScalar>(&mut self) -> Self::Typed {
            TypedVtable {
                ser: |entity, path, ser: &mut <&mut serde_json::Serializer<Writer> as serde::Serializer>::SerializeMap| {
                    let value = entity.get::<ScalarData<T>>().expect("type checked in serde query");
                    ser.serialize_entry(&path.join("."), value.0.as_serialize())
                },
                de: |mut entity, value| {
                    let value: T::Deserialize = serde_json::from_str(value.get()).map_err(serde_json::Error::custom)?;
                    let mut entry = entity.get_mut::<ScalarData::<T>>().expect("type checked in serde qurey");
                    entry.0.set_deserialized(value);
                    Ok(())
                },
            }
        }

        type SerInput<'a> = &'a mut serde_json::Serializer<Writer>;

        type DeInput<'de> = &'de mut serde_json::Deserializer<Reader>;
        type DeKey<'de> = String;
        fn index_map_by_de_key<'de, 'map, V>(
            &self,
            map: &'map HashMap<Vec<String>, V>,
            key: Self::DeKey<'de>,
        ) -> Option<&'map V> {
            let key: Vec<_> = key.split('.').map(String::from).collect();
            map.get(&key)
        }
    }

    impl super::TypedAdapter for TypedVtable {
        type SerContext<'a> =
            <&'a mut serde_json::Serializer<Writer> as serde::Serializer>::SerializeMap;
        type SerError<'a> = serde_json::Error;
        fn serialize_once<'a>(
            &self,
            entity: EntityRef,
            path: &[String],
            ser: &mut Self::SerContext<'a>,
        ) -> Result<(), Self::SerError<'a>> {
            (self.ser)(entity, path, ser)
        }

        fn deserialize_map_value<'de, M: MapAccess<'de>>(
            &self,
            entity: EntityWorldMut,
            map: &mut M,
        ) -> Result<(), M::Error> {
            // Deserialize the value into a consistent type instead of the generic `MapAccess`
            // so that it can be passed to the vtable without knowing `M` during startup.
            // This is a terrible hack, but it is necessary for type erasure.
            let value: Box<RawValue> = map.next_value()?;
            (self.de)(entity, &value).map_err(M::Error::custom)
        }
    }

    impl super::Serde<JsonAdapter> {
        /// Serialize all config data in the world to a JSON string.
        pub fn to_string(&self, world: &mut World) -> Result<String, serde_json::Error> {
            let bytes = self.to_writer(world, Vec::<u8>::new())?;
            String::from_utf8(bytes).map_err(<serde_json::Error as serde::ser::Error>::custom)
        }

        /// Serialize all config data in the world to a [writer](io::Write).
        pub fn to_writer<W: Any + io::Write>(
            &self,
            world: &mut World,
            writer: W,
        ) -> Result<W, serde_json::Error> {
            let writer: Writer = BufWriter::new(Box::new(writer) as Box<dyn AnyWrite>);
            let mut serializer = serde_json::ser::Serializer::new(writer);
            self.serialize_all(world, &mut serializer)?;
            let boxed = serializer.into_inner().into_inner().map_err(serde_json::Error::custom)?;
            Ok(*Box::<dyn Any>::downcast::<W>(boxed)
                .expect("Serializer should preserve the underlying type"))
        }

        /// Deserialize config data from a JSON string.
        ///
        /// There is no special implementation for UTF-8-validated inputs (e.g. `&str`),
        /// because supporting that would require an extra vtable entry for each type,
        /// which appears unnecessary for negligible overhead in foreseeable use cases.
        /// If you have found a use case where
        /// benchmarks show significant improvement from `&str` support,
        /// please open an issue.
        pub fn from_reader<R: Any + io::Read>(
            &self,
            world: &mut World,
            reader: R,
        ) -> Result<(), serde_json::Error> {
            let mut deserializer = serde_json::Deserializer::from_reader(BufReader::new(Box::new(
                reader,
            )
                as Box<dyn AnyRead>));
            self.deserialize(world, &mut deserializer)
        }
    }
}

#[cfg(feature = "serde_json")]
pub use json::Json;

pub trait SerdeScalar: Send + Sync + 'static {
    fn as_serialize(&self) -> &(impl Serialize + ?Sized);

    type Deserialize: DeserializeOwned;
    fn set_deserialized(&mut self, value: Self::Deserialize);
}

impl<T: Serialize + DeserializeOwned + Send + Sync + 'static> SerdeScalar for T {
    fn as_serialize(&self) -> &(impl Serialize + ?Sized) { self }

    type Deserialize = Self;
    fn set_deserialized(&mut self, value: Self::Deserialize) { *self = value; }
}

const _: () = {
    impl<T: EnumDiscriminant> SerdeScalar for EnumDiscriminantWrapper<T> {
        fn as_serialize(&self) -> &(impl Serialize + ?Sized) { self.0.name() }

        type Deserialize = DeserializeEnumDiscriminant<T>;
        fn set_deserialized(&mut self, value: Self::Deserialize) { self.0 = value.0; }
    }

    pub struct DeserializeEnumDiscriminant<T>(T);

    impl<'de, T> Deserialize<'de> for DeserializeEnumDiscriminant<T>
    where
        T: EnumDiscriminant,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct Visitor<T>(PhantomData<T>);

            impl<'de, T: EnumDiscriminant> serde::de::Visitor<'de> for Visitor<T> {
                type Value = T;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    write!(formatter, "a variant of `{}`", core::any::type_name::<T>())
                }

                fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    T::from_name(value)
                        .ok_or_else(|| E::custom(format_args!("unknown enum variant: {value}")))
                }
            }

            deserializer.deserialize_identifier(Visitor(PhantomData::<T>)).map(Self)
        }
    }
};
