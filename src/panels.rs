use bevy::prelude::*;
use bevy_workbench::dock::WorkbenchPanel;

use crate::{
    MapCategory, MapLoadRequest, MapManifest, MapManifestEntry, MapSection, SectionVisibilityState,
    SelectedMapDetails, SharedTranslations,
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
        "map_details_inspector"
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

        ui.heading(selected.display_title());
        ui.separator();
        details_row(ui, &t.details_path, &selected.path);
        if let Some(section) = &selected.section {
            details_row(ui, &t.details_section, section);
        }
        if let Some(category) = &selected.category {
            details_row(ui, &t.details_category, category);
        }

        if !selected.badges.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(&t.details_badges).strong());
            ui.horizontal_wrapped(|ui| {
                for badge in &selected.badges {
                    render_badge(ui, &badge.label, badge_color(badge.tone.as_deref()));
                }
            });
        }

        if !selected.details.is_empty() {
            ui.separator();
            for detail in &selected.details {
                details_row(ui, &detail.label, &detail.value);
            }
        }
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

#[derive(Default)]
pub(crate) struct MapListPanel {
    pub(crate) translations: SharedTranslations,
    pub(crate) sections: Vec<MapSection>,
    pub(crate) categories: Vec<MapCategory>,
    manifest_path: String,
    maps: Vec<MapManifestEntry>,
    scanned: bool,
    selected: Option<String>,
}

impl MapListPanel {
    pub(crate) fn new(
        translations: SharedTranslations,
        sections: Vec<MapSection>,
        categories: Vec<MapCategory>,
        manifest_path: String,
    ) -> Self {
        Self {
            translations,
            sections,
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
            if let Some(mut overlay) = world.get_resource_mut::<crate::WebLoadingOverlayState>() {
                overlay.show_with(&self.translations, |t| t.list_loading_maps.clone(), 0.45);
            }
            self.scan_maps();
            if let Some(mut overlay) = world.get_resource_mut::<crate::WebLoadingOverlayState>() {
                overlay.finish();
            }
        }

        let visible_sections = world.resource::<SectionVisibilityState>().0.clone();

        let Ok(t) = self.translations.read() else {
            ui.label("Translations unavailable");
            return;
        };
        let map_list = t.map_list.clone();
        let list_loading_maps = t.list_loading_maps.clone();
        let list_no_maps = t.list_no_maps.clone();
        let list_other_group = t.list_other_group.clone();
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

        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut load_target: Option<MapManifestEntry> = None;

            if self.sections.is_empty() {
                render_category_groups(
                    ui,
                    grouped_maps_for_categories(
                        &self.categories,
                        self.maps.iter().collect(),
                        &list_other_group,
                    ),
                    &mut self.selected,
                    &mut load_target,
                );
            } else {
                for section in &self.sections {
                    if !visible_sections.get(&section.key).copied().unwrap_or(false) {
                        continue;
                    }
                    let entries: Vec<&MapManifestEntry> = self
                        .maps
                        .iter()
                        .filter(|entry| entry.section.as_deref() == Some(section.key.as_str()))
                        .collect();
                    if entries.is_empty() {
                        continue;
                    }
                    let total = entries.len();
                    ui.heading(format!("{} ({total})", section.name));
                    render_category_groups(
                        ui,
                        grouped_maps_for_categories(&self.categories, entries, &list_other_group),
                        &mut self.selected,
                        &mut load_target,
                    );
                    ui.separator();
                }

                let uncategorized_sections: Vec<&MapManifestEntry> = self
                    .maps
                    .iter()
                    .filter(|entry| entry.section.is_none())
                    .collect();
                if !uncategorized_sections.is_empty() {
                    ui.heading(format!(
                        "{} ({})",
                        list_other_group,
                        uncategorized_sections.len()
                    ));
                    render_category_groups(
                        ui,
                        grouped_maps_for_categories(
                            &self.categories,
                            uncategorized_sections,
                            &list_other_group,
                        ),
                        &mut self.selected,
                        &mut load_target,
                    );
                }
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

fn render_category_groups(
    ui: &mut egui::Ui,
    groups: Vec<(String, Vec<&MapManifestEntry>)>,
    selected: &mut Option<String>,
    load_target: &mut Option<MapManifestEntry>,
) {
    for (group_name, maps) in groups {
        if maps.is_empty() {
            continue;
        }
        let header = format!("{} ({})", group_name, maps.len());
        egui::CollapsingHeader::new(header)
            .default_open(false)
            .show(ui, |ui| {
                for entry in maps {
                    render_map_entry(ui, entry, selected, load_target);
                }
            });
    }
}

fn render_map_entry(
    ui: &mut egui::Ui,
    entry: &MapManifestEntry,
    selected: &Option<String>,
    load_target: &mut Option<MapManifestEntry>,
) {
    let is_selected = selected.as_deref() == Some(entry.path.as_str());
    ui.horizontal_wrapped(|ui| {
        let response = ui.selectable_label(is_selected, entry.display_title());
        if response.clicked() && !is_selected {
            *load_target = Some(entry.clone());
        }
        for badge in &entry.badges {
            render_badge(ui, &badge.label, badge_color(badge.tone.as_deref()));
        }
    });
}

fn render_badge(ui: &mut egui::Ui, label: &str, color: egui::Color32) {
    ui.label(
        egui::RichText::new(format!(" {label} "))
            .background_color(color.gamma_multiply(0.2))
            .color(color)
            .strong(),
    );
}

fn badge_color(tone: Option<&str>) -> egui::Color32 {
    match tone {
        Some("success") => egui::Color32::from_rgb(80, 190, 120),
        Some("info") => egui::Color32::from_rgb(80, 170, 220),
        Some("warning") => egui::Color32::from_rgb(235, 180, 70),
        Some("danger") => egui::Color32::from_rgb(230, 100, 100),
        Some("muted") => egui::Color32::from_rgb(150, 150, 150),
        Some("accent") => egui::Color32::from_rgb(180, 120, 230),
        _ => egui::Color32::LIGHT_GRAY,
    }
}

fn grouped_maps_for_categories<'a>(
    categories: &'a [MapCategory],
    entries: Vec<&'a MapManifestEntry>,
    other_label: &str,
) -> Vec<(String, Vec<&'a MapManifestEntry>)> {
    if categories.is_empty() {
        return vec![(other_label.to_string(), entries)];
    }

    let mut groups: Vec<(String, Vec<&MapManifestEntry>)> = categories
        .iter()
        .map(|category| (category.name.clone(), Vec::new()))
        .collect();
    let mut uncategorized: Vec<&MapManifestEntry> = Vec::new();

    for entry in entries {
        let mut found = false;
        for (index, category) in categories.iter().enumerate() {
            if entry.category.as_deref() == Some(category.key.as_str()) {
                groups[index].1.push(entry);
                found = true;
                break;
            }
        }
        if !found {
            uncategorized.push(entry);
        }
    }

    if !uncategorized.is_empty() {
        groups.push((other_label.to_string(), uncategorized));
    }

    groups
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
    let title = std::path::Path::new(&normalized)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&normalized)
        .to_string();

    MapManifestEntry {
        path: normalized,
        title,
        ..Default::default()
    }
}
