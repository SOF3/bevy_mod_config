use bevy_mod_config::{AppExt, Config, ReadConfig};

#[derive(Config)]
struct Foo {
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
    app.init_config::<ManagerType, Foo>("foo");
    app.add_systems(bevy_app::Update, |foo: ReadConfig<Foo>| {
        let foo = foo.read();
        assert_eq!(foo.thickness, 3);
        assert!(matches!(foo.color, ColorRead::White));
    });
    app.update();

    #[cfg(feature = "serde_json")]
    dump_json(&mut app);
}

#[cfg(feature = "serde_json")]
fn dump_json(app: &mut bevy_app::App) {
    use bevy_mod_config::manager;

    let (json,) = &app.world_mut().resource::<manager::Instance<ManagerType>>().instance;
    let json = json.clone();
    let data = json.to_string(app.world_mut()).unwrap();
    assert_eq!(
        data,
        r#"{"foo.color.Named:code":"","foo.color.Rgb:0":0.0,"foo.color.Rgb:1":0.0,"foo.color.Rgb:2":0.0,"foo.color.Rgba:0.0":0.0,"foo.color.Rgba:0.1":0.0,"foo.color.Rgba:0.2":0.0,"foo.color.Rgba:0.3":0.0,"foo.color.discrim":"White","foo.thickness":3}"#
    );
}
