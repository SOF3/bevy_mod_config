use alloc::string::String;

use bevy_ecs::entity::Entity;

use super::impl_scalar_config_field_ as impl_scalar_config_field;
use crate::{ConfigField, ConfigNode, FieldGeneration, QueryLike, ScalarData};

macro_rules! impl_numeric_config_field {
    ($($ty:ty,)*) => {
        $(
            impl_scalar_config_field!(
                $ty,
                NumericMetadata<$ty>,
                |metadata: &NumericMetadata<$ty>| metadata.default,
                'a => $ty,
                |&value: &$ty| value,
            );
        )*
    };
}

impl_numeric_config_field!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64,
);

#[derive(Default, Clone)]
pub struct NumericMetadata<T> {
    pub min:     Option<T>,
    pub max:     Option<T>,
    pub default: T,
}

impl_scalar_config_field!(
    String,
    StringMetadata,
    |metadata: &StringMetadata| metadata.default.into(),
    'a => &'a str,
    String::as_str,
);

#[derive(Default, Clone)]
pub struct StringMetadata {
    pub default:    &'static str,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub multiline:  bool,
}

impl_scalar_config_field!(
    bool,
    BoolMetadata,
    |metadata: &BoolMetadata| metadata.default,
    'a => bool,
    |&b: &bool| b,
);

#[derive(Default, Clone)]
pub struct BoolMetadata {
    pub default: bool,
}

#[cfg(feature = "bevy_color")]
impl_scalar_config_field!(
    bevy_color::Color,
    ColorMetadata,
    |metadata: &ColorMetadata| metadata.default,
    'a => bevy_color::Color,
    |&value: &bevy_color::Color| value,
);

#[cfg(feature = "bevy_color")]
#[derive(Default, Clone)]
pub struct ColorMetadata {
    pub default:        bevy_color::Color,
    pub alpha_blend:    bool,
    pub alpha_additive: bool,
}

/// A [`ConfigField`] wrapper implementation with no metadata.
///
/// Used to implement on foreign types that do not implement [`ConfigField`] directly.
pub struct BareField<T>(pub T);

impl<T> ConfigField for BareField<T>
where
    T: Clone + Send + Sync + 'static,
{
    type SpawnHandle = Entity;
    type Reader<'a> = &'a T;
    type ReadQueryData = Option<&'static ScalarData<Self>>;
    type Metadata = BareMetadata;
    type Changed = FieldGeneration;
    type ChangedQueryData = ();

    fn read_world<'a>(
        query: impl QueryLike<Item = Option<&'a ScalarData<Self>>>,
        &spawn_handle: &Entity,
    ) -> Self::Reader<'a> {
        let data = query.get(spawn_handle).expect(
            "entity managed by config field must remain active as long as the config handle is \
             used",
        );
        &data.as_ref().expect("scalar data component must remain valid with Self type").0.0
    }

    fn changed<'a>(
        query: impl QueryLike<Item = (&'a ConfigNode, ())>,
        &spawn_handle: &Entity,
    ) -> Self::Changed {
        let entity = query.get(spawn_handle).expect(
            "entity managed by config field must remain active as long as the config handle is \
             used",
        );
        entity.0.generation
    }
}

#[derive(Default, Clone)]
pub struct BareMetadata {}
