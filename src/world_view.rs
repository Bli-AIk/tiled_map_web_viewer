use bevy::math::Rect;
use bevy::prelude::*;
use bevy_ecs_tiled::prelude::{TiledWorldAsset, TiledWorldChunking, TilemapAnchor};

use crate::{CameraZoomState, MapPreviewState, PreviewCamera};

const DEFAULT_MAP_SCALE: f32 = 4.0;
const MIN_INITIAL_WORLD_SCALE: f32 = 0.2;
const WORLD_SCALE_PADDING: f32 = 1.1;
const WORLD_CHUNK_PADDING: f32 = 1.1;
const MAX_INITIAL_WORLD_SCALE: f32 = 8.0;
const MAX_INITIAL_VISIBLE_MAPS: usize = 4;
const MAX_UNCHUNKED_WORLD_MAPS: usize = 48;
const MIN_WORLD_CHUNK_HALF_EXTENT: f32 = 512.0;

pub(crate) fn focus_preview_camera_for_map(
    preview_camera: &mut Query<&mut Transform, With<PreviewCamera>>,
    zoom_state: &mut ResMut<CameraZoomState>,
) {
    if let Ok(mut transform) = preview_camera.single_mut() {
        transform.translation.x = 0.0;
        transform.translation.y = 0.0;
    }
    zoom_state.current_scale = DEFAULT_MAP_SCALE;
    zoom_state.target_scale = DEFAULT_MAP_SCALE;
}

pub(crate) fn focus_preview_camera_for_world(
    preview_camera: &mut Query<&mut Transform, With<PreviewCamera>>,
    zoom_state: &mut ResMut<CameraZoomState>,
    preview: &MapPreviewState,
    tiled_world: &TiledWorldAsset,
) {
    let bounds = displayed_world_bounds(tiled_world, &TilemapAnchor::Center);
    let center = bounds.center();

    if let Ok(mut transform) = preview_camera.single_mut() {
        transform.translation.x = center.x;
        transform.translation.y = center.y;
    }

    let fitted_scale = fit_scale_for_bounds(bounds, preview);
    let initial_scale = if should_disable_chunking(tiled_world) {
        fitted_scale
    } else {
        limit_initial_scale_by_visible_maps(
            tiled_world,
            &TilemapAnchor::Center,
            center,
            preview,
            fitted_scale,
        )
    };
    zoom_state.current_scale = initial_scale;
    zoom_state.target_scale = initial_scale;
}

pub(crate) fn world_chunking_for_preview(
    tiled_world: &TiledWorldAsset,
    preview: &MapPreviewState,
    scale: f32,
) -> TiledWorldChunking {
    if should_disable_chunking(tiled_world) {
        return TiledWorldChunking(None);
    }

    let half_width =
        (preview.width as f32 * scale * 0.5 * WORLD_CHUNK_PADDING).max(MIN_WORLD_CHUNK_HALF_EXTENT);
    let half_height = (preview.height as f32 * scale * 0.5 * WORLD_CHUNK_PADDING)
        .max(MIN_WORLD_CHUNK_HALF_EXTENT);

    TiledWorldChunking::new(half_width, half_height)
}

fn should_disable_chunking(tiled_world: &TiledWorldAsset) -> bool {
    tiled_world.maps.len() <= MAX_UNCHUNKED_WORLD_MAPS
}

fn displayed_world_bounds(tiled_world: &TiledWorldAsset, anchor: &TilemapAnchor) -> Rect {
    let offset = world_anchor_offset(tiled_world, anchor);
    tiled_world
        .maps
        .iter()
        .map(|(rect, _)| {
            Rect::new(
                rect.min.x + offset.x,
                rect.min.y + offset.y,
                rect.max.x + offset.x,
                rect.max.y + offset.y,
            )
        })
        .reduce(|bounds, rect| bounds.union(rect))
        .unwrap_or(tiled_world.rect)
}

fn fit_scale_for_bounds(bounds: Rect, preview: &MapPreviewState) -> f32 {
    let size = bounds.size();
    let width = preview.width.max(1) as f32;
    let height = preview.height.max(1) as f32;
    let scale_x = size.x.abs() / width;
    let scale_y = size.y.abs() / height;

    (scale_x.max(scale_y) * WORLD_SCALE_PADDING)
        .clamp(MIN_INITIAL_WORLD_SCALE, MAX_INITIAL_WORLD_SCALE)
}

fn world_anchor_offset(tiled_world: &TiledWorldAsset, anchor: &TilemapAnchor) -> Vec2 {
    let min = tiled_world.rect.min;
    let max = tiled_world.rect.max;
    match anchor {
        TilemapAnchor::None => Vec2::ZERO,
        TilemapAnchor::TopLeft => Vec2::new(-min.x, -max.y),
        TilemapAnchor::TopRight => Vec2::new(-max.x, -max.y),
        TilemapAnchor::TopCenter => Vec2::new(-(max.x + min.x) / 2.0, -max.y),
        TilemapAnchor::CenterRight => Vec2::new(-max.x, -(max.y + min.y) / 2.0),
        TilemapAnchor::CenterLeft => Vec2::new(-min.x, -(max.y + min.y) / 2.0),
        TilemapAnchor::BottomLeft => Vec2::new(-min.x, -min.y),
        TilemapAnchor::BottomRight => Vec2::new(-max.x, -min.y),
        TilemapAnchor::BottomCenter => Vec2::new(-(max.x + min.x) / 2.0, -min.y),
        TilemapAnchor::Center => Vec2::new(-(max.x + min.x) / 2.0, -(max.y + min.y) / 2.0),
        TilemapAnchor::Custom(v) => Vec2::new(
            (-0.5 - v.x) * (max.x - min.x) - min.x,
            (-0.5 - v.y) * (max.y - min.y) - min.y,
        ),
    }
}

fn limit_initial_scale_by_visible_maps(
    tiled_world: &TiledWorldAsset,
    anchor: &TilemapAnchor,
    center: Vec2,
    preview: &MapPreviewState,
    fitted_scale: f32,
) -> f32 {
    if count_initial_visible_maps(tiled_world, anchor, center, preview, fitted_scale)
        <= MAX_INITIAL_VISIBLE_MAPS
    {
        return fitted_scale;
    }

    let mut low = MIN_INITIAL_WORLD_SCALE;
    let mut high = fitted_scale;
    for _ in 0..16 {
        let mid = (low + high) * 0.5;
        if count_initial_visible_maps(tiled_world, anchor, center, preview, mid)
            > MAX_INITIAL_VISIBLE_MAPS
        {
            high = mid;
        } else {
            low = mid;
        }
    }

    low
}

fn count_initial_visible_maps(
    tiled_world: &TiledWorldAsset,
    anchor: &TilemapAnchor,
    center: Vec2,
    preview: &MapPreviewState,
    scale: f32,
) -> usize {
    let chunking = world_chunking_for_preview(tiled_world, preview, scale);
    let Some(chunking) = chunking.0 else {
        return tiled_world.maps.len();
    };
    let min = center - chunking;
    let max = center + chunking;
    let offset = world_anchor_offset(tiled_world, anchor);

    tiled_world
        .maps
        .iter()
        .filter(|(rect, _)| {
            let displayed = Rect::new(
                rect.min.x + offset.x,
                rect.min.y + offset.y,
                rect.max.x + offset.x,
                rect.max.y + offset.y,
            );
            displayed.max.x >= min.x
                && displayed.min.x <= max.x
                && displayed.max.y >= min.y
                && displayed.min.y <= max.y
        })
        .count()
}
