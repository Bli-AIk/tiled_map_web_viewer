use bevy_workbench::i18n::I18n;
use std::sync::{Arc, RwLock};

#[derive(Clone, Default)]
pub(crate) struct Translations {
    pub(crate) allow_dev_windows: String,
    pub(crate) zoom_sensitivity: String,
    pub(crate) pan_sensitivity: String,
    pub(crate) developer_label: String,
    pub(crate) sensitivity_label: String,
    pub(crate) map_sections_label: String,
    pub(crate) settings_visible_sections_hint: String,
    pub(crate) map_list: String,
    pub(crate) map_preview: String,
    pub(crate) map_details: String,
    pub(crate) list_loading_maps: String,
    pub(crate) list_no_maps: String,
    pub(crate) list_other_group: String,
    pub(crate) list_maps_group: String,
    pub(crate) list_worlds_group: String,
    pub(crate) details_no_selection: String,
    pub(crate) details_path: String,
    pub(crate) details_kind: String,
    pub(crate) details_section: String,
    pub(crate) details_category: String,
    pub(crate) details_badges: String,
    pub(crate) render_settings: String,
    pub(crate) render_background: String,
    pub(crate) render_background_hint: String,
    pub(crate) render_preview_grid: String,
    pub(crate) render_preview_grid_color: String,
    pub(crate) render_preview_grid_hint: String,
    pub(crate) render_android_grid_warning: String,
    pub(crate) render_world_grid: String,
    pub(crate) render_world_grid_color: String,
    pub(crate) render_world_grid_hint: String,
}

impl Translations {
    pub(crate) fn from_i18n(i18n: &I18n) -> Self {
        Self {
            allow_dev_windows: i18n.t("settings-allow-dev-windows"),
            zoom_sensitivity: i18n.t("settings-zoom-sensitivity"),
            pan_sensitivity: i18n.t("settings-pan-sensitivity"),
            developer_label: i18n.t("settings-developer"),
            sensitivity_label: i18n.t("settings-sensitivity"),
            map_sections_label: i18n.t("settings-map-sections"),
            settings_visible_sections_hint: i18n.t("settings-visible-sections-hint"),
            map_list: i18n.t("panel-map-list"),
            map_preview: i18n.t("panel-map-preview"),
            map_details: i18n.t("panel-map-details"),
            list_loading_maps: i18n.t("list-loading-maps"),
            list_no_maps: i18n.t("list-no-maps"),
            list_other_group: i18n.t("list-other-group"),
            list_maps_group: i18n.t("list-maps-group"),
            list_worlds_group: i18n.t("list-worlds-group"),
            details_no_selection: i18n.t("details-no-selection"),
            details_path: i18n.t("details-path"),
            details_kind: i18n.t("details-kind"),
            details_section: i18n.t("details-section"),
            details_category: i18n.t("details-category"),
            details_badges: i18n.t("details-badges"),
            render_settings: i18n.t("panel-render-settings"),
            render_background: i18n.t("render-background"),
            render_background_hint: i18n.t("render-background-hint"),
            render_preview_grid: i18n.t("render-preview-grid"),
            render_preview_grid_color: i18n.t("render-preview-grid-color"),
            render_preview_grid_hint: i18n.t("render-preview-grid-hint"),
            render_android_grid_warning: i18n.t("render-android-grid-warning"),
            render_world_grid: i18n.t("render-world-grid"),
            render_world_grid_color: i18n.t("render-world-grid-color"),
            render_world_grid_hint: i18n.t("render-world-grid-hint"),
        }
    }
}

pub(crate) type SharedTranslations = Arc<RwLock<Translations>>;
