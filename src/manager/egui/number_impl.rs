use alloc::string::{String, ToString};
use core::fmt;
use core::hash::Hash;
use core::str::FromStr;

use bevy_egui::egui;

use super::Editable;
use crate::ConfigField;
use crate::impls::NumericMetadata;

trait Number:
    num_traits::Bounded
    + num_traits::Num
    + Copy
    + PartialOrd
    + ConfigField<Metadata = NumericMetadata<Self>>
    + fmt::Display
    + FromStr
{
    fn saturating_add_usize(self, i: usize) -> Self;

    fn saturating_sub_usize(self, i: usize) -> Self;
}

macro_rules! impl_number_signed {
    ($(($ty:ty, $unsigned:ty),)*) => {
        $(
            impl Number for $ty {
                fn saturating_add_usize(self, i: usize) -> Self {
                    self.saturating_add_unsigned(<$unsigned>::try_from(i).unwrap_or_else(|_| <$unsigned>::max_value()))
                }

                fn saturating_sub_usize(self, i: usize) -> Self {
                    self.saturating_sub_unsigned(<$unsigned>::try_from(i).unwrap_or_else(|_| <$unsigned>::max_value()))
                }
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
            impl Number for $ty {
                fn saturating_add_usize(self, i: usize) -> Self {
                    self.saturating_add(Self::try_from(i).unwrap_or_else(|_| Self::max_value()))
                }

                fn saturating_sub_usize(self, i: usize) -> Self {
                    self.saturating_sub(Self::try_from(i).unwrap_or_else(|_| Self::max_value()))
                }
            }
        )*
    };
}

impl_number_unsigned!(u8, u16, u32, u64, u128, usize);

impl Number for f32 {
    fn saturating_add_usize(self, i: usize) -> Self { self + i as f32 }

    fn saturating_sub_usize(self, i: usize) -> Self { self - i as f32 }
}

impl Number for f64 {
    fn saturating_add_usize(self, i: usize) -> Self { self + i as f64 }

    fn saturating_sub_usize(self, i: usize) -> Self { self - i as f64 }
}

impl<T> Editable for T
where
    T: Number,
{
    type TempData = String;

    fn show(
        ui: &mut egui::Ui,
        value: &mut Self,
        metadata: &Self::Metadata,
        temp_data: &mut Option<Self::TempData>,
        id_salt: impl Hash,
    ) -> egui::Response {
        let mut value_str = temp_data.take().unwrap_or_else(|| value.to_string());
        let edit = egui::TextEdit::singleline(&mut value_str).id_salt(id_salt);
        let resp = ui.add(edit);
        let parsed = value_str.parse::<Self>().ok();
        *temp_data = Some(value_str);
        if resp.changed()
            && let Some(parsed) = parsed
        {
            let min = metadata.min.unwrap_or_else(T::min_value);
            let max = metadata.max.unwrap_or_else(T::max_value);
            *value = num_traits::clamp(parsed, min, max);
        } else if resp.has_focus() {
            ui.input_mut(|input| {
                if let presses @ 1.. =
                    input.count_and_consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)
                {
                    *value = value.saturating_add_usize(presses);
                    *temp_data = Some(value.to_string());
                }
                if let presses @ 1.. =
                    input.count_and_consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
                {
                    *value = value.saturating_sub_usize(presses);
                    *temp_data = Some(value.to_string());
                }
            });
        }
        if resp.lost_focus() {
            *temp_data = None;
        }
        resp
    }
}
