use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk};
use bevy_ecs_tiled::prelude::*;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Tiled Map Test".into(),
                        resolution: (1280u32, 720u32).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(ClearColor(Color::srgb(0.2, 0.2, 0.3)))
        .add_plugins(TiledPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (take_screenshot, debug_tiles))
        .insert_resource(ScreenshotTimer(Timer::from_seconds(5.0, TimerMode::Once)))
        .insert_resource(DebugDone(false))
        .run();
}

#[derive(Resource)]
struct ScreenshotTimer(Timer);

#[derive(Resource)]
struct DebugDone(bool);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    let map_handle: Handle<TiledMapAsset> = asset_server.load("maps/001-3.tmx");
    commands.spawn((TiledMap(map_handle), TilemapAnchor::Center));
    info!("Loading map: maps/001-3.tmx");
}

fn take_screenshot(mut timer: ResMut<ScreenshotTimer>, time: Res<Time>, mut commands: Commands) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        info!("Taking screenshot...");
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk("screenshot_debug.png"));
    }
}

fn debug_tiles(
    mut done: ResMut<DebugDone>,
    tilemaps: Query<(Entity, &Name), With<TiledMap>>,
    children_query: Query<&Children>,
    name_query: Query<&Name>,
) {
    if done.0 {
        return;
    }
    let count = tilemaps.iter().count();
    if count > 0 {
        done.0 = true;
        info!("=== MAP ENTITIES: {} ===", count);
        for (entity, name) in tilemaps.iter() {
            info!("  Map: {:?} name={}", entity, name.as_str());
            if let Ok(children) = children_query.get(entity) {
                for child in children.iter() {
                    let child_name = name_query
                        .get(child)
                        .map(|n| n.as_str().to_string())
                        .unwrap_or_default();
                    info!("    Child: {:?} name={}", child, child_name);
                }
            }
        }
    }
}
