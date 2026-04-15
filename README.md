# bevy\_mod\_config
A modular configuration framework for Bevy applications,
decoupling configuration access and change detection from
management utilities like persistence and UI.

# Why do I need a framework?

- As a library author, I want to declare only the configuration model my plugin needs,
  so I can focus on what the library serves instead of how it is managed.
- As a game developer, I want to choose how settings are presented, persisted, or transferred,
  so I can compose many configurable libraries without coupling them to one workflow.

`bevy_mod_config` comes with batteries included:
`manager::Serde` for persistence and `manager::Egui` for live editor UI,
both reusable with the different config models.
You can also write your own managers for other workflows like
using another UI framework or synchronizing over the network.

# How it looks
Library code:

```rs
#[derive(Config)]
struct WindowSettings {
    #[config(default = 1920, min = 100)]
    width:      u32,
    #[config(default = 1280, min = 100)]
    height:     u32,
}

struct WindowPlugin<M>(PhantomData<M>);

impl<M> Plugin for WindowPlugin<M>
where
    WindowSettings: ConfigFieldFor<M>,
{
    fn build(&self, app: &mut App) { app.init_config::<M, WindowSettings>("video"); }
}

fn apply_video(settings: ReadConfig<WindowSettings>) {
    let settings = settings.read();
    if settings.fullscreen {
        // ...
    }
}

fn resize_system(mut settings: ReadConfigChange<WindowSettings>) {
    if settings.consume_change() {
        let settings = settings.read();
        resize_window(settings.width, settings.height);
    }
}
```

User main code:

```rs
type ManagerType = (manager::Egui, manager::serde::Json);

fn main() {
    App::new()
        .add_plugins(WindowPlugin::<ManagerType>(PhantomData))
        .add_systems(
            EguiPrimaryContextPass,
            |mut ctxs: EguiContexts, display: manager::egui::Display| {
                egui::Window::new("Settings").show(ctxs.ctx_mut()?, |ui| {
                    display.show(ui);
                });
                Ok(())
            },
        );
}
```
