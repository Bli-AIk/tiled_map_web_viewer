use bevy::asset::RecursiveDependencyLoadState;
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy_ecs_tiled::prelude::*;
use bevy_egui::{EguiContexts, EguiTextureHandle};
use bevy_workbench::console::console_log_layer;
use bevy_workbench::dock::WorkbenchPanel;
use bevy_workbench::i18n::{I18n, Locale};
use bevy_workbench::prelude::*;

use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Shared translation cache for closures/panels without World access.
#[derive(Clone, Default)]
#[allow(dead_code)]
struct Translations {
    // Settings
    allow_dev_windows: String,
    zoom_sensitivity: String,
    pan_sensitivity: String,
    developer_label: String,
    sensitivity_label: String,
    // Dev menu
    dev_windows_menu: String,
    inspector: String,
    console: String,
    // Panel titles
    map_list: String,
    map_preview: String,
    // Loading states
    loading_cleanup: String,
    loading_textures: String,
    loading_spawning: String,
}

impl Translations {
    fn from_i18n(i18n: &I18n) -> Self {
        Self {
            allow_dev_windows: i18n.t("settings-allow-dev-windows"),
            zoom_sensitivity: i18n.t("settings-zoom-sensitivity"),
            pan_sensitivity: i18n.t("settings-pan-sensitivity"),
            developer_label: i18n.t("settings-developer"),
            sensitivity_label: i18n.t("settings-sensitivity"),
            dev_windows_menu: i18n.t("menu-dev-windows"),
            inspector: i18n.t("menu-dev-inspector"),
            console: i18n.t("menu-dev-console"),
            map_list: i18n.t("panel-map-list"),
            map_preview: i18n.t("panel-map-preview"),
            loading_cleanup: i18n.t("loading-cleanup"),
            loading_textures: i18n.t("loading-textures"),
            loading_spawning: i18n.t("loading-spawning"),
        }
    }
}

type SharedTranslations = Arc<RwLock<Translations>>;

// --- Resources ---

/// Request to load a specific map file.
#[derive(Resource, Default)]
struct MapLoadRequest {
    map_to_load: Option<String>,
}

/// Tracks map loading progress.
#[derive(Resource, Default)]
struct MapLoadingState {
    phase: LoadPhase,
    current_map: Option<String>,
    /// Asset handle for tracking load progress.
    pending_handle: Option<Handle<TiledMapAsset>>,
    /// Status text for display.
    status_text: String,
}

#[derive(Default, PartialEq)]
enum LoadPhase {
    #[default]
    Idle,
    /// Old map is being despawned, waiting one frame before loading new.
    Cleanup,
    /// Asset and dependencies are loading asynchronously.
    LoadingAssets,
    /// Assets loaded, entities being spawned by bevy_ecs_tiled.
    Spawning,
}

/// Holds the render target for the map preview.
#[derive(Resource)]
struct MapPreviewState {
    render_target: Handle<Image>,
    egui_texture_id: Option<egui::TextureId>,
    width: u32,
    height: u32,
}

/// Input events from the preview panel, consumed by camera systems.
#[derive(Resource, Default)]
struct PreviewInput {
    /// Scroll delta from mouse wheel (positive = zoom in).
    scroll_delta: f32,
    /// Drag delta in panel-local pixels (for panning).
    drag_delta: egui::Vec2,
    /// Whether the preview image is hovered.
    hovered: bool,
    /// Cursor position as UV within the render target [0..1].
    cursor_uv: Option<egui::Pos2>,
    /// Size of the displayed image in screen pixels.
    image_screen_size: egui::Vec2,
}

/// Camera zoom state for smooth interpolation.
#[derive(Resource)]
struct CameraZoomState {
    current_scale: f32,
    target_scale: f32,
}

impl Default for CameraZoomState {
    fn default() -> Self {
        Self {
            current_scale: 4.0,
            target_scale: 4.0,
        }
    }
}

/// Whether the developer windows menu is enabled.
#[derive(Resource, Default)]
struct DevWindowsEnabled(bool);

/// Scroll sensitivity configuration.
#[derive(Resource)]
struct ScrollSensitivity {
    /// Zoom sensitivity multiplier (applied to scroll delta).
    zoom: f32,
    /// Pan sensitivity multiplier (applied to drag delta).
    pan: f32,
}

impl Default for ScrollSensitivity {
    fn default() -> Self {
        Self {
            zoom: 0.01,
            pan: 1.0,
        }
    }
}

/// Marker for the preview camera that renders maps to texture.
#[derive(Component)]
struct PreviewCamera;

fn main() {
    let mut app = App::new();

    // Shared toggle for the settings checkbox ↔ DevWindowsEnabled resource
    let dev_toggle = Arc::new(AtomicBool::new(false));
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Tiled Map Web Viewer".into(),
                    resolution: (1280u32, 720u32).into(),
                    canvas: Some("#the_canvas_id".to_string()),
                    fit_canvas_to_parent: true,
                    prevent_default_event_handling: true,
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
    .add_plugins(TiledPlugin::default());

    // Register locale FTL sources with bevy_workbench I18n
    {
        let mut i18n = app.world_mut().resource_mut::<I18n>();
        i18n.add_custom_source(Locale::En, include_str!("../locales/en.ftl"));
        i18n.add_custom_source(Locale::ZhCn, include_str!("../locales/zh-CN.ftl"));
    }

    // Build initial shared translations from current locale
    let shared_translations: SharedTranslations = {
        let i18n = app.world().resource::<I18n>();
        Arc::new(RwLock::new(Translations::from_i18n(i18n)))
    };

    app.init_resource::<MapLoadRequest>()
        .init_resource::<MapLoadingState>()
        .init_resource::<PreviewInput>()
        .init_resource::<CameraZoomState>()
        .init_resource::<DevWindowsEnabled>()
        .init_resource::<ScrollSensitivity>()
        .init_resource::<MenuBarExtensions>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_map_load,
                track_map_loading,
                sync_preview_to_panel,
                apply_camera_zoom,
                apply_camera_pan,
                handle_dev_menu_actions,
            ),
        )
        .add_systems(
            bevy_egui::EguiPrimaryContextPass,
            sync_dev_menu.before(bevy_workbench::menu_bar::menu_bar_system),
        );

    // Register panels with shared translations for dynamic titles
    let t_for_list = shared_translations.clone();
    app.register_panel(MapListPanel {
        translations: t_for_list,
        ..default()
    });
    let t_for_preview = shared_translations.clone();
    app.register_panel(MapPreviewPanel {
        translations: t_for_preview,
        ..default()
    });

    // Hide Inspector and Console from the Window menu and default layout
    {
        let mut tile_state = app
            .world_mut()
            .resource_mut::<bevy_workbench::dock::TileLayoutState>();
        tile_state.hide_from_window_menu("workbench_inspector");
        tile_state.hide_from_window_menu("workbench_console");
        tile_state.set_default_hidden("workbench_inspector");
        tile_state.set_default_hidden("workbench_console");
    }

    // Settings: Developer section
    let toggle_for_settings = dev_toggle.clone();
    let t_for_dev_section = shared_translations.clone();
    app.register_settings_section(SettingsSection {
        label: shared_translations.read().unwrap().developer_label.clone(),
        ui_fn: Box::new(move |ui| {
            let label = t_for_dev_section.read().unwrap().allow_dev_windows.clone();
            let mut val = toggle_for_settings.load(Ordering::Relaxed);
            if ui.checkbox(&mut val, label).changed() {
                toggle_for_settings.store(val, Ordering::Relaxed);
            }
        }),
    });

    // Settings: Sensitivity section
    let zoom_sens = Arc::new(AtomicU32::new(0.01_f32.to_bits()));
    let pan_sens = Arc::new(AtomicU32::new(1.0_f32.to_bits()));
    let zoom_for_settings = zoom_sens.clone();
    let pan_for_settings = pan_sens.clone();
    let t_for_sens_section = shared_translations.clone();
    app.register_settings_section(SettingsSection {
        label: shared_translations
            .read()
            .unwrap()
            .sensitivity_label
            .clone(),
        ui_fn: Box::new(move |ui| {
            let t = t_for_sens_section.read().unwrap();
            let mut zoom_val = f32::from_bits(zoom_for_settings.load(Ordering::Relaxed));
            if ui
                .add(egui::Slider::new(&mut zoom_val, 0.001..=0.05).text(&t.zoom_sensitivity))
                .changed()
            {
                zoom_for_settings.store(zoom_val.to_bits(), Ordering::Relaxed);
            }

            let mut pan_val = f32::from_bits(pan_for_settings.load(Ordering::Relaxed));
            if ui
                .add(egui::Slider::new(&mut pan_val, 0.1..=3.0).text(&t.pan_sensitivity))
                .changed()
            {
                pan_for_settings.store(pan_val.to_bits(), Ordering::Relaxed);
            }
        }),
    });

    // System to sync atomic values → resources and update translations on locale change
    let toggle_for_system = dev_toggle.clone();
    let zoom_for_system = zoom_sens.clone();
    let pan_for_system = pan_sens.clone();
    let t_for_sync = shared_translations.clone();
    app.add_systems(
        Update,
        move |mut dev_enabled: ResMut<DevWindowsEnabled>,
              mut sensitivity: ResMut<ScrollSensitivity>,
              i18n: Res<I18n>| {
            dev_enabled.0 = toggle_for_system.load(Ordering::Relaxed);
            sensitivity.zoom = f32::from_bits(zoom_for_system.load(Ordering::Relaxed));
            sensitivity.pan = f32::from_bits(pan_for_system.load(Ordering::Relaxed));

            // Refresh translations if locale changed
            if i18n.is_changed()
                && let Ok(mut t) = t_for_sync.write()
            {
                *t = Translations::from_i18n(&i18n);
            }
        },
    );

    app.run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let width = 1920u32;
    let height = 1080u32;

    let image = Image::new_target_texture(width, height, TextureFormat::Rgba8UnormSrgb, None);
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
        PreviewCamera,
    ));

    commands.insert_resource(MapPreviewState {
        render_target,
        egui_texture_id: None,
        width,
        height,
    });
}

/// Syncs the "Developer Windows" custom menu to the menu bar.
fn sync_dev_menu(
    dev_enabled: Res<DevWindowsEnabled>,
    i18n: Res<I18n>,
    mut extensions: ResMut<MenuBarExtensions>,
    tile_state: Res<bevy_workbench::dock::TileLayoutState>,
) {
    let inspector_visible = tile_state.is_panel_visible("workbench_inspector");
    let console_visible = tile_state.is_panel_visible("workbench_console");

    let inspector_label = i18n.t("menu-dev-inspector");
    let console_label = i18n.t("menu-dev-console");

    extensions.custom_menus = vec![CustomMenu {
        id: "dev_windows",
        label: i18n.t("menu-dev-windows"),
        enabled: dev_enabled.0,
        items: vec![
            MenuExtItem {
                id: "toggle_inspector",
                label: if inspector_visible {
                    format!("✓ {inspector_label}")
                } else {
                    format!("  {inspector_label}")
                },
                enabled: true,
            },
            MenuExtItem {
                id: "toggle_console",
                label: if console_visible {
                    format!("✓ {console_label}")
                } else {
                    format!("  {console_label}")
                },
                enabled: true,
            },
        ],
    }];
}

/// Handles developer window menu actions (toggle inspector/console).
fn handle_dev_menu_actions(
    mut menu_actions: MessageReader<MenuAction>,
    mut tile_state: ResMut<bevy_workbench::dock::TileLayoutState>,
) {
    for action in menu_actions.read() {
        match action.id {
            "toggle_inspector" => {
                if tile_state.is_panel_visible("workbench_inspector") {
                    tile_state.close_panel("workbench_inspector");
                } else {
                    tile_state.request_open_panel("workbench_inspector");
                }
            }
            "toggle_console" => {
                if tile_state.is_panel_visible("workbench_console") {
                    tile_state.close_panel("workbench_console");
                } else {
                    tile_state.request_open_panel("workbench_console");
                }
            }
            _ => {}
        }
    }
}

/// Phase 1: Accept load request, despawn old map, enter Cleanup phase.
fn handle_map_load(
    mut commands: Commands,
    mut load_request: ResMut<MapLoadRequest>,
    mut loading: ResMut<MapLoadingState>,
    i18n: Res<I18n>,
    existing_maps: Query<Entity, With<TiledMap>>,
) {
    let Some(map_name) = load_request.map_to_load.take() else {
        return;
    };

    // Despawn existing map entities
    for entity in &existing_maps {
        commands.entity(entity).despawn();
    }

    loading.phase = LoadPhase::Cleanup;
    loading.current_map = Some(map_name);
    loading.pending_handle = None;
    loading.status_text = i18n.t("loading-cleanup");
}

/// Phase 2+: Drive the loading state machine.
fn track_map_loading(
    mut commands: Commands,
    mut loading: ResMut<MapLoadingState>,
    asset_server: Res<AssetServer>,
    i18n: Res<I18n>,
    maps: Query<&Children, With<TiledMap>>,
) {
    match loading.phase {
        LoadPhase::Idle => {}
        LoadPhase::Cleanup => {
            // Wait one frame after despawn, then start loading
            if let Some(ref map_name) = loading.current_map.clone() {
                let handle: Handle<TiledMapAsset> = asset_server.load(map_name);
                loading.pending_handle = Some(handle);
                loading.phase = LoadPhase::LoadingAssets;
                loading.status_text = i18n.t("loading-textures");
                info!("Loading map: {}", map_name);
            }
        }
        LoadPhase::LoadingAssets => {
            if let Some(ref handle) = loading.pending_handle {
                let load_state = asset_server.recursive_dependency_load_state(handle);
                match load_state {
                    RecursiveDependencyLoadState::Loaded => {
                        // All assets ready — spawn the map entity
                        commands.spawn((TiledMap(handle.clone()), TilemapAnchor::Center));
                        loading.phase = LoadPhase::Spawning;
                        loading.status_text = i18n.t("loading-spawning");
                    }
                    RecursiveDependencyLoadState::Failed(_) => {
                        error!("Failed to load map assets");
                        loading.phase = LoadPhase::Idle;
                        loading.status_text.clear();
                    }
                    _ => {
                        // Still loading
                        loading.status_text = i18n.t("loading-textures");
                    }
                }
            }
        }
        LoadPhase::Spawning => {
            // Map entity spawning complete when children (layers) appear
            for children in &maps {
                if !children.is_empty() {
                    loading.phase = LoadPhase::Idle;
                    loading.status_text.clear();
                    loading.pending_handle = None;
                }
            }
        }
    }
}

/// Smooth zoom centered on cursor position.
fn apply_camera_zoom(
    mut zoom_state: ResMut<CameraZoomState>,
    input: Res<PreviewInput>,
    preview: Res<MapPreviewState>,
    sensitivity: Res<ScrollSensitivity>,
    time: Res<Time>,
    mut camera_q: Query<(&mut Transform, &mut Projection), With<PreviewCamera>>,
) {
    let Ok((mut transform, mut projection)) = camera_q.single_mut() else {
        return;
    };
    let Projection::Orthographic(ref mut ortho) = *projection else {
        return;
    };

    // Apply scroll delta to target scale
    if input.scroll_delta.abs() > 0.001 {
        let zoom_factor = 1.0 - input.scroll_delta * sensitivity.zoom;
        zoom_state.target_scale = (zoom_state.target_scale * zoom_factor).clamp(0.2, 30.0);
    }

    // Smooth interpolation toward target
    let lerp_speed = 12.0;
    let dt = time.delta_secs();
    let old_scale = zoom_state.current_scale;
    zoom_state.current_scale +=
        (zoom_state.target_scale - zoom_state.current_scale) * (1.0 - (-lerp_speed * dt).exp());
    ortho.scale = zoom_state.current_scale;

    // Zoom centered on cursor: adjust camera position so the world point
    // under the cursor stays fixed.
    if let Some(uv) = input.cursor_uv
        && (old_scale - zoom_state.current_scale).abs() > 0.0001
    {
        let rt_w = preview.width as f32;
        let rt_h = preview.height as f32;
        // Cursor offset from center of viewport in NDC-like coords [-0.5, 0.5]
        let cx = uv.x - 0.5;
        let cy = -(uv.y - 0.5); // Flip Y (screen Y is down, world Y is up)

        let world_offset_x = cx * rt_w * old_scale;
        let world_offset_y = cy * rt_h * old_scale;

        let new_world_offset_x = cx * rt_w * zoom_state.current_scale;
        let new_world_offset_y = cy * rt_h * zoom_state.current_scale;

        transform.translation.x += world_offset_x - new_world_offset_x;
        transform.translation.y += world_offset_y - new_world_offset_y;
    }
}

/// Pan camera by dragging (middle mouse button or right mouse button).
fn apply_camera_pan(
    mut input: ResMut<PreviewInput>,
    zoom_state: Res<CameraZoomState>,
    sensitivity: Res<ScrollSensitivity>,
    preview: Res<MapPreviewState>,
    mut camera_q: Query<&mut Transform, With<PreviewCamera>>,
) {
    let Ok(mut transform) = camera_q.single_mut() else {
        return;
    };

    if input.drag_delta.length_sq() > 0.001 {
        // Convert panel-pixel drag to world units.
        let img_w = input.image_screen_size.x.max(1.0);
        let img_h = input.image_screen_size.y.max(1.0);
        let scale_x = preview.width as f32 / img_w * zoom_state.current_scale;
        let scale_y = preview.height as f32 / img_h * zoom_state.current_scale;

        transform.translation.x -= input.drag_delta.x * scale_x * sensitivity.pan;
        transform.translation.y += input.drag_delta.y * scale_y * sensitivity.pan; // Flip Y
    }

    // Reset per-frame input
    input.scroll_delta = 0.0;
    input.drag_delta = egui::Vec2::ZERO;
}

/// Syncs render target texture to the preview panel, and reads back input.
fn sync_preview_to_panel(
    mut state: ResMut<MapPreviewState>,
    mut contexts: EguiContexts,
    mut tile_state: ResMut<bevy_workbench::dock::TileLayoutState>,
    loading: Res<MapLoadingState>,
    mut input: ResMut<PreviewInput>,
    mut images: ResMut<Assets<Image>>,
) {
    if state.egui_texture_id.is_none() && state.render_target != Handle::default() {
        let texture_id = contexts.add_image(EguiTextureHandle::Strong(state.render_target.clone()));
        state.egui_texture_id = Some(texture_id);
    }

    if let Some(panel) = tile_state.get_panel_mut::<MapPreviewPanel>("map_preview") {
        panel.egui_texture_id = state.egui_texture_id;
        panel.is_loading = loading.phase != LoadPhase::Idle;
        panel.loading_status = loading.status_text.clone();

        // Resize render target if panel size changed significantly
        let panel_w = (panel.panel_size.x as u32).max(2);
        let panel_h = (panel.panel_size.y as u32).max(2);
        if panel_w > 0
            && panel_h > 0
            && (panel_w != state.width || panel_h != state.height)
            && panel.panel_size.x > 10.0
        {
            let new_image =
                Image::new_target_texture(panel_w, panel_h, TextureFormat::Rgba8UnormSrgb, None);
            if let Some(img) = images.get_mut(&state.render_target) {
                *img = new_image;
            }
            state.width = panel_w;
            state.height = panel_h;
        }

        panel.width = state.width;
        panel.height = state.height;

        // Read input accumulated by the panel
        input.scroll_delta += panel.pending_scroll;
        input.drag_delta += panel.pending_drag;
        input.hovered = panel.is_hovered;
        input.cursor_uv = panel.cursor_uv;
        input.image_screen_size = panel.image_screen_size;

        // Clear panel's pending input
        panel.pending_scroll = 0.0;
        panel.pending_drag = egui::Vec2::ZERO;
    }
}

// --- Map Preview Panel ---

#[derive(Default)]
struct MapPreviewPanel {
    translations: SharedTranslations,
    egui_texture_id: Option<egui::TextureId>,
    width: u32,
    height: u32,
    is_loading: bool,
    loading_status: String,
    // Input state written by UI, read by systems
    pending_scroll: f32,
    pending_drag: egui::Vec2,
    is_hovered: bool,
    cursor_uv: Option<egui::Pos2>,
    image_screen_size: egui::Vec2,
    /// Panel's available size last frame, for render target resizing.
    panel_size: egui::Vec2,
}

impl WorkbenchPanel for MapPreviewPanel {
    fn id(&self) -> &str {
        "map_preview"
    }

    fn title(&self) -> String {
        self.translations
            .read()
            .map(|t| t.map_preview.clone())
            .unwrap_or_else(|_| "Map Preview".into())
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let Some(tex_id) = self.egui_texture_id else {
            ui.centered_and_justified(|ui| {
                ui.label("Select a map from the Map List panel");
            });
            return;
        };

        let avail = ui.available_size();
        if avail.x <= 0.0 || avail.y <= 0.0 {
            return;
        }
        self.panel_size = avail;

        // Fill the entire available area with the preview
        let display_size = avail;
        self.image_screen_size = display_size;

        // Allocate the image area as a sense rect for input handling
        let (response, painter) = ui.allocate_painter(display_size, egui::Sense::click_and_drag());
        let rect = response.rect;

        // Draw the preview texture
        painter.image(
            tex_id,
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );

        // Track hover state and cursor UV
        self.is_hovered = response.hovered();
        if self.is_hovered {
            if let Some(pos) = response.hover_pos() {
                let uv_x = (pos.x - rect.left()) / rect.width();
                let uv_y = (pos.y - rect.top()) / rect.height();
                self.cursor_uv = Some(egui::pos2(uv_x, uv_y));
            }
        } else {
            self.cursor_uv = None;
        }

        // Scroll wheel → zoom
        if self.is_hovered {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll.abs() > 0.1 {
                self.pending_scroll += scroll;
            }
        }

        // Middle mouse or right mouse drag → pan
        if response.dragged_by(egui::PointerButton::Middle)
            || response.dragged_by(egui::PointerButton::Secondary)
        {
            self.pending_drag += response.drag_delta();
        }

        // Loading overlay
        if self.is_loading {
            let overlay_color = egui::Color32::from_rgba_unmultiplied(40, 40, 40, 180);
            painter.rect_filled(rect, 0.0, overlay_color);

            // Spinning circle
            let center = rect.center();
            let radius = 20.0;
            let t = ui.input(|i| i.time) as f32;
            let segments = 8;
            for i in 0..segments {
                let angle_start = t * 3.0 + (i as f32 / segments as f32) * std::f32::consts::TAU;
                let angle_end = angle_start + 0.3;
                let alpha = ((i as f32 / segments as f32) * 255.0) as u8;
                let color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha);
                let p1 =
                    center + egui::vec2(angle_start.cos() * radius, angle_start.sin() * radius);
                let p2 = center + egui::vec2(angle_end.cos() * radius, angle_end.sin() * radius);
                painter.line_segment([p1, p2], egui::Stroke::new(3.0, color));
            }

            painter.text(
                center + egui::vec2(0.0, radius + 16.0),
                egui::Align2::CENTER_TOP,
                &self.loading_status,
                egui::FontId::proportional(14.0),
                egui::Color32::WHITE,
            );

            // Request repaint for animation
            ui.ctx().request_repaint();
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
    translations: SharedTranslations,
    maps: Vec<String>,
    scanned: bool,
    selected: Option<String>,
}

impl MapListPanel {
    #[cfg(not(target_arch = "wasm32"))]
    fn scan_maps(&mut self) {
        let assets_dir = std::path::Path::new("assets");
        if assets_dir.exists() {
            self.walk_dir(assets_dir, assets_dir);
        }
        self.maps.sort();
        self.scanned = true;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn walk_dir(&mut self, dir: &std::path::Path, base: &std::path::Path) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                self.walk_dir(&path, base);
            } else if path.extension().is_some_and(|ext| ext == "tmx")
                && let Ok(rel) = path.strip_prefix(base)
            {
                self.maps.push(rel.to_string_lossy().to_string());
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn scan_maps(&mut self) {
        // In WASM, fetch manifest.txt via synchronous XHR
        let Ok(xhr) = web_sys::XmlHttpRequest::new() else {
            self.scanned = true;
            return;
        };
        if xhr
            .open_with_async("GET", "assets/manifest.txt", false)
            .is_err()
        {
            self.scanned = true;
            return;
        }
        if xhr.send().is_err() {
            self.scanned = true;
            return;
        }
        if let Ok(Some(text)) = xhr.response_text() {
            self.maps = text
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.to_string())
                .collect();
            self.maps.sort();
        }
        self.scanned = true;
    }
}

impl WorkbenchPanel for MapListPanel {
    fn id(&self) -> &str {
        "map_list"
    }

    fn title(&self) -> String {
        self.translations
            .read()
            .map(|t| t.map_list.clone())
            .unwrap_or_else(|_| "Map List".into())
    }

    fn ui(&mut self, _ui: &mut egui::Ui) {}

    fn ui_world(&mut self, ui: &mut egui::Ui, world: &mut World) {
        if !self.scanned {
            self.scan_maps();
        }

        ui.heading("Maps");
        ui.separator();

        if !self.scanned {
            ui.spinner();
            ui.label("Loading map list...");
            return;
        }

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
