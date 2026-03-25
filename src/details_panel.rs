use bevy::prelude::*;
use bevy_workbench::dock::WorkbenchPanel;

use crate::{
    SelectedMapDetails, SharedTranslations,
    download::{AssetRootPath, DownloadUiState, PendingDownloadReceiver, request_download},
};

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
        let details_no_selection = t.details_no_selection.clone();
        let details_download = t.details_download.clone();
        let details_download_hint = t.details_download_hint.clone();
        let details_download_busy = t.details_download_busy.clone();
        let details_path = t.details_path.clone();
        let details_kind = t.details_kind.clone();
        let details_section = t.details_section.clone();
        let details_category = t.details_category.clone();
        let details_badges = t.details_badges.clone();
        drop(t);

        let Some(selected) = maybe_selected else {
            ui.label(&details_no_selection);
            return;
        };

        let is_busy = world.resource::<DownloadUiState>().is_busy;
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.add_space(6.0);
            let button = egui::Button::new(if is_busy {
                &details_download_busy
            } else {
                &details_download
            });
            if ui.add_enabled(!is_busy, button).clicked() {
                let asset_root = world.resource::<AssetRootPath>().0.clone();
                world.resource_scope(|world, mut state: Mut<DownloadUiState>| {
                    let pending = world.resource::<PendingDownloadReceiver>();
                    request_download(asset_root, selected.clone(), &mut state, pending);
                });
            }
            ui.label(&details_download_hint);
            ui.add_space(6.0);
        });
        ui.add_space(6.0);

        if let Some(status) = &world.resource::<DownloadUiState>().last_status {
            let color = if status.is_error {
                egui::Color32::from_rgb(230, 100, 100)
            } else {
                egui::Color32::from_rgb(120, 200, 120)
            };
            ui.label(egui::RichText::new(&status.message).color(color));
            ui.separator();
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.heading(selected.display_title());
                ui.separator();
                details_row(ui, &details_path, &selected.path);
                details_row(ui, &details_kind, selected.asset_kind().label());
                if let Some(section) = &selected.section {
                    details_row(ui, &details_section, section);
                }
                if let Some(category) = &selected.category {
                    details_row(ui, &details_category, category);
                }

                if !selected.badges.is_empty() {
                    ui.separator();
                    ui.label(egui::RichText::new(&details_badges).strong());
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
            });
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

pub(crate) fn render_badge(ui: &mut egui::Ui, label: &str, color: egui::Color32) {
    ui.label(
        egui::RichText::new(format!(" {label} "))
            .background_color(color.gamma_multiply(0.2))
            .color(color)
            .strong(),
    );
}

pub(crate) fn badge_color(tone: Option<&str>) -> egui::Color32 {
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
