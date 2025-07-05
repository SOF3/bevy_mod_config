use bevy_mod_config::{AppExt, Config, ReadConfig};

#[derive(Config)]
struct Settings {
    #[config(default = 3)]
    thickness: i32,
    color:     Color,
}

#[derive(Config)]
#[config(expose(read))]
enum Color {
    White,
    Rgb(f32, f32, f32),
    Rgba(Rgba),
    Named { code: String },
}

#[derive(Config)]
struct Rgba(f32, f32, f32, f32);

#[cfg(feature = "serde_json")]
type ManagerType = (bevy_mod_config::manager::serde::Json,);
#[cfg(not(feature = "serde_json"))]
type ManagerType = ();

#[cfg_attr(test, test)]
fn main() {
    let mut app = bevy_app::App::new();
    app.init_config::<ManagerType, Settings>("ui");
    app.add_systems(bevy_app::Update, |settings: ReadConfig<Settings>| {
        let settings = settings.read();
        assert_eq!(settings.thickness, 3);
        assert!(matches!(settings.color, ColorRead::White));
    });
    app.update();

    #[cfg(feature = "serde_json")]
    dump_json(&mut app);
    #[cfg(feature = "serde_json")]
    load_json(&mut app);
}

#[cfg(feature = "serde_json")]
fn dump_json(app: &mut bevy_app::App) {
    use bevy_mod_config::manager;

    let (json,) = &app.world_mut().resource::<manager::Instance<ManagerType>>().instance;
    let json = json.clone();
    let data = json.to_string(app.world_mut()).unwrap();
    assert_eq!(
        data,
        r#"{"ui.color.Named:code":"","ui.color.Rgb:0":0.0,"ui.color.Rgb:1":0.0,"ui.color.Rgb:2":0.0,"ui.color.Rgba:0.0":0.0,"ui.color.Rgba:0.1":0.0,"ui.color.Rgba:0.2":0.0,"ui.color.Rgba:0.3":0.0,"ui.color.discrim":"White","ui.thickness":3}"#
    );
}

#[cfg(feature = "serde_json")]
fn load_json(app: &mut bevy_app::App) {
    use std::io::Cursor;

    use bevy_ecs::system::RunSystemOnce;

    let input = String::from(
        r#"{
        "ui.thickness": 5,
        "ui.color.discrim": "Named",
        "ui.color.Named:code": "red"
    }"#,
    );
    let (json,) =
        &app.world_mut().resource::<bevy_mod_config::manager::Instance<ManagerType>>().instance;
    let json = json.clone();
    json.from_reader(app.world_mut(), Cursor::new(input)).unwrap();

    app.world_mut()
        .run_system_once(|settings: ReadConfig<Settings>| {
            let settings = settings.read();
            assert_eq!(settings.thickness, 5);
            assert!(matches!(settings.color, ColorRead::Named { code: "red" }));
        })
        .unwrap();
}
