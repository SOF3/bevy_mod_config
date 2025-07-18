// Contains implementations of `ConfigField` for various scalar types.
//! Exports the [metadata](crate::ConfigField::Metadata) structs for foreign scalar types.

use alloc::string::String;
use core::time::Duration;

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
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, Duration,
);

/// Metadata for numeric scalar config fields.
#[derive(Clone)]
pub struct NumericMetadata<T> {
    /// The default value.
    pub default:   T,
    /// The minimum possible value.
    pub min:       T,
    /// The maximum possible value.
    pub max:       T,
    /// The precision of the value.
    pub precision: Option<T>,
    /// Whether to display the value as a slider in the UI.
    pub slider:    bool,
}

impl<T: Numeric> Default for NumericMetadata<T> {
    fn default() -> Self {
        Self {
            default:   T::ZERO,
            min:       T::MIN,
            max:       T::MAX,
            precision: Some(T::ONE),
            slider:    false,
        }
    }
}

trait Numeric: Sized {
    const MIN: Self;
    const MAX: Self;
    const ZERO: Self;
    const ONE: Self;
}

macro_rules! impl_int {
    ($($ty:ty),*) => {
        $(
            impl Numeric for $ty {
                const MIN: Self = Self::MIN;
                const MAX: Self = Self::MAX;
                const ZERO: Self = 0;
                const ONE: Self = 1;
            }
        )*
    };
}

impl_int!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

impl Numeric for f32 {
    const MIN: Self = f32::MIN;
    const MAX: Self = f32::MAX;
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
}

impl Numeric for f64 {
    const MIN: Self = f64::MIN;
    const MAX: Self = f64::MAX;
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
}

impl Numeric for Duration {
    const MIN: Self = Duration::ZERO;
    const MAX: Self = Duration::MAX;
    const ZERO: Self = Duration::ZERO;
    const ONE: Self = Duration::from_secs(1);
}

impl_scalar_config_field!(
    String,
    StringMetadata,
    |metadata: &StringMetadata| metadata.default.into(),
    'a => &'a str,
    String::as_str,
);

/// Metadata for [`String`] fields.
#[derive(Default, Clone)]
pub struct StringMetadata {
    /// The default value.
    pub default:    &'static str,
    /// The maximum length of the string.
    pub max_length: Option<usize>,
    /// Whether the string can span multiple lines.
    ///
    /// This affects the UI representation of the field,
    /// allowing it to be rendered as a multiline text input.
    pub multiline:  bool,
}

impl_scalar_config_field!(
    bool,
    BoolMetadata,
    |metadata: &BoolMetadata| metadata.default,
    'a => bool,
    |&b: &bool| b,
);

/// Metadata for [`bool`] fields.
#[derive(Default, Clone)]
pub struct BoolMetadata {
    /// The default value.
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

/// Metadata for [`bevy_color::Color`] fields.
#[cfg(feature = "bevy_color")]
#[derive(Default, Clone)]
pub struct ColorMetadata {
    /// The default value.
    pub default:        bevy_color::Color,
    /// Show blend options for alpha.
    pub alpha_blend:    bool,
    /// Show additive alpha blending option.
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

/// Dummy metadata type for [`BareField`].
#[derive(Default, Clone)]
pub struct BareMetadata {}
