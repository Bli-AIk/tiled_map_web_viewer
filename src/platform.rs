#[cfg(target_arch = "wasm32")]
use bevy::prelude::Local;

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

pub(crate) fn default_asset_file_path() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        "assets".into()
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(root) = std::env::var("BEVY_ASSET_ROOT") {
            return root;
        }

        let manifest_assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
        if manifest_assets.exists() {
            return manifest_assets.to_string_lossy().into_owned();
        }

        "assets".into()
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn notify_web_loader_ready(mut notified: Local<bool>) {
    if *notified {
        return;
    }

    let Some(window) = web_sys::window() else {
        return;
    };

    let Ok(event) = web_sys::Event::new("bevy-app-ready") else {
        return;
    };

    if window.dispatch_event(&event).is_ok() {
        *notified = true;
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn notify_web_loader_ready() {}

#[cfg(target_arch = "wasm32")]
pub(crate) fn initial_window_resolution(requested: (u32, u32)) -> (u32, u32) {
    let Some(window) = web_sys::window() else {
        return requested;
    };

    let viewport_width = window
        .inner_width()
        .ok()
        .and_then(|value| value.as_f64())
        .map(|value| value.max(1.0).floor() as u32);
    let viewport_height = window
        .inner_height()
        .ok()
        .and_then(|value| value.as_f64())
        .map(|value| value.max(1.0).floor() as u32);

    match (viewport_width, viewport_height) {
        (Some(width), Some(height)) => (requested.0.min(width), requested.1.min(height)),
        _ => requested,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn initial_window_resolution(requested: (u32, u32)) -> (u32, u32) {
    requested
}
