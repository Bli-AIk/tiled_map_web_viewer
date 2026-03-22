use bevy::prelude::*;
use bevy_workbench::dock::WorkbenchPanel;

use crate::{
    MapCategory, MapLoadRequest, MapManifest, MapManifestEntry, SelectedMapDetails,
    SharedTranslations, ShowRawMapsEnabled,
};

#[derive(Default)]
pub(crate) struct MapPreviewPanel {
    pub(crate) translations: SharedTranslations,
    pub(crate) egui_texture_id: Option<egui::TextureId>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) is_loading: bool,
    pub(crate) loading_status: String,
    pub(crate) pending_scroll: f32,
    pub(crate) pending_drag: egui::Vec2,
    pub(crate) is_hovered: bool,
    pub(crate) cursor_uv: Option<egui::Pos2>,
    pub(crate) image_screen_size: egui::Vec2,
    pub(crate) panel_size: egui::Vec2,
}

impl MapPreviewPanel {
    pub(crate) fn new(translations: SharedTranslations) -> Self {
        Self {
            translations,
            ..Default::default()
        }
    }
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

        let display_size = avail;
        self.image_screen_size = display_size;

        let (response, painter) = ui.allocate_painter(display_size, egui::Sense::click_and_drag());
        let rect = response.rect;

        painter.image(
            tex_id,
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );

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

        if self.is_hovered {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll.abs() > 0.1 {
                self.pending_scroll += scroll;
            }
        }

        if response.dragged_by(egui::PointerButton::Middle)
            || response.dragged_by(egui::PointerButton::Secondary)
        {
            self.pending_drag += response.drag_delta();
        }

        if self.is_loading {
            let overlay_color = egui::Color32::from_rgba_unmultiplied(40, 40, 40, 180);
            painter.rect_filled(rect, 0.0, overlay_color);

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

#[derive(Default)]
pub(crate) struct MapDetailsPanel {
    pub(crate) translations: SharedTranslations,
}

impl MapDetailsPanel {
    pub(crate) fn new(translations: SharedTranslations) -> Self {
        Self { translations }
    }
}

impl WorkbenchPanel for MapDetailsPanel {
    fn id(&self) -> &str {
        "map_details"
    }

    fn title(&self) -> String {
        self.translations
            .read()
            .map(|t| t.map_details.clone())
            .unwrap_or_else(|_| "Map Details".into())
    }

    fn ui(&mut self, _ui: &mut egui::Ui) {}

    fn ui_world(&mut self, ui: &mut egui::Ui, world: &mut World) {
        let maybe_selected = world.resource::<SelectedMapDetails>().0.clone();
        let Ok(t) = self.translations.read() else {
            ui.label("Translations unavailable");
            return;
        };

        let Some(selected) = maybe_selected else {
            ui.label(&t.details_no_selection);
            return;
        };

        ui.heading(&selected.room_name);
        ui.separator();
        details_row(ui, &t.details_path, &selected.path);
        details_row(ui, &t.details_source, &selected.source);
        details_row(ui, &t.details_dataset, &selected.dataset);
        details_row(
            ui,
            &t.details_visual_status,
            selected.visual_status.as_deref().unwrap_or("—"),
        );
        details_row(
            ui,
            &t.details_logic_status,
            selected.logic_status.as_deref().unwrap_or("—"),
        );
        details_row(
            ui,
            &t.details_scope,
            selected.scope.as_deref().unwrap_or("—"),
        );

        ui.separator();
        ui.label(egui::RichText::new(&t.details_notes).strong());
        ui.label(selected.notes.as_deref().unwrap_or("—"));
    }

    fn needs_world(&self) -> bool {
        true
    }

    fn default_visible(&self) -> bool {
        true
    }
}

fn details_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new(label).strong());
        ui.label(value);
    });
}

// --- Map List Panel ---

#[derive(Default)]
pub(crate) struct MapListPanel {
    pub(crate) translations: SharedTranslations,
    pub(crate) categories: Vec<MapCategory>,
    manifest_path: String,
    maps: Vec<MapManifestEntry>,
    scanned: bool,
    selected: Option<String>,
}

impl MapListPanel {
    pub(crate) fn new(
        translations: SharedTranslations,
        categories: Vec<MapCategory>,
        manifest_path: String,
    ) -> Self {
        Self {
            translations,
            categories,
            manifest_path,
            ..Default::default()
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scan_maps(&mut self) {
        if !self.load_manifest_from_json_native() {
            self.maps = self.scan_maps_from_assets_native();
        }
        self.maps.sort_by(|a, b| a.path.cmp(&b.path));
        self.scanned = true;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load_manifest_from_json_native(&mut self) -> bool {
        let Ok(text) = std::fs::read_to_string(&self.manifest_path) else {
            return false;
        };
        let Ok(manifest) = serde_json::from_str::<MapManifest>(&text) else {
            return false;
        };
        self.maps = manifest.maps;
        true
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scan_maps_from_assets_native(&self) -> Vec<MapManifestEntry> {
        let assets_dir = std::path::Path::new("assets");
        let mut maps = Vec::new();
        if assets_dir.exists() {
            walk_dir_collect(assets_dir, assets_dir, &mut maps);
        }
        maps
    }

    #[cfg(target_arch = "wasm32")]
    fn scan_maps(&mut self) {
        if !self.load_manifest_from_json_wasm() {
            self.load_manifest_from_txt_wasm();
        }
        self.maps.sort_by(|a, b| a.path.cmp(&b.path));
        self.scanned = true;
    }

    #[cfg(target_arch = "wasm32")]
    fn load_manifest_from_json_wasm(&mut self) -> bool {
        let Ok(xhr) = web_sys::XmlHttpRequest::new() else {
            return false;
        };
        if xhr
            .open_with_async("GET", &self.manifest_path, false)
            .is_err()
        {
            return false;
        }
        if xhr.send().is_err() {
            return false;
        }
        let Ok(Some(text)) = xhr.response_text() else {
            return false;
        };
        let Ok(manifest) = serde_json::from_str::<MapManifest>(&text) else {
            return false;
        };
        self.maps = manifest.maps;
        true
    }

    #[cfg(target_arch = "wasm32")]
    fn load_manifest_from_txt_wasm(&mut self) {
        let txt_path = self.manifest_path.replace(".json", ".txt");
        let Ok(xhr) = web_sys::XmlHttpRequest::new() else {
            return;
        };
        if xhr.open_with_async("GET", &txt_path, false).is_err() {
            return;
        }
        if xhr.send().is_err() {
            return;
        }
        if let Ok(Some(text)) = xhr.response_text() {
            self.maps = text
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(default_entry_from_path)
                .collect();
        }
    }
}

fn grouped_maps_for_categories<'a>(
    categories: &'a [MapCategory],
    entries: Vec<&'a MapManifestEntry>,
) -> Vec<(String, Vec<&'a MapManifestEntry>)> {
    let mut groups: Vec<(String, Vec<&MapManifestEntry>)> = categories
        .iter()
        .map(|c| (c.name.clone(), Vec::new()))
        .collect();
    let mut uncategorized: Vec<&MapManifestEntry> = Vec::new();

    for entry in entries {
        let mut found = false;
        for (i, cat) in categories.iter().enumerate() {
            if entry.dataset == cat.directory {
                groups[i].1.push(entry);
                found = true;
                break;
            }
        }
        if !found {
            uncategorized.push(entry);
        }
    }

    if !uncategorized.is_empty() {
        groups.push(("Other".into(), uncategorized));
    }

    groups
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

        let show_raw_maps = world.resource::<ShowRawMapsEnabled>().0;

        let Ok(t) = self.translations.read() else {
            ui.label("Translations unavailable");
            return;
        };
        let map_list = t.map_list.clone();
        let list_loading_maps = t.list_loading_maps.clone();
        let list_no_maps = t.list_no_maps.clone();
        let list_curated_maps = t.list_curated_maps.clone();
        let list_raw_maps = t.list_raw_maps.clone();
        let list_missing_metadata = t.list_missing_metadata.clone();
        drop(t);

        ui.heading(&map_list);
        ui.separator();

        if !self.scanned {
            ui.spinner();
            ui.label(&list_loading_maps);
            return;
        }

        if self.maps.is_empty() {
            ui.label(&list_no_maps);
            return;
        }

        let curated_entries: Vec<&MapManifestEntry> = self
            .maps
            .iter()
            .filter(|entry| entry.source == "curated")
            .collect();
        let raw_entries: Vec<&MapManifestEntry> = self
            .maps
            .iter()
            .filter(|entry| entry.source == "raw")
            .collect();
        let curated_groups = grouped_maps_for_categories(&self.categories, curated_entries);
        let raw_groups = grouped_maps_for_categories(&self.categories, raw_entries);

        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut load_target: Option<MapManifestEntry> = None;

            render_source_section(
                ui,
                &list_curated_maps,
                curated_groups.clone(),
                &mut self.selected,
                &mut load_target,
                &list_missing_metadata,
            );

            if show_raw_maps {
                ui.separator();
                render_source_section(
                    ui,
                    &list_raw_maps,
                    raw_groups.clone(),
                    &mut self.selected,
                    &mut load_target,
                    &list_missing_metadata,
                );
            }

            if let Some(target) = load_target {
                self.selected = Some(target.path.clone());
                world.resource_mut::<MapLoadRequest>().map_to_load = Some(target.path.clone());
                world.resource_mut::<SelectedMapDetails>().0 = Some(target);
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

fn render_source_section(
    ui: &mut egui::Ui,
    title: &str,
    groups: Vec<(String, Vec<&MapManifestEntry>)>,
    selected: &mut Option<String>,
    load_target: &mut Option<MapManifestEntry>,
    missing_metadata_label: &str,
) {
    let total: usize = groups.iter().map(|(_, entries)| entries.len()).sum();
    if total == 0 {
        return;
    }

    ui.heading(format!("{title} ({total})"));
    for (group_name, maps) in groups {
        if maps.is_empty() {
            continue;
        }
        let header = format!("{} ({})", group_name, maps.len());
        egui::CollapsingHeader::new(header)
            .default_open(false)
            .show(ui, |ui| {
                for entry in maps {
                    render_map_entry(ui, entry, selected, load_target, missing_metadata_label);
                }
            });
    }
}

fn render_map_entry(
    ui: &mut egui::Ui,
    entry: &MapManifestEntry,
    selected: &Option<String>,
    load_target: &mut Option<MapManifestEntry>,
    missing_metadata_label: &str,
) {
    let is_selected = selected.as_deref() == Some(entry.path.as_str());
    ui.horizontal_wrapped(|ui| {
        let response = ui.selectable_label(is_selected, &entry.room_name);
        if response.clicked() && !is_selected {
            *load_target = Some(entry.clone());
        }
        render_status_badge(ui, &entry.source, source_color(&entry.source));
        if let Some(status) = &entry.visual_status {
            render_status_badge(ui, status, visual_status_color(status));
        } else if entry.source == "curated" {
            render_status_badge(ui, missing_metadata_label, egui::Color32::RED);
        }
        if let Some(scope) = &entry.scope {
            render_status_badge(ui, scope, egui::Color32::from_rgb(140, 140, 160));
        }
    });
}

fn render_status_badge(ui: &mut egui::Ui, label: &str, color: egui::Color32) {
    ui.label(
        egui::RichText::new(format!(" {label} "))
            .background_color(color.gamma_multiply(0.2))
            .color(color)
            .strong(),
    );
}

fn source_color(source: &str) -> egui::Color32 {
    match source {
        "curated" => egui::Color32::from_rgb(80, 190, 120),
        "raw" => egui::Color32::from_rgb(140, 150, 165),
        _ => egui::Color32::LIGHT_GRAY,
    }
}

fn visual_status_color(status: &str) -> egui::Color32 {
    match status {
        "curated" => egui::Color32::from_rgb(80, 190, 120),
        "reviewed_clean" => egui::Color32::from_rgb(80, 170, 220),
        "seeded" => egui::Color32::from_rgb(220, 190, 80),
        "needs_work" => egui::Color32::from_rgb(235, 145, 60),
        "unreviewed" => egui::Color32::from_rgb(150, 150, 150),
        _ => egui::Color32::LIGHT_GRAY,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn walk_dir_collect(
    dir: &std::path::Path,
    base: &std::path::Path,
    maps: &mut Vec<MapManifestEntry>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_dir_collect(&path, base, maps);
        } else if path.extension().is_some_and(|ext| ext == "tmx")
            && let Ok(rel) = path.strip_prefix(base)
        {
            maps.push(default_entry_from_path(rel.to_string_lossy().as_ref()));
        }
    }
}

fn default_entry_from_path(path: &str) -> MapManifestEntry {
    let normalized = path.replace('\\', "/");
    let room_name = std::path::Path::new(&normalized)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&normalized)
        .to_string();

    MapManifestEntry {
        path: normalized.clone(),
        source: infer_source(&normalized).into(),
        dataset: infer_dataset(&normalized).into(),
        room_name,
        ..Default::default()
    }
}

fn infer_source(path: &str) -> &str {
    if path.starts_with("curated/") {
        "curated"
    } else {
        "raw"
    }
}

fn infer_dataset(path: &str) -> &str {
    let path = path
        .trim_start_matches("raw/")
        .trim_start_matches("curated/");
    if path.starts_with("undertale/") {
        "undertale"
    } else if path.starts_with("deltarune/deltarune_ch1/") || path.starts_with("deltarune_ch1/") {
        "deltarune_ch1"
    } else if path.starts_with("deltarune/deltarune_ch2/") || path.starts_with("deltarune_ch2/") {
        "deltarune_ch2"
    } else if path.starts_with("deltarune/deltarune_ch3/") || path.starts_with("deltarune_ch3/") {
        "deltarune_ch3"
    } else if path.starts_with("deltarune/deltarune_ch4/") || path.starts_with("deltarune_ch4/") {
        "deltarune_ch4"
    } else {
        "other"
    }
}
