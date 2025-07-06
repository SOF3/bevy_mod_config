use alloc::string::String;

use super::impl_scalar_config_field_ as impl_scalar_config_field;

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

#[derive(Default)]
pub struct NumericMetadata<T> {
    pub min:     Option<T>,
    pub max:     Option<T>,
    pub default: T,
}

impl_scalar_config_field!(
    String,
    StringMetadata,
    |metadata: &StringMetadata| metadata.default.clone(),
    'a => &'a str,
    String::as_str,
);

#[derive(Default)]
pub struct StringMetadata {
    pub default:    String,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
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
#[derive(Default)]
pub struct ColorMetadata {
    pub default: bevy_color::Color,
}
