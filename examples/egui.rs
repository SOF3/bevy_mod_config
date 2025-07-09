use std::io::Cursor;

use bevy::asset::Assets;
use bevy::core_pipeline::core_2d::Camera2d;
use bevy::math::primitives::Rectangle;
use bevy::math::{Vec2, Vec3};
use bevy::render::mesh::{Mesh, Mesh2d};
use bevy::transform::components::Transform;
use bevy_app::AppExit;
use bevy_color::Color;
use bevy_ecs::component::Component;
use bevy_ecs::query::With;
use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::system::{Command, Commands, ResMut, Single};
use bevy_ecs::world::World;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use bevy_mod_config::{AppExt, Config, ReadConfig, manager};
use bevy_sprite::{ColorMaterial, MeshMaterial2d};

#[derive(Config)]
struct Settings {
    #[config(default = "Rect width = length of this field")]
    text:      String,
    #[config(default = 10.)]
    thickness: f32,
    color:     ChooseColor,
}

#[derive(Config)]
#[config(expose(read))]
enum ChooseColor {
    White,
    Rgb(u8, u8, u8),
    BevyColor(bevy_color::Color),
}

impl ChooseColorRead<'_> {
    fn to_color(&self) -> Color {
        match self {
            Self::White => Color::WHITE,
            &Self::Rgb(r, g, b) => Color::srgb_u8(r, g, b),
            &Self::BevyColor(color) => color,
        }
    }
}

#[derive(Config)]
struct Rgba(f32, f32, f32, f32);

#[cfg(feature = "serde_json")]
type SerdeJsonManager = bevy_mod_config::manager::serde::json::Pretty;
#[cfg(not(feature = "serde_json"))]
type SerdeJsonManager = ();

type ManagerType = (SerdeJsonManager, manager::Egui);

fn main() -> AppExit {
    let mut app = bevy_app::App::new();
    app.add_plugins((bevy::DefaultPlugins, bevy_egui::EguiPlugin::default()));

    app.init_config::<ManagerType, Settings>("ui");

    #[cfg(feature = "serde_json")]
    app.init_resource::<JsonEditorText>();
    app.add_systems(bevy_app::Startup, |mut commands: Commands| {
        commands.spawn(Camera2d);
    });
    app.add_systems(EguiPrimaryContextPass, show_settings);
    app.add_systems(bevy_app::Startup, init_line);
    app.add_systems(EguiPrimaryContextPass, display_line.after(show_settings));
    #[cfg(feature = "serde_json")]
    app.add_systems(
        EguiPrimaryContextPass,
        show_json_editor.after(show_settings).before(display_line),
    );

    app.run()
}

fn show_settings(mut contexts: EguiContexts, mut display: manager::egui::Display<ManagerType>) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    egui::SidePanel::left("settings").show(ctx, |ui| {
        ui.heading("Settings");
        display.show(ui);
    });
}

#[cfg(feature = "serde_json")]
#[derive(Resource, Default)]
struct JsonEditorText {
    text: Option<String>,
}

#[cfg(feature = "serde_json")]
struct DumpJsonCommand;

#[cfg(feature = "serde_json")]
impl Command for DumpJsonCommand {
    fn apply(self, world: &mut World) {
        let manager = world.resource::<manager::Instance<ManagerType>>();
        match manager.0.clone().to_string(world) {
            Ok(json) => {
                let mut editor = world.resource_mut::<JsonEditorText>();
                editor.text = Some(json);
            }
            Err(err) => {
                bevy_log::error!("Failed to dump JSON: {err}");
                let mut editor = world.resource_mut::<JsonEditorText>();
                editor.text = None;
            }
        }
    }
}

#[cfg(feature = "serde_json")]
struct LoadJsonCommand;

#[cfg(feature = "serde_json")]
impl Command for LoadJsonCommand {
    fn apply(self, world: &mut World) {
        let editor = world.resource_mut::<JsonEditorText>();
        let Some(text) = editor.text.clone() else {
            bevy_log::error!("No JSON text to load");
            return;
        };
        let manager = world.resource::<manager::Instance<ManagerType>>();
        if let Err(err) = manager.0.clone().from_reader(world, Cursor::new(text.into_bytes())) {
            bevy_log::error!("Failed to load JSON: {err}");
        }
    }
}

#[cfg(feature = "serde_json")]
fn show_json_editor(
    mut contexts: EguiContexts,
    mut editor_text: ResMut<JsonEditorText>,
    mut commands: Commands,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    egui::SidePanel::right("json_editor").show(ctx, |ui| {
        ui.heading("JSON Editor");

        if ui.button("Dump JSON").clicked() {
            commands.queue(DumpJsonCommand);
        }
        if ui.button("Reload JSON").clicked() {
            commands.queue(LoadJsonCommand);
        }

        if let Some(text) = &mut editor_text.text {
            ui.add(egui::TextEdit::multiline(text).code_editor().desired_rows(20));
        } else {
            ui.label("Press 'Dump JSON' to see the current settings as JSON.");
        }
    });
}

#[derive(Component)]
struct MainShape;

fn init_line(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mesh = meshes.add(Mesh::from(Rectangle { half_size: Vec2::new(10., 1.) }));
    let material = materials
        .add(ColorMaterial { color: ChooseColorRead::White.to_color(), ..Default::default() });
    commands.spawn((
        Mesh2d(mesh),
        MeshMaterial2d(material),
        Transform::from_scale(Vec3::new(1., 10., 1.)),
        MainShape,
    ));
}

fn display_line(
    settings: ReadConfig<Settings>,
    mut shape: Single<(&MeshMaterial2d<ColorMaterial>, &mut Transform), With<MainShape>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let settings = settings.read();

    let (MeshMaterial2d(material_handle), ref mut shape_transform) = *shape;
    materials.get_mut(material_handle).unwrap().color = settings.color.to_color();
    shape_transform.scale.x = settings.text.len() as f32;
    shape_transform.scale.y = settings.thickness;
}
