#![cfg_attr(
    not(feature = "serde_json"),
    allow(unused_imports, reason = "not bothering to optimize per-import cfg")
)]

use std::io::Cursor;
use std::time::Duration;

use bevy::asset::Assets;
use bevy::camera::{Camera2d, ClearColor};
use bevy::math::primitives::Rectangle;
use bevy::math::{Quat, Vec2, Vec3};
use bevy::mesh::{Mesh, Mesh2d};
use bevy::sprite_render::{ColorMaterial, MeshMaterial2d};
use bevy::transform::components::Transform;
use bevy_app::AppExit;
use bevy_color::Color;
use bevy_ecs::component::Component;
use bevy_ecs::query::With;
use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::{IntoScheduleConfigs, SystemSet};
use bevy_ecs::system::{Command, Commands, Local, Res, ResMut, Single};
use bevy_ecs::world::World;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use bevy_mod_config::{AppExt, Config, ReadConfig, manager};
use bevy_time::Time;

#[derive(Config)]
#[config(expose(read, changed))]
struct Settings {
    bg_color: ChooseColor,
    duration: Duration,

    line1: Line,
    #[config(rotate.default = true, fg_color.discrim.default = ChooseColorDiscrim::BevyColor, fg_color.v_BevyColor.0.default = Color::srgb(1.0, 0.0, 0.0))]
    line2: Line,
}

#[derive(Config)]
#[config(expose(read))]
struct Line {
    #[config(default = "Rect width = length of this field")]
    text:      String,
    #[config(default = 10.)]
    thickness: f32,
    rotate:    bool,

    #[config(discrim.default = ChooseColorDiscrim::Rgb, v_Rgb.2.default = 255)]
    fg_color: ChooseColor,
}

#[derive(Config)]
#[config(expose(read, discrim))]
enum ChooseColor {
    White,
    Rgb(u8, u8, u8),
    BevyColor(bevy_color::Color),
}

impl ChooseColorRead<'_> {
    fn to_bevy_color(self) -> Color {
        match self {
            Self::White => Color::WHITE,
            Self::Rgb(r, g, b) => Color::srgb_u8(r, g, b),
            Self::BevyColor(color) => color,
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
    app.add_systems(bevy_app::Update, set_clear_color);
    app.add_systems(
        bevy_app::Update,
        display_line::<MainShape1>.after(show_settings).in_set(DisplayLines),
    );
    app.add_systems(
        bevy_app::Update,
        display_line::<MainShape2>.after(show_settings).in_set(DisplayLines),
    );
    #[cfg(feature = "serde_json")]
    app.add_systems(
        EguiPrimaryContextPass,
        show_json_editor.after(show_settings).before(DisplayLines),
    );

    app.run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
struct DisplayLines;

fn show_settings(mut contexts: EguiContexts, mut display: manager::egui::Display) {
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

trait MainShape: Component {
    fn get_settings<'a>(settings: SettingsRead<'a>) -> LineRead<'a>;
}

#[derive(Component)]
struct MainShape1;

impl MainShape for MainShape1 {
    fn get_settings<'a>(settings: SettingsRead<'a>) -> LineRead<'a> { settings.line1 }
}

impl MainShape for MainShape2 {
    fn get_settings<'a>(settings: SettingsRead<'a>) -> LineRead<'a> { settings.line2 }
}

#[derive(Component)]
struct MainShape2;

fn init_line(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mesh = meshes.add(Mesh::from(Rectangle { half_size: Vec2::new(10., 1.) }));
    let mut new_material = || {
        materials.add(ColorMaterial {
            color: ChooseColorRead::White.to_bevy_color(),
            ..Default::default()
        })
    };
    commands.spawn((
        Mesh2d(mesh.clone()),
        MeshMaterial2d(new_material()),
        Transform::from_scale(Vec3::new(1., 10., 1.)).with_translation(Vec3::new(0., 30., 0.)),
        MainShape1,
    ));
    commands.spawn((
        Mesh2d(mesh),
        MeshMaterial2d(new_material()),
        Transform::from_scale(Vec3::new(1., 10., 1.)).with_translation(Vec3::new(0., -30., 0.)),
        MainShape2,
    ));
}

fn set_clear_color(mut clear_color: ResMut<ClearColor>, settings: ReadConfig<Settings>) {
    let settings = settings.read();
    clear_color.0 = settings.bg_color.to_bevy_color();
}

fn display_line<WhichShape: MainShape>(
    settings: ReadConfig<Settings>,
    mut shape: Single<(&MeshMaterial2d<ColorMaterial>, &mut Transform), With<WhichShape>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut last_changed: Local<Option<(Duration, SettingsChanged)>>,
    time: Res<Time>,
) {
    let last_change_time = match *last_changed {
        None => {
            *last_changed = Some((time.elapsed(), settings.changed()));
            time.elapsed()
        }
        Some((last_change_time, ref last_changed_value)) => {
            if *last_changed_value != settings.changed() {
                *last_changed = Some((time.elapsed(), settings.changed()));
                time.elapsed()
            } else {
                last_change_time
            }
        }
    };
    let time_since_change = (time.elapsed() - last_change_time).as_secs_f32();

    let settings = settings.read();
    let line_settings = WhichShape::get_settings(settings);

    let (MeshMaterial2d(material_handle), ref mut shape_transform) = *shape;
    materials.get_mut(material_handle).unwrap().color = line_settings.fg_color.to_bevy_color();
    shape_transform.scale.x = line_settings.text.len() as f32;
    shape_transform.scale.y = line_settings.thickness;
    shape_transform.rotation =
        Quat::from_rotation_z(time_since_change * if line_settings.rotate { 1.0 } else { 0.0 });
}
