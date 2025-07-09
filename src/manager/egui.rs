use alloc::string::String;
use alloc::vec::Vec;
use core::hash::Hash;

use bevy_ecs::bundle::Bundle;
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::query::{QueryFilter, With, Without};
use bevy_ecs::system::{Query, Res, SystemParam};
use bevy_ecs::world::EntityMut;
use bevy_egui::{EguiContext, egui};

use crate::manager::{self, Manager};
use crate::{
    ChildNodeList, ConditionalRelevance, ConfigField, ConfigNode, EnumDiscriminant,
    EnumDiscriminantWrapper, RootNode, ScalarData, ScalarMetadata,
};

/// A [`Manager`] providing an editor UI for config fields through [egui].
#[derive(Default)]
pub struct Egui;

/// A type erasure vtable attached to each scalar field to describe how to draw it in egui.
#[derive(Component)]
struct ScalarDraw {
    draw_fn: fn(&mut egui::Ui, &mut EntityMut<'_>) -> egui::Response,
}

impl Manager for Egui {}

impl<T> manager::Supports<T> for Egui
where
    T: Editable + Send + Sync + 'static,
    T::Metadata: Clone,
{
    fn new_entity_for_type(&mut self) -> impl Bundle {
        (
            ScalarDraw {
                draw_fn: |ui, entity| {
                    #[derive(Hash)]
                    struct FieldIdSalt(Entity);

                    let id_salt = FieldIdSalt(entity.id());

                    ui.horizontal_top(|ui| {
                        let node = entity
                            .get::<ConfigNode>()
                            .expect("draw_fn must be called with a ConfigNode entity");
                        ui.label(node.path.last().expect("node path must be nonempty"));

                        let metadata = entity
                            .get::<ScalarMetadata<T>>()
                            .expect(
                                "caller of new_entity must populate the metadata componentwith \
                                 the corresponding type",
                            )
                            .0
                            .clone();

                        let mut temp_data = entity
                            .get_mut::<TempData<T::TempData>>()
                            .expect("inserted with ScalarDraw");
                        let mut temp_data = temp_data.0.take();

                        let mut field = entity.get_mut::<ScalarData<T>>().expect(
                            "caller of new_entity must populate entity with the corresponding \
                             ScalarData type",
                        );

                        let resp = T::show(ui, &mut field.0, &metadata, &mut temp_data, id_salt);

                        entity
                            .get_mut::<TempData<T::TempData>>()
                            .expect("inserted with ScalarDraw")
                            .0 = temp_data;

                        if resp.changed() {
                            let mut node =
                                entity.get_mut::<ConfigNode>().expect("checked at the beginning");
                            node.generation = node.generation.next();
                        }
                        resp
                    })
                    .response
                },
            },
            TempData::<T::TempData>(None),
        )
    }
}

#[derive(Component)]
struct TempData<T>(Option<T>);

#[derive(SystemParam)]
pub struct Display<'w, 's, M: Manager, F: QueryFilter + 'static = ()> {
    _manager:   Res<'w, manager::Instance<M>>,
    node_query: Query<'w, 's, EntityMut<'static>, (Without<EguiContext>, F)>,
    root_query: Query<'w, 's, Entity, With<RootNode>>,
}

impl<M: Manager, F: QueryFilter + 'static> Display<'_, '_, M, F> {
    pub fn show(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            for root in &self.root_query {
                show_node(ui, &mut self.node_query, root);
            }
        })
        .response
    }
}

fn show_node<F: QueryFilter + 'static>(
    ui: &mut egui::Ui,
    node_query: &mut Query<EntityMut, F>,
    id: Entity,
) {
    {
        let entity = node_query.get(id).expect("config node must remain in the world once spawned");
        if let Some(&ConditionalRelevance { dependency, is_entity_relevant }) = entity.get() {
            let dep = match node_query.get(dependency) {
                Ok(dep) => dep,
                Err(err) => {
                    panic!("Config node {id:?} references invalid dependency {dependency:?}: {err}")
                }
            };
            if !is_entity_relevant(dep) {
                // If the dependency is not relevant, skip this node.
                return;
            }
        }
    }

    let mut entity =
        node_query.get_mut(id).expect("config node must remain in the world once spawned");
    if let Some(&ScalarDraw { draw_fn }) = entity.get() {
        draw_fn(ui, &mut entity);
    } else if let Some(children) = entity.get::<ChildNodeList>() {
        let children: Vec<_> = children.iter().copied().collect();
        let node = entity.get::<ConfigNode>().expect("show_node must provide a ConfigNode");
        let path = node.path.last().expect("node path must be nonempty").clone();
        ui.collapsing(path, |ui| {
            for child in children {
                show_node(ui, node_query, child);
            }
        });
    }
}

pub trait Editable: ConfigField {
    type TempData: Send + Sync + 'static;

    fn show(
        ui: &mut egui::Ui,
        value: &mut Self,
        metadata: &Self::Metadata,
        temp: &mut Option<Self::TempData>,
        id_salt: impl Hash,
    ) -> egui::Response;
}

mod number_impl;

impl Editable for String {
    type TempData = ();

    fn show(
        ui: &mut egui::Ui,
        value: &mut Self,
        metadata: &Self::Metadata,
        _: &mut Option<()>,
        id_salt: impl Hash,
    ) -> egui::Response {
        let editor = if metadata.multiline {
            egui::TextEdit::multiline(value)
        } else {
            egui::TextEdit::singleline(value)
        }
        .char_limit(metadata.max_length.unwrap_or(usize::MAX))
        .id_salt(id_salt);
        ui.add(editor)
    }
}

impl<T: EnumDiscriminant> manager::Supports<EnumDiscriminantWrapper<T>> for Egui {
    fn new_entity_for_type(&mut self) -> impl Bundle {
        ScalarDraw {
            draw_fn: |ui, entity| {
                #[derive(Hash)]
                struct FieldIdSalt(Entity);

                let id_salt = FieldIdSalt(entity.id());

                ui.horizontal_top(|ui| {
                    let mut field =
                        entity.get_mut::<ScalarData<EnumDiscriminantWrapper<T>>>().expect(
                            "caller of new_entity must populate entity with the corresponding \
                             ScalarData type",
                        );

                    let resp = egui::ComboBox::from_id_salt(id_salt)
                        .selected_text(field.0.0.name())
                        .show_ui(ui, |ui| {
                            for variant in T::VARIANTS {
                                ui.selectable_value(&mut field.0.0, *variant, variant.name());
                            }
                        })
                        .response;

                    if resp.changed() {
                        let mut node = entity
                            .get_mut::<ConfigNode>()
                            .expect("draw_fn must be called with a ConfigNode entity");
                        node.generation = node.generation.next();
                    }
                    resp
                })
                .response
            },
        }
    }
}

#[cfg(feature = "bevy_color")]
impl Editable for bevy_color::Color {
    type TempData = ();
    fn show(
        ui: &mut egui::Ui,
        value: &mut Self,
        metadata: &Self::Metadata,
        _: &mut Option<()>,
        _: impl Hash,
    ) -> egui::Response {
        use bevy_color::ColorToPacked;
        use bevy_egui::egui::color_picker::{self, color_edit_button_srgba};

        let [r, g, b, a] = value.to_srgba().to_u8_array();
        let mut color32 = egui::Color32::from_rgba_unmultiplied(r, g, b, a);
        let resp = color_edit_button_srgba(
            ui,
            &mut color32,
            if metadata.alpha_blend {
                if metadata.alpha_additive {
                    color_picker::Alpha::BlendOrAdditive
                } else {
                    color_picker::Alpha::OnlyBlend
                }
            } else {
                color_picker::Alpha::Opaque
            },
        );

        if resp.changed() {
            let [r, g, b, a] = color32.to_array();
            *value = bevy_color::Color::srgba_u8(r, g, b, a)
        }
        resp
    }
}
