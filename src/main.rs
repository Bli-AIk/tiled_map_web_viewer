use bevy::prelude::*;
use bevy_ecs_tiled::prelude::*;
use bevy_workbench::console::console_log_layer;
use bevy_workbench::prelude::*;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Tiled Map Web Viewer".into(),
                        resolution: (1280u32, 720u32).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(bevy::log::LogPlugin {
                    custom_layer: console_log_layer,
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(WorkbenchPlugin {
            config: WorkbenchConfig {
                show_toolbar: false,
                ..default()
            },
        })
        .add_plugins(TiledPlugin::default())
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}
