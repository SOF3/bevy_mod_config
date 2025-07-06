#[cfg(all(feature = "egui", feature = "bevy_color"))]
mod example {
    use std::io::Cursor;

    use bevy::core_pipeline::core_2d::Camera2d;
    use bevy::math::UVec2;
    use bevy::render::camera::{Camera, Viewport};
    use bevy::render::view::RenderLayers;
    use bevy::window::{PrimaryWindow, Window};
    use bevy_app::AppExit;
    use bevy_color::{Color, ColorToPacked};
    use bevy_ecs::query::{With, Without};
    use bevy_ecs::resource::Resource;
    use bevy_ecs::schedule::IntoScheduleConfigs;
    #[cfg(feature = "serde_json")]
    use bevy_ecs::system::ResMut;
    use bevy_ecs::system::{Command, Commands, Local, Res, Single};
    use bevy_ecs::world::World;
    use bevy_egui::{EguiContext, EguiContexts, EguiPrimaryContextPass, PrimaryEguiContext, egui};
    use bevy_mod_config::{AppExt, Config, ConfigData, ReadConfig, manager};
    use bevy_time::Time;

    #[derive(Config)]
    struct Settings {
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
    type SerdeJsonManager = bevy_mod_config::manager::serde::Json;
    #[cfg(not(feature = "serde_json"))]
    type SerdeJsonManager = ();

    type ManagerType = (SerdeJsonManager, manager::Egui);

    pub(super) fn main() -> AppExit {
        let mut app = bevy_app::App::new();
        app.add_plugins((bevy::DefaultPlugins, bevy_egui::EguiPlugin::default()));

        app.init_config::<ManagerType, Settings>("ui");

        app.add_systems(bevy_app::Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
            commands.spawn((
                PrimaryEguiContext,
                Camera2d,
                RenderLayers::none(),
                Camera { order: 1, ..Default::default() },
            ));
        });
        app.add_systems(EguiPrimaryContextPass, show_settings);
        app.add_systems(EguiPrimaryContextPass, display_line.after(show_settings));
        #[cfg(feature = "serde_json")]
        app.add_systems(
            EguiPrimaryContextPass,
            show_json_editor.after(show_settings).before(display_line),
        );

        app.run()
    }

    #[derive(Resource)]
    struct PanelWidth {
        left:  f32,
        right: f32,
    }

    fn show_settings(mut contexts: EguiContexts, mut panel_width: ResMut<PanelWidth>) {
        let Ok(ctx) = contexts.ctx_mut() else { return };

        let left = egui::SidePanel::left("settings")
            .show(ctx, |ui| {
                ui.heading("Settings");
            })
            .response
            .rect
            .width();
        panel_width.left = left;
    }

    #[cfg(feature = "serde_json")]
    #[derive(Resource)]
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
        mut editor_text: Local<Option<String>>,
        mut commands: Commands,
        mut panel_width: ResMut<PanelWidth>,
    ) {
        let Ok(ctx) = contexts.ctx_mut() else { return };

        let right = egui::SidePanel::right("json_editor")
            .show(ctx, |ui| {
                ui.heading("JSON Editor");

                if ui.button("Dump JSON").clicked() {
                    commands.queue(DumpJsonCommand);
                }
                if ui.button("Reload JSON").clicked() {
                    commands.queue(LoadJsonCommand);
                }

                if let Some(text) = &mut *editor_text {
                    ui.text_edit_multiline(text);
                } else {
                    ui.label("Press 'Dump JSON' to see the current settings as JSON.");
                }
            })
            .response
            .rect
            .width();
        panel_width.right += right;
    }

    fn display_line(
        mut contexts: EguiContexts,
        time: Res<Time>,
        settings: ReadConfig<Settings>,
        mut commands: Commands,
        panel_width: Res<PanelWidth>,
        window: Option<Single<&mut Window, (With<PrimaryWindow>, Without<ConfigData>)>>,
        camera: Option<Single<&mut Camera, (Without<ConfigData>, Without<EguiContext>)>>,
    ) {
        let settings = settings.read();
        let Ok(ctx) = contexts.ctx_mut() else { return };
        let Some(window) = window else { return };
        let Some(mut camera) = camera else { return };

        let left = panel_width.left * window.scale_factor();
        let right = panel_width.right * window.scale_factor();
        let pos = UVec2::new(left as u32, 0);
        let size = UVec2::new(window.physical_width(), window.physical_height())
            - pos
            - UVec2::new(right as u32, 0);

        camera.viewport =
            Some(Viewport { physical_position: pos, physical_size: size, ..Default::default() });
    }
}

#[cfg(all(feature = "egui", feature = "bevy_color"))]
fn main() { example::main(); }

#[cfg(not(all(feature = "egui", feature = "bevy_color")))]
fn main() {
    eprintln!("This example requires both the `egui` and `bevy_color` features to be enabled.");
    std::process::exit(1);
}
