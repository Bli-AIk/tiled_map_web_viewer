use bevy::prelude::*;

use crate::{SharedTranslations, Translations};

#[derive(Resource)]
pub(crate) struct WebLoadingOverlayState {
    visible: bool,
    status_text: String,
    progress: f32,
    dirty: bool,
}

impl WebLoadingOverlayState {
    pub(crate) fn new(status_text: String) -> Self {
        Self {
            visible: true,
            status_text,
            progress: 0.12,
            dirty: true,
        }
    }

    pub(crate) fn show_with(
        &mut self,
        translations: &SharedTranslations,
        text: impl FnOnce(&Translations) -> String,
        progress: f32,
    ) {
        let status_text = translations
            .read()
            .map(|t| text(&t))
            .unwrap_or_else(|_| self.status_text.clone());
        self.visible = true;
        self.status_text = status_text;
        self.progress = progress.clamp(0.0, 1.0);
        self.dirty = true;
    }

    pub(crate) fn finish(&mut self) {
        self.progress = 1.0;
        self.visible = false;
        self.dirty = true;
    }
}

pub(crate) fn install(status_text: &str, progress: f32) {
    install_web_loading_overlay(status_text, progress);
}

pub(crate) fn sync_overlay(mut overlay: ResMut<WebLoadingOverlayState>) {
    if !overlay.dirty {
        return;
    }

    if overlay.visible {
        update_web_loading_overlay(&overlay.status_text, overlay.progress);
    } else {
        remove_web_loading_overlay();
    }
    overlay.dirty = false;
}

#[cfg(target_arch = "wasm32")]
const WEB_LOADING_OVERLAY_ID: &str = "tmwv-loading-overlay";
#[cfg(target_arch = "wasm32")]
const WEB_LOADING_OVERLAY_STATUS_ID: &str = "tmwv-loading-overlay-status";
#[cfg(target_arch = "wasm32")]
const WEB_LOADING_OVERLAY_BAR_ID: &str = "tmwv-loading-overlay-bar";

#[cfg(target_arch = "wasm32")]
fn install_web_loading_overlay(status_text: &str, progress: f32) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(body) = document.body() else {
        return;
    };

    if document.get_element_by_id(WEB_LOADING_OVERLAY_ID).is_none() {
        let Ok(overlay) = document.create_element("div") else {
            return;
        };
        overlay.set_id(WEB_LOADING_OVERLAY_ID);
        let _ = overlay.set_attribute(
            "style",
            concat!(
                "position:fixed;inset:0;z-index:99999;display:flex;align-items:center;",
                "justify-content:center;background:linear-gradient(180deg,#070a12 0%,#0a1020 100%);",
                "color:#f4f7fb;font-family:system-ui,-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;"
            ),
        );
        overlay.set_inner_html(
            "<div style=\"width:min(420px,calc(100vw - 48px));padding:24px 26px;border:1px solid rgba(255,255,255,0.1);border-radius:16px;background:rgba(12,18,32,0.92);box-shadow:0 20px 60px rgba(0,0,0,0.45);\">\
                <div style=\"font-size:12px;letter-spacing:0.12em;text-transform:uppercase;color:#8eb3ff;margin-bottom:10px;\">Tiled Map Web Viewer</div>\
                <div id=\"tmwv-loading-overlay-status\" style=\"font-size:18px;font-weight:600;line-height:1.4;\">Loading viewer...</div>\
                <div style=\"margin-top:16px;height:10px;border-radius:999px;overflow:hidden;background:rgba(255,255,255,0.08);\">\
                    <div id=\"tmwv-loading-overlay-bar\" style=\"height:100%;width:12%;border-radius:999px;background:linear-gradient(90deg,#66b3ff 0%,#9bdbff 100%);transition:width 180ms ease;\"></div>\
                </div>\
            </div>",
        );
        let _ = body.append_child(&overlay);
    }

    update_web_loading_overlay(status_text, progress);
}

#[cfg(not(target_arch = "wasm32"))]
fn install_web_loading_overlay(_status_text: &str, _progress: f32) {}

#[cfg(target_arch = "wasm32")]
fn update_web_loading_overlay(status_text: &str, progress: f32) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };

    if document.get_element_by_id(WEB_LOADING_OVERLAY_ID).is_none() {
        install_web_loading_overlay(status_text, progress);
        return;
    }

    if let Some(status) = document.get_element_by_id(WEB_LOADING_OVERLAY_STATUS_ID) {
        status.set_inner_html(status_text);
    }

    if let Some(bar) = document.get_element_by_id(WEB_LOADING_OVERLAY_BAR_ID) {
        let percent = (progress.clamp(0.0, 1.0) * 100.0).round();
        let _ = bar.set_attribute(
            "style",
            &format!(
                "height:100%;width:{percent}%;border-radius:999px;background:linear-gradient(90deg,#66b3ff 0%,#9bdbff 100%);transition:width 180ms ease;"
            ),
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn update_web_loading_overlay(_status_text: &str, _progress: f32) {}

#[cfg(target_arch = "wasm32")]
fn remove_web_loading_overlay() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(overlay) = document.get_element_by_id(WEB_LOADING_OVERLAY_ID) else {
        return;
    };
    overlay.remove();
}

#[cfg(not(target_arch = "wasm32"))]
fn remove_web_loading_overlay() {}
