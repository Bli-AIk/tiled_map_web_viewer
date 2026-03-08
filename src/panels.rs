use bevy::prelude::*;
use bevy_workbench::dock::WorkbenchPanel;

use crate::{MapCategory, MapLoadRequest, SharedTranslations};

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

// --- Map List Panel ---

#[derive(Default)]
pub(crate) struct MapListPanel {
    pub(crate) translations: SharedTranslations,
    pub(crate) categories: Vec<MapCategory>,
    maps: Vec<String>,
    scanned: bool,
    selected: Option<String>,
}

impl MapListPanel {
    pub(crate) fn new(translations: SharedTranslations, categories: Vec<MapCategory>) -> Self {
        Self {
            translations,
            categories,
            ..Default::default()
        }
    }

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

    /// Returns maps grouped by category. The last group contains uncategorized maps.
    fn grouped_maps(&self) -> Vec<(&str, Vec<&str>)> {
        let mut groups: Vec<(&str, Vec<&str>)> = self
            .categories
            .iter()
            .map(|c| (c.name.as_str(), Vec::new()))
            .collect();
        let mut uncategorized: Vec<&str> = Vec::new();

        for map in &self.maps {
            let mut found = false;
            for (i, cat) in self.categories.iter().enumerate() {
                if map.starts_with(&cat.directory) && map[cat.directory.len()..].starts_with('/') {
                    groups[i].1.push(map.as_str());
                    found = true;
                    break;
                }
            }
            if !found {
                uncategorized.push(map.as_str());
            }
        }

        if !uncategorized.is_empty() {
            groups.push(("Other", uncategorized));
        }

        groups
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

            if self.categories.is_empty() {
                // Flat list
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
            } else {
                // Grouped by category
                let groups = self.grouped_maps();
                for (group_name, maps) in &groups {
                    if maps.is_empty() {
                        continue;
                    }
                    let header = format!("{} ({})", group_name, maps.len());
                    egui::CollapsingHeader::new(header)
                        .default_open(false)
                        .show(ui, |ui| {
                            for map_name in maps {
                                let is_selected = self.selected.as_deref() == Some(*map_name);
                                // Show just the filename, not the full path
                                let display_name = map_name.rsplit('/').next().unwrap_or(map_name);
                                let text = if is_selected {
                                    egui::RichText::new(display_name).strong()
                                } else {
                                    egui::RichText::new(display_name)
                                };
                                if ui.selectable_label(is_selected, text).clicked() && !is_selected
                                {
                                    load_target = Some(map_name.to_string());
                                }
                            }
                        });
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
