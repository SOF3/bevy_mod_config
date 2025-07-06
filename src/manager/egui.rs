use alloc::string::String;

use bevy_ecs::bundle::Bundle;

use crate::manager::{self, Manager};
use crate::{EnumDiscriminant, EnumDiscriminantWrapper};

#[derive(Default)]
pub struct Egui;

impl Manager for Egui {}

impl manager::Supports<u8> for Egui {
    fn new_entity_for_type(&mut self) -> impl Bundle { () }
}

impl manager::Supports<f32> for Egui {
    fn new_entity_for_type(&mut self) -> impl Bundle { () }
}

impl manager::Supports<String> for Egui {
    fn new_entity_for_type(&mut self) -> impl Bundle { () }
}

impl<T: EnumDiscriminant> manager::Supports<EnumDiscriminantWrapper<T>> for Egui {
    fn new_entity_for_type(&mut self) -> impl Bundle { () }
}

#[cfg(feature = "bevy_color")]
impl manager::Supports<bevy_color::Color> for Egui {
    fn new_entity_for_type(&mut self) -> impl Bundle { () }
}
