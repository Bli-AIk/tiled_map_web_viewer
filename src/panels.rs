use bevy::prelude::*;
use bevy_workbench::dock::WorkbenchPanel;

use crate::{
    MapCategory, MapListView, MapLoadRequest, MapManifest, MapManifestEntry, MapSection,
    SectionVisibilityState, SelectedMapDetails, SharedTranslations,
    details_panel::{badge_color, render_badge},
    manifest::manifest_entry_from_path,
};

pub(crate) struct MapPreviewPanel {
    pub(crate) translations: SharedTranslations,
    pub(crate) egui_texture_id: Option<egui::TextureId>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) is_loading: bool,
    pub(crate) loading_status: String,
    pub(crate) pending_scroll: f32,
    pub(crate) pending_zoom_factor: f32,
    pub(crate) pending_drag: egui::Vec2,
    pub(crate) is_hovered: bool,
    pub(crate) cursor_uv: Option<egui::Pos2>,
    pub(crate) image_screen_size: egui::Vec2,
    pub(crate) panel_size: egui::Vec2,
}

impl Default for MapPreviewPanel {
    fn default() -> Self {
        Self {
            translations: SharedTranslations::default(),
            egui_texture_id: None,
            width: 0,
            height: 0,
            is_loading: false,
            loading_status: String::new(),
            pending_scroll: 0.0,
            pending_zoom_factor: 1.0,
            pending_drag: egui::Vec2::ZERO,
            is_hovered: false,
            cursor_uv: None,
            image_screen_size: egui::Vec2::ZERO,
            panel_size: egui::Vec2::ZERO,
        }
    }
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
                ui.label("Select a map or world from the Map List panel");
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

        let touch_info = ui.input(|i| i.multi_touch());
        let gesture_zoom = ui.input(|i| i.zoom_delta());
        let raw_scroll = ui.input(|i| i.raw_scroll_delta.y);
        let touch_center = touch_info.map(|touch| touch.center_pos);
        let touch_center = touch_center.filter(|pos| rect.contains(*pos));

        painter.image(
            tex_id,
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );

        self.is_hovered =
            response.contains_pointer() || response.dragged() || touch_center.is_some();
        let pointer_pos = touch_center
            .or(ui.ctx().pointer_latest_pos())
            .or(response.interact_pointer_pos())
            .filter(|pos| rect.contains(*pos));
        if let Some(pos) = pointer_pos {
            let uv_x = (pos.x - rect.left()) / rect.width();
            let uv_y = (pos.y - rect.top()) / rect.height();
            self.cursor_uv = Some(egui::pos2(uv_x, uv_y));
        } else {
            self.cursor_uv = None;
        }

        if self.is_hovered && raw_scroll.abs() > 0.1 {
            self.pending_scroll += raw_scroll;
        }

        if self.is_hovered && (gesture_zoom - 1.0).abs() > 0.001 {
            self.pending_zoom_factor *= gesture_zoom.max(0.01);
        }

        if response.dragged_by(egui::PointerButton::Primary)
            || response.dragged_by(egui::PointerButton::Middle)
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
pub(crate) struct MapListPanel {
    pub(crate) translations: SharedTranslations,
    pub(crate) sections: Vec<MapSection>,
    pub(crate) categories: Vec<MapCategory>,
    list: MapListView,
    manifest_path: String,
    maps: Vec<MapManifestEntry>,
    scanned: bool,
    selected: Option<String>,
    search_query: String,
}

impl MapListPanel {
    pub(crate) fn new(
        translations: SharedTranslations,
        sections: Vec<MapSection>,
        categories: Vec<MapCategory>,
        manifest_path: String,
        list: MapListView,
    ) -> Self {
        Self {
            translations,
            sections,
            categories,
            list,
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
                .map(manifest_entry_from_path)
                .collect();
        }
    }
}

impl WorkbenchPanel for MapListPanel {
    fn id(&self) -> &str {
        &self.list.id
    }

    fn title(&self) -> String {
        self.list.title.clone()
    }

    fn ui(&mut self, _ui: &mut egui::Ui) {}

    fn ui_world(&mut self, ui: &mut egui::Ui, world: &mut World) {
        if !self.scanned {
            self.scan_maps();
        }

        let visible_sections = world.resource::<SectionVisibilityState>().0.clone();

        let Ok(t) = self.translations.read() else {
            ui.label("Translations unavailable");
            return;
        };
        let list_loading_maps = t.list_loading_maps.clone();
        let list_no_maps = t.list_no_maps.clone();
        let list_other_group = t.list_other_group.clone();
        let list_maps_group = t.list_maps_group.clone();
        let list_worlds_group = t.list_worlds_group.clone();
        let list_search_label = t.list_search_label.clone();
        let list_search_hint = t.list_search_hint.clone();
        drop(t);

        ui.heading(self.title());
        ui.horizontal(|ui| {
            ui.label(&list_search_label);
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text(&list_search_hint)
                    .desired_width(f32::INFINITY),
            );
        });
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

            if let Some(section_filter) = self.list.section_filter.as_deref() {
                let entries: Vec<&MapManifestEntry> = self
                    .maps
                    .iter()
                    .filter(|entry| entry.section.as_deref() == Some(section_filter))
                    .filter(|entry| map_entry_matches_search(entry, &self.search_query))
                    .collect();
                if entries.is_empty() {
                    ui.label(&list_no_maps);
                    return;
                }
                render_category_groups(
                    ui,
                    grouped_maps_for_categories(
                        &self.categories,
                        entries,
                        &list_other_group,
                        &list_maps_group,
                        &list_worlds_group,
                    ),
                    &mut self.selected,
                    &mut load_target,
                );
            } else if self.sections.is_empty() {
                render_category_groups(
                    ui,
                    grouped_maps_for_categories(
                        &self.categories,
                        self.maps
                            .iter()
                            .filter(|entry| map_entry_matches_search(entry, &self.search_query))
                            .collect(),
                        &list_other_group,
                        &list_maps_group,
                        &list_worlds_group,
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
                        .filter(|entry| map_entry_matches_search(entry, &self.search_query))
                        .collect();
                    if entries.is_empty() {
                        continue;
                    }
                    let total = entries.len();
                    ui.heading(format!("{} ({total})", section.name));
                    render_category_groups(
                        ui,
                        grouped_maps_for_categories(
                            &self.categories,
                            entries,
                            &list_other_group,
                            &list_maps_group,
                            &list_worlds_group,
                        ),
                        &mut self.selected,
                        &mut load_target,
                    );
                    ui.separator();
                }

                let uncategorized_sections: Vec<&MapManifestEntry> = self
                    .maps
                    .iter()
                    .filter(|entry| entry.section.is_none())
                    .filter(|entry| map_entry_matches_search(entry, &self.search_query))
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
                            &list_maps_group,
                            &list_worlds_group,
                        ),
                        &mut self.selected,
                        &mut load_target,
                    );
                }
            }

            if let Some(target) = load_target {
                self.selected = Some(target.path.clone());
                world.resource_mut::<MapLoadRequest>().entry_to_load = Some(target.clone());
                world.resource_mut::<SelectedMapDetails>().0 = Some(target);
            }
        });
    }

    fn needs_world(&self) -> bool {
        true
    }

    fn default_visible(&self) -> bool {
        self.list.default_visible
    }
}

fn map_entry_matches_search(entry: &MapManifestEntry, query: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }

    let query = query.to_ascii_lowercase();
    entry.display_title().to_ascii_lowercase().contains(&query)
        || entry.path.to_ascii_lowercase().contains(&query)
        || entry
            .category
            .as_deref()
            .is_some_and(|category| category.to_ascii_lowercase().contains(&query))
        || entry
            .section
            .as_deref()
            .is_some_and(|section| section.to_ascii_lowercase().contains(&query))
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

fn grouped_maps_for_categories<'a>(
    categories: &'a [MapCategory],
    entries: Vec<&'a MapManifestEntry>,
    other_label: &str,
    maps_label: &str,
    worlds_label: &str,
) -> Vec<(String, Vec<&'a MapManifestEntry>)> {
    if categories.is_empty() {
        let mut maps_group = Vec::new();
        let mut worlds_group = Vec::new();

        for entry in entries {
            match entry.asset_kind() {
                crate::MapAssetKind::Map => maps_group.push(entry),
                crate::MapAssetKind::World => worlds_group.push(entry),
            }
        }

        let mut groups = Vec::new();
        if !maps_group.is_empty() {
            groups.push((maps_label.to_string(), maps_group));
        }
        if !worlds_group.is_empty() {
            groups.push((worlds_label.to_string(), worlds_group));
        }
        if groups.is_empty() {
            groups.push((other_label.to_string(), Vec::new()));
        }
        return groups;
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
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "tmx" | "world"))
            && let Ok(rel) = path.strip_prefix(base)
        {
            maps.push(manifest_entry_from_path(rel.to_string_lossy().as_ref()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::map_entry_matches_search;
    use crate::MapManifestEntry;

    #[test]
    fn empty_query_matches_every_entry() {
        let entry = MapManifestEntry {
            title: "room_tundra8".into(),
            path: "curated/undertale/room_tundra8.tmx".into(),
            ..Default::default()
        };
        assert!(map_entry_matches_search(&entry, ""));
        assert!(map_entry_matches_search(&entry, "   "));
    }

    #[test]
    fn search_matches_title_and_path_case_insensitively() {
        let entry = MapManifestEntry {
            title: "Dark Sanctuary".into(),
            path: "curated/worlds/deltarune/deltarune_ch4/dark_sanctuary.world".into(),
            ..Default::default()
        };
        assert!(map_entry_matches_search(&entry, "sanctuary"));
        assert!(map_entry_matches_search(&entry, "CURATED/WORLDS"));
        assert!(!map_entry_matches_search(&entry, "waterfall"));
    }

    #[test]
    fn search_matches_section_and_category() {
        let entry = MapManifestEntry {
            title: "room_torhouse".into(),
            path: "curated/undertale/room_torhouse.tmx".into(),
            section: Some("curated".into()),
            category: Some("worlds".into()),
            ..Default::default()
        };
        assert!(map_entry_matches_search(&entry, "curated"));
        assert!(map_entry_matches_search(&entry, "worlds"));
    }
}
