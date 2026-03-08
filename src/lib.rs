use bevy::asset::{AssetMetaCheck, RecursiveDependencyLoadState};
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy_ecs_tiled::prelude::*;
use bevy_egui::{EguiContexts, EguiTextureHandle};
use bevy_workbench::console::console_log_layer;
use bevy_workbench::i18n::{I18n, Locale};
use bevy_workbench::prelude::*;

use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

mod panels;

use panels::{MapListPanel, MapPreviewPanel};

// --- Public API ---

/// A named group of maps shown as a collapsible section in the map list panel.
#[derive(Clone, Debug)]
pub struct MapCategory {
    /// Display name (e.g. "Undertale", "Deltarune Ch1").
    pub name: String,
    /// Directory prefix under `assets/` that maps in this category share
    /// (e.g. "undertale", "deltarune_ch1").
    pub directory: String,
}

/// Top-level configuration for the viewer application.
pub struct ViewerConfig {
    /// Window title.
    pub title: String,
    /// Initial window resolution (width, height).
    pub resolution: (u32, u32),
    /// Map categories. When non-empty the map list panel groups maps
    /// under collapsible headers. When empty, all maps are shown in a flat list.
    pub categories: Vec<MapCategory>,
    /// Additional Fluent locale sources `(locale, ftl_content)` to register.
    pub locale_sources: Vec<(Locale, &'static str)>,
}

impl Default for ViewerConfig {
    fn default() -> Self {
        Self {
            title: "Tiled Map Web Viewer".into(),
            resolution: (1280, 720),
            categories: vec![],
            locale_sources: vec![],
        }
    }
}

/// Entry point — builds and runs the Bevy application with the given configuration.
pub fn run(config: ViewerConfig) {
    let mut app = App::new();

    let dev_toggle = Arc::new(AtomicBool::new(false));
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                meta_check: AssetMetaCheck::Never,
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: config.title.clone(),
                    resolution: (config.resolution.0, config.resolution.1).into(),
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

    // Register built-in locale FTL sources
    {
        let mut i18n = app.world_mut().resource_mut::<I18n>();
        i18n.add_custom_source(Locale::En, include_str!("../locales/en.ftl"));
        i18n.add_custom_source(Locale::ZhCn, include_str!("../locales/zh-CN.ftl"));
        for (locale, ftl) in &config.locale_sources {
            i18n.add_custom_source(*locale, *ftl);
        }
    }

    let shared_translations: SharedTranslations = {
        let i18n = app.world().resource::<I18n>();
        Arc::new(RwLock::new(Translations::from_i18n(i18n)))
    };

    // Store categories as a resource for the map list panel
    let categories = config.categories.clone();

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

    // Register panels
    let t_for_list = shared_translations.clone();
    app.register_panel(MapListPanel::new(t_for_list, categories));
    let t_for_preview = shared_translations.clone();
    app.register_panel(MapPreviewPanel::new(t_for_preview));

    // Hide Inspector and Console
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

            if i18n.is_changed()
                && let Ok(mut t) = t_for_sync.write()
            {
                *t = Translations::from_i18n(&i18n);
            }
        },
    );

    app.run();
}

// --- Internal types ---

#[derive(Clone, Default)]
#[allow(dead_code)]
struct Translations {
    allow_dev_windows: String,
    zoom_sensitivity: String,
    pan_sensitivity: String,
    developer_label: String,
    sensitivity_label: String,
    dev_windows_menu: String,
    inspector: String,
    console: String,
    map_list: String,
    map_preview: String,
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

pub(crate) type SharedTranslations = Arc<RwLock<Translations>>;

#[derive(Resource, Default)]
struct MapLoadRequest {
    map_to_load: Option<String>,
}

#[derive(Resource, Default)]
struct MapLoadingState {
    phase: LoadPhase,
    current_map: Option<String>,
    pending_handle: Option<Handle<TiledMapAsset>>,
    status_text: String,
}

#[derive(Default, PartialEq)]
enum LoadPhase {
    #[default]
    Idle,
    Cleanup,
    LoadingAssets,
    Spawning,
}

#[derive(Resource)]
struct MapPreviewState {
    render_target: Handle<Image>,
    egui_texture_id: Option<egui::TextureId>,
    width: u32,
    height: u32,
}

#[derive(Resource, Default)]
struct PreviewInput {
    scroll_delta: f32,
    drag_delta: egui::Vec2,
    #[allow(dead_code)]
    hovered: bool,
    cursor_uv: Option<egui::Pos2>,
    image_screen_size: egui::Vec2,
}

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

#[derive(Resource, Default)]
struct DevWindowsEnabled(bool);

#[derive(Resource)]
struct ScrollSensitivity {
    zoom: f32,
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

#[derive(Component)]
struct PreviewCamera;

// --- Systems ---

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let width = 1920u32;
    let height = 1080u32;

    let image = Image::new_target_texture(width, height, TextureFormat::Rgba8UnormSrgb, None);
    let render_target = images.add(image);

    commands.spawn(Camera2d);
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

    for entity in &existing_maps {
        commands.entity(entity).despawn();
    }

    loading.phase = LoadPhase::Cleanup;
    loading.current_map = Some(map_name);
    loading.pending_handle = None;
    loading.status_text = i18n.t("loading-cleanup");
}

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
                        loading.status_text = i18n.t("loading-textures");
                    }
                }
            }
        }
        LoadPhase::Spawning => {
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

    if input.scroll_delta.abs() > 0.001 {
        let zoom_factor = 1.0 - input.scroll_delta * sensitivity.zoom;
        zoom_state.target_scale = (zoom_state.target_scale * zoom_factor).clamp(0.2, 30.0);
    }

    let lerp_speed = 12.0;
    let dt = time.delta_secs();
    let old_scale = zoom_state.current_scale;
    zoom_state.current_scale +=
        (zoom_state.target_scale - zoom_state.current_scale) * (1.0 - (-lerp_speed * dt).exp());
    ortho.scale = zoom_state.current_scale;

    if let Some(uv) = input.cursor_uv
        && (old_scale - zoom_state.current_scale).abs() > 0.0001
    {
        let rt_w = preview.width as f32;
        let rt_h = preview.height as f32;
        let cx = uv.x - 0.5;
        let cy = -(uv.y - 0.5);

        let world_offset_x = cx * rt_w * old_scale;
        let world_offset_y = cy * rt_h * old_scale;
        let new_world_offset_x = cx * rt_w * zoom_state.current_scale;
        let new_world_offset_y = cy * rt_h * zoom_state.current_scale;

        transform.translation.x += world_offset_x - new_world_offset_x;
        transform.translation.y += world_offset_y - new_world_offset_y;
    }
}

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
        let img_w = input.image_screen_size.x.max(1.0);
        let img_h = input.image_screen_size.y.max(1.0);
        let scale_x = preview.width as f32 / img_w * zoom_state.current_scale;
        let scale_y = preview.height as f32 / img_h * zoom_state.current_scale;

        transform.translation.x -= input.drag_delta.x * scale_x * sensitivity.pan;
        transform.translation.y += input.drag_delta.y * scale_y * sensitivity.pan;
    }

    input.scroll_delta = 0.0;
    input.drag_delta = egui::Vec2::ZERO;
}

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

        input.scroll_delta += panel.pending_scroll;
        input.drag_delta += panel.pending_drag;
        input.hovered = panel.is_hovered;
        input.cursor_uv = panel.cursor_uv;
        input.image_screen_size = panel.image_screen_size;

        panel.pending_scroll = 0.0;
        panel.pending_drag = egui::Vec2::ZERO;
    }
}
