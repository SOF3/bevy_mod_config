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

type ManagerType = ();

fn main() {
    let mut app = bevy_app::App::new();
    app.init_config::<ManagerType, Foo>("foo");
    app.add_systems(bevy_app::Update, |foo: ReadConfig<Foo>| {
        let foo = foo.read();
        assert_eq!(foo.thickness, 3);
        assert!(matches!(foo.color, ColorRead::White));
    });
    app.update();
}
