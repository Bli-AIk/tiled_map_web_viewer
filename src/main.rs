use bevy::prelude::*;
use bevy_ecs_tiled::prelude::*;
use bevy_workbench::console::console_log_layer;
use bevy_workbench::dock::WorkbenchPanel;
use bevy_workbench::prelude::*;

/// Request to load a specific map file.
#[derive(Resource, Default)]
struct MapLoadRequest {
    map_to_load: Option<String>,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Tiled Map Web Viewer".into(),
                    resolution: (1280u32, 720u32).into(),
                    ..default()
                }),
                ..default()
            })
            .set(ImagePlugin::default_nearest())
            .set(bevy::log::LogPlugin {
                custom_layer: console_log_layer,
                ..default()
            }),
    )
    .insert_resource(ClearColor(Color::BLACK))
    .add_plugins(WorkbenchPlugin::default())
    .add_plugins(TiledPlugin::default())
    .init_resource::<MapLoadRequest>()
    .add_systems(Startup, setup)
    .add_systems(Update, handle_map_load);

    app.register_panel(MapListPanel::default());

    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Handles pending map load requests from the UI panel.
fn handle_map_load(
    mut commands: Commands,
    mut load_request: ResMut<MapLoadRequest>,
    asset_server: Res<AssetServer>,
    existing_maps: Query<Entity, With<TiledMap>>,
) {
    let Some(map_name) = load_request.map_to_load.take() else {
        return;
    };

    for entity in &existing_maps {
        commands.entity(entity).despawn();
    }

    let map_handle: Handle<TiledMapAsset> = asset_server.load(&map_name);
    commands.spawn((TiledMap(map_handle), TilemapAnchor::Center));
    info!("Loading map: {}", map_name);
}

// --- Map List Panel ---

#[derive(Default)]
struct MapListPanel {
    maps: Vec<String>,
    scanned: bool,
    selected: Option<String>,
}

impl MapListPanel {
    fn scan_maps(&mut self) {
        let assets_dir = std::path::Path::new("assets");
        if assets_dir.exists() {
            self.walk_dir(assets_dir, assets_dir);
        }
        self.maps.sort();
    }

    fn walk_dir(&mut self, dir: &std::path::Path, base: &std::path::Path) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                self.walk_dir(&path, base);
            } else if path.extension().is_some_and(|ext| ext == "tmx") {
                if let Ok(rel) = path.strip_prefix(base) {
                    self.maps.push(rel.to_string_lossy().to_string());
                }
            }
        }
    }
}

impl WorkbenchPanel for MapListPanel {
    fn id(&self) -> &str {
        "map_list"
    }

    fn title(&self) -> String {
        "Map List".into()
    }

    fn ui(&mut self, _ui: &mut egui::Ui) {}

    fn ui_world(&mut self, ui: &mut egui::Ui, world: &mut World) {
        if !self.scanned {
            self.scan_maps();
            self.scanned = true;
        }

        ui.heading("Maps");
        ui.separator();

        if self.maps.is_empty() {
            ui.label("No .tmx files found in assets/");
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut load_target = None;

            for map_name in &self.maps {
                let is_selected = self.selected.as_deref() == Some(map_name);
                let text = if is_selected {
                    egui::RichText::new(map_name).strong()
                } else {
                    egui::RichText::new(map_name)
                };
                if ui.selectable_label(is_selected, text).clicked() && !is_selected {
                    load_target = Some(map_name.clone());
                }
            }

            if let Some(target) = load_target {
                self.selected = Some(target.clone());
                world.resource_mut::<MapLoadRequest>().map_to_load = Some(target);
            }
        });
    }

    fn needs_world(&self) -> bool {
        true
    }

    fn default_visible(&self) -> bool {
        true
    }
}
