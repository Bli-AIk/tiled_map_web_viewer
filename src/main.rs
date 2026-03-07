use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy_ecs_tiled::prelude::*;
use bevy_egui::{EguiContexts, EguiTextureHandle};
use bevy_workbench::console::console_log_layer;
use bevy_workbench::dock::WorkbenchPanel;
use bevy_workbench::game_view::ViewZoom;
use bevy_workbench::prelude::*;

/// Request to load a specific map file.
#[derive(Resource, Default)]
struct MapLoadRequest {
    map_to_load: Option<String>,
}

/// Holds the render target for the map preview.
#[derive(Resource)]
struct MapPreviewState {
    render_target: Handle<Image>,
    egui_texture_id: Option<egui::TextureId>,
    width: u32,
    height: u32,
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
    .add_plugins(WorkbenchPlugin {
        config: WorkbenchConfig {
            show_toolbar: false,
            enable_game_view: false,
            ..default()
        },
    })
    .add_plugins(TiledPlugin::default())
    .init_resource::<MapLoadRequest>()
    .add_systems(Startup, setup)
    .add_systems(Update, (handle_map_load, sync_preview_to_panel));

    app.register_panel(MapListPanel::default());
    app.register_panel(MapPreviewPanel::default());

    app.run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let width = 1920u32;
    let height = 1080u32;

    let image = Image::new_target_texture(
        width,
        height,
        TextureFormat::Rgba8UnormSrgb,
        None,
    );
    let render_target = images.add(image);

    // Window camera for egui panels
    commands.spawn(Camera2d);

    // Map preview camera — renders to texture
    commands.spawn((
        Camera2d,
        Camera {
            order: -1,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.1, 0.1, 0.15)),
            ..default()
        },
        RenderTarget::from(render_target.clone()),
    ));

    commands.insert_resource(MapPreviewState {
        render_target,
        egui_texture_id: None,
        width,
        height,
    });
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

/// Syncs render target texture to the preview panel.
fn sync_preview_to_panel(
    mut state: ResMut<MapPreviewState>,
    mut contexts: EguiContexts,
    mut tile_state: ResMut<bevy_workbench::dock::TileLayoutState>,
) {
    if state.egui_texture_id.is_none() && state.render_target != Handle::default() {
        let texture_id =
            contexts.add_image(EguiTextureHandle::Strong(state.render_target.clone()));
        state.egui_texture_id = Some(texture_id);
    }

    if let Some(panel) = tile_state.get_panel_mut::<MapPreviewPanel>("map_preview") {
        panel.egui_texture_id = state.egui_texture_id;
        panel.width = state.width;
        panel.height = state.height;
    }
}

// --- Map Preview Panel ---

#[derive(Default)]
struct MapPreviewPanel {
    egui_texture_id: Option<egui::TextureId>,
    width: u32,
    height: u32,
    zoom: ViewZoom,
}

impl WorkbenchPanel for MapPreviewPanel {
    fn id(&self) -> &str {
        "map_preview"
    }

    fn title(&self) -> String {
        "Map Preview".into()
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let Some(tex_id) = self.egui_texture_id else {
            ui.centered_and_justified(|ui| {
                ui.label("Select a map from the Map List panel");
            });
            return;
        };

        // Zoom toolbar
        ui.horizontal(|ui| {
            let zoom_label = match self.zoom {
                ViewZoom::Auto => "Auto".to_string(),
                ViewZoom::Fixed(z) => format!("{:.0}%", z * 100.0),
            };
            egui::ComboBox::from_id_salt("map_zoom")
                .selected_text(&zoom_label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.zoom, ViewZoom::Auto, "Auto");
                    ui.selectable_value(&mut self.zoom, ViewZoom::Fixed(0.25), "25%");
                    ui.selectable_value(&mut self.zoom, ViewZoom::Fixed(0.5), "50%");
                    ui.selectable_value(&mut self.zoom, ViewZoom::Fixed(0.75), "75%");
                    ui.selectable_value(&mut self.zoom, ViewZoom::Fixed(1.0), "100%");
                    ui.selectable_value(&mut self.zoom, ViewZoom::Fixed(1.5), "150%");
                    ui.selectable_value(&mut self.zoom, ViewZoom::Fixed(2.0), "200%");
                });
        });
        ui.separator();

        let avail = ui.available_size();
        if avail.x <= 0.0 || avail.y <= 0.0 {
            return;
        }

        let aspect = self.width as f32 / self.height.max(1) as f32;

        let display_size = match self.zoom {
            ViewZoom::Auto => {
                let w = avail.x;
                let h = w / aspect;
                if h > avail.y {
                    egui::vec2(avail.y * aspect, avail.y)
                } else {
                    egui::vec2(w, h)
                }
            }
            ViewZoom::Fixed(z) => egui::vec2(self.width as f32 * z, self.height as f32 * z),
        };

        let padding = (avail - display_size).max(egui::Vec2::ZERO) * 0.5;

        if matches!(self.zoom, ViewZoom::Fixed(_))
            && (display_size.x > avail.x || display_size.y > avail.y)
        {
            egui::ScrollArea::both().show(ui, |ui| {
                ui.image(egui::load::SizedTexture::new(tex_id, display_size));
            });
        } else {
            ui.add_space(padding.y);
            ui.vertical_centered(|ui| {
                ui.image(egui::load::SizedTexture::new(tex_id, display_size));
            });
        }
    }

    fn closable(&self) -> bool {
        false
    }

    fn default_visible(&self) -> bool {
        true
    }
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
