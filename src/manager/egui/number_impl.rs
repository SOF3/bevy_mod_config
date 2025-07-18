use alloc::string::{String, ToString};
use core::hash::Hash;
use core::time::Duration;

use bevy_egui::egui;

use super::{DefaultStyle, Editable};
use crate::ConfigField;
use crate::impls::NumericMetadata;

/// A trait for types that can be displayed like numbers.
pub trait NumericLike: ConfigField + PartialOrd + Copy + Sized {
    /// Parses the value from a string.
    fn parse_from_str(s: &str) -> Option<Self>;

    /// Converts the value to a string.
    /// Should be roughly the inverse of [`parse_from_str`](NumericLike::parse_from_tsr).
    fn to_string(&self) -> String;

    /// Adds a `usize` to the value, saturating at the maximum value if overflow occurs.
    fn saturating_add_usize(self, i: usize) -> Self;

    /// Subtracts a `usize` from the value, saturating at the minimum value if underflow occurs.
    fn saturating_sub_usize(self, i: usize) -> Self;

    /// Whether the metadata requests the value to be displayed as a slider in the UI.
    fn metadata_wants_slider(metadata: &Self::Metadata) -> bool;

    /// Returns the lower bound specified by the metadata, if any.
    fn metadata_min(metadata: &Self::Metadata) -> Option<Self>;

    /// Returns the upper bound specified by the metadata, if any.
    fn metadata_max(metadata: &Self::Metadata) -> Option<Self>;

    /// Returns the slider precision specified by the metadata, if any.
    fn metadata_precision(metadata: &Self::Metadata) -> Option<f64>;

    /// Converts the value to a float for slider display.
    fn as_float(&self) -> f64;

    /// Converts a float from slider response back to the numeric type.
    fn from_float(f: f64) -> Self;
}

macro_rules! impl_primitive {
    (
        $ty:ty,
        saturating_add_usize: $self1:ident, $i1:ident => $saturating_add_usize:expr,
        saturating_sub_usize: $self2:ident, $i2:ident => $saturating_sub_usize:expr,
        $metadata:ident => $precision:expr,
        $float:ident => $from_float:expr,
    ) => {
        impl NumericLike for $ty {
            fn parse_from_str(s: &str) -> Option<Self> {
                s.parse::<Self>().ok()
            }

            fn to_string(&self) -> String {
                ToString::to_string(self)
            }

            fn saturating_add_usize($self1, $i1: usize) -> Self {
                $saturating_add_usize
            }

            fn saturating_sub_usize($self2, $i2: usize) -> Self {
                $saturating_sub_usize
            }

            fn metadata_wants_slider(metadata: &Self::Metadata) -> bool {
                metadata.slider
            }

            fn metadata_min(metadata: &Self::Metadata) -> Option<Self> {
                Some(metadata.min)
            }

            fn metadata_max(metadata: &Self::Metadata) -> Option<Self> {
                Some(metadata.max)
            }

            fn metadata_precision($metadata: &Self::Metadata) -> Option<f64> {
                $precision
            }

            fn as_float(&self) -> f64 {
                *self as f64
            }

            fn from_float($float: f64) -> Self {
                $from_float
            }
        }
    }
}

macro_rules! impl_number_signed {
    ($(($ty:ty, $unsigned:ty),)*) => {
        $(
            impl_primitive! {
                $ty,
                saturating_add_usize: self, i => {
                    self.saturating_add_unsigned(<$unsigned>::try_from(i).unwrap_or_else(|_| <$unsigned>::max_value()))
                },
                saturating_sub_usize: self, i => {
                    self.saturating_sub_unsigned(<$unsigned>::try_from(i).unwrap_or_else(|_| <$unsigned>::max_value()))
                },
                metadata => { metadata.precision.map(|n| n as f64) },
                float => { float.round() as $ty },
            }
        )*
    };
}

impl_number_signed! {
    (i8, u8),
    (i16, u16),
    (i32, u32),
    (i64, u64),
    (i128, u128),
    (isize, usize),
}

macro_rules! impl_number_unsigned {
    ($($ty:ty),*) => {
        $(
            impl_primitive! {
                $ty,
                saturating_add_usize: self, i => {
                    self.saturating_add(Self::try_from(i).unwrap_or_else(|_| Self::max_value()))
                },
                saturating_sub_usize: self, i => {
                    self.saturating_sub(Self::try_from(i).unwrap_or_else(|_| Self::max_value()))
                },
                metadata => { metadata.precision.map(|n| n as f64) },
                float => { float.round() as $ty },
            }
        )*
    };
}

impl_number_unsigned!(u8, u16, u32, u64, u128, usize);

impl_primitive! {
    f32,
    saturating_add_usize: self, i =>  self + i as f32 ,
    saturating_sub_usize: self, i =>  self - i as f32 ,
    metadata =>  metadata.precision.map(f64::from) ,
    float =>  float as f32 ,
}
impl_primitive! {
    f64,
    saturating_add_usize: self, i =>  self + i as f64 ,
    saturating_sub_usize: self, i =>  self - i as f64 ,
    metadata =>  metadata.precision ,
    float => float,
}

/// Implements the `NumericLike` trait for types that can be converted into a closed interval of
/// floats, parsed with an optional suffix.
pub trait FloatLikeWithSuffix: ConfigField + PartialOrd + Copy + Sized {
    /// Returns the suffix behind the string representation of the value.
    fn suffix() -> &'static str;
    /// Converts the value to a float.
    fn as_float(&self) -> f64;
    /// Converts the value from a float.
    fn from_float(f: f64) -> Self;
    /// Adds a `usize` to the value.
    fn add_usize(&self, i: usize) -> Self;
    /// Subtracts a `usize` from the value.
    fn sub_usize(&self, i: usize) -> Self;
    /// Converts the metadata to a [`NumericMetadata`] type.
    fn numeric_metadata(metadata: &Self::Metadata) -> NumericMetadata<Self>;
}

impl<T: FloatLikeWithSuffix> NumericLike for T {
    fn parse_from_str(s: &str) -> Option<Self> {
        let s = s.trim_end();
        let s = s.strip_suffix(T::suffix()).unwrap_or(s);
        let s = s.trim_end();
        s.parse::<f64>().ok().map(T::from_float)
    }
    fn to_string(&self) -> String { alloc::format!("{}{}", self.as_float(), T::suffix()) }

    fn saturating_add_usize(self, i: usize) -> Self { self.add_usize(i) }
    fn saturating_sub_usize(self, i: usize) -> Self { self.sub_usize(i) }

    fn metadata_wants_slider(metadata: &Self::Metadata) -> bool {
        T::numeric_metadata(metadata).slider
    }
    fn metadata_min(metadata: &Self::Metadata) -> Option<Self> {
        Some(T::numeric_metadata(metadata).min)
    }
    fn metadata_max(metadata: &Self::Metadata) -> Option<Self> {
        Some(T::numeric_metadata(metadata).max)
    }
    fn metadata_precision(metadata: &Self::Metadata) -> Option<f64> {
        T::numeric_metadata(metadata).precision.map(|v| v.as_float())
    }

    fn as_float(&self) -> f64 { <T as FloatLikeWithSuffix>::as_float(self) }
    fn from_float(float: f64) -> Self { <T as FloatLikeWithSuffix>::from_float(float) }
}

impl FloatLikeWithSuffix for Duration {
    fn suffix() -> &'static str { "s" }
    fn as_float(&self) -> f64 { self.as_secs_f64() }
    fn from_float(f: f64) -> Self { Duration::from_secs_f64(f) }
    fn add_usize(&self, i: usize) -> Self { *self + Duration::from_secs(i as u64) }
    fn sub_usize(&self, i: usize) -> Self { *self - Duration::from_secs(i as u64) }
    fn numeric_metadata(metadata: &Self::Metadata) -> NumericMetadata<Self> { metadata.clone() }
}

impl<T> Editable<DefaultStyle> for T
where
    T: NumericLike,
{
    type TempData = String;

    fn show(
        ui: &mut egui::Ui,
        value: &mut Self,
        metadata: &Self::Metadata,
        temp_data: &mut Option<Self::TempData>,
        id_salt: impl Hash,
        _: &DefaultStyle,
    ) -> egui::Response {
        if let (true, Some(min), Some(max)) = (
            T::metadata_wants_slider(metadata),
            T::metadata_min(metadata),
            T::metadata_max(metadata),
        ) {
            let mut value_float = value.as_float();
            let min_float = min.as_float();
            let max_float = max.as_float();
            let resp = ui.add(egui::Slider::new(&mut value_float, min_float..=max_float).step_by(
                T::metadata_precision(metadata).and_then(|n| n.try_into().ok()).unwrap_or(0.0),
            ));
            if resp.changed() {
                *value = T::from_float(value_float);
            }
            resp
        } else {
            let mut value_str = temp_data.take().unwrap_or_else(|| value.to_string());
            let edit = egui::TextEdit::singleline(&mut value_str).id_salt(id_salt);
            let mut resp = ui.add(edit);
            let parsed = T::parse_from_str(&value_str);
            *temp_data = Some(value_str);
            if resp.changed()
                && let Some(mut parsed) = parsed
            {
                if let Some(min) = T::metadata_min(metadata) {
                    if parsed < min {
                        parsed = min;
                    }
                }
                if let Some(max) = T::metadata_max(metadata) {
                    if parsed > max {
                        parsed = max;
                    }
                }
                *value = parsed;
            } else if resp.has_focus() {
                ui.input_mut(|input| {
                    if let presses @ 1.. =
                        input.count_and_consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)
                    {
                        *value = value.saturating_add_usize(presses);
                        *temp_data = Some(value.to_string());
                        resp.mark_changed();
                    }
                    if let presses @ 1.. =
                        input.count_and_consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
                    {
                        *value = value.saturating_sub_usize(presses);
                        *temp_data = Some(value.to_string());
                        resp.mark_changed();
                    }
                });
            }
            if resp.lost_focus() {
                *temp_data = None;
            }
            resp
        }
    }
}
