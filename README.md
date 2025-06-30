# bevy\_mod\_config

A bevy plugin for configuration management.

## Concepts
- Scalar: A non-composite config type, e.g. a number, string, color, direction, etc.
- Root field: A top-level config field directly registered to the app.
- Manager: A plugin that works on scalar config fields.
- Reader: A type derived from `#[derive(Config)]` that can be used to access config values
  through `ReadConfig` in systems.

## Usage
Declare one or more config hierarchies with `#[derive(Config)]`:

```rs
#[derive(Config)]
struct Foo {
    thickness: i32,
    color:     Color,
}

#[derive(Config)]
#[config(expose(read))] // Expose the generated `ColorRead`
enum Color {
    White,
    Black,
}
```

Initialize the root field with `App::init_config`:

```rs
type ManagerType = (Manager1, Manager2, ...);
    bevy_mod_config::ScalarManager,
)

app.init_config::<ManagerType, Foo>("foo");
```

Root fields can be accessed from systems using `ReadConfig`:

```rs
fn my_system(foo: ReadConfig<Foo>) {
    let foo = foo.read();
    assert_eq!(foo.thickness, 3);
    assert!(matches!(foo.color, ColorRead::White));
}
```

Note that `read()` returns the Reader type instead of the original type
(similar to how `#[derive(QueryData)]` gives `XxxItem` to systems).
This may have an impact on matching and passing values around.
The Reader type may be accessed as `<Foo as ConfigField>::Read<'_>`,
or directly exposed with `#[config(expose(read))]` after the derive.

## Managers
Managers enable systematic management of scalar config fields.

- Storage:
  - `bevy_mod_config::manager::Serde` exposes APIs to
    load/save config values to/from serialized data.
- Editors:
  - `bevy_mod_config::manager::EguiEditor` provides an in-game `egui` editor
    to modify config values live.
