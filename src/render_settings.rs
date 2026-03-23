use bevy::color::Color;
use bevy::prelude::*;
use bevy_ecs_tiled::prelude::{
    TiledMap, TiledMapAsset, TiledWorld, TiledWorldAsset, TilemapAnchor, TilemapType,
    grid_size_from_map, tilemap_type_from_map,
};
use bevy_workbench::dock::{TileLayoutState, WorkbenchPanel};

use crate::{PreviewCamera, SharedTranslations};

const PREVIEW_BG_DEFAULT: [u8; 4] = [26, 26, 38, 255];
const PREVIEW_GRID_DEFAULT: [u8; 4] = [160, 160, 160, 96];
const WORLD_GRID_DEFAULT: [u8; 4] = [255, 255, 255, 255];

type PreviewMapEntry<'a> = (
    &'a TiledMap,
    Option<&'a ChildOf>,
    Option<&'a TilemapAnchor>,
    &'a GlobalTransform,
    Option<&'a Visibility>,
);
type PreviewWorldEntry<'a> = (
    &'a TiledWorld,
    Option<&'a TilemapAnchor>,
    &'a GlobalTransform,
    Option<&'a Visibility>,
);

#[derive(Resource, Clone)]
pub(crate) struct RenderSettingsState {
    pub(crate) preview_background: [u8; 4],
    pub(crate) show_preview_grid: bool,
    pub(crate) preview_grid_color: [u8; 4],
    pub(crate) show_world_grid: bool,
    pub(crate) world_grid_color: [u8; 4],
}

impl Default for RenderSettingsState {
    fn default() -> Self {
        Self {
            preview_background: PREVIEW_BG_DEFAULT,
            show_preview_grid: true,
            preview_grid_color: PREVIEW_GRID_DEFAULT,
            show_world_grid: true,
            world_grid_color: WORLD_GRID_DEFAULT,
        }
    }
}

#[derive(Default)]
pub(crate) struct RenderSettingsPanel {
    pub(crate) translations: SharedTranslations,
}

impl RenderSettingsPanel {
    pub(crate) fn new(translations: SharedTranslations) -> Self {
        Self { translations }
    }
}

impl WorkbenchPanel for RenderSettingsPanel {
    fn id(&self) -> &str {
        "render_settings_inspector"
    }

    fn title(&self) -> String {
        self.translations
            .read()
            .map(|t| t.render_settings.clone())
            .unwrap_or_else(|_| "Render Settings".into())
    }

    fn ui(&mut self, _ui: &mut egui::Ui) {}

    fn ui_world(&mut self, ui: &mut egui::Ui, world: &mut World) {
        let Ok(t) = self.translations.read() else {
            ui.label("Translations unavailable");
            return;
        };
        let mut state = world.resource_mut::<RenderSettingsState>();

        ui.heading(&t.render_settings);
        ui.separator();

        color_row(
            ui,
            &t.render_background,
            &mut state.preview_background,
            &t.render_background_hint,
        );

        ui.separator();
        ui.checkbox(&mut state.show_preview_grid, &t.render_preview_grid);
        color_row(
            ui,
            &t.render_preview_grid_color,
            &mut state.preview_grid_color,
            &t.render_preview_grid_hint,
        );

        ui.separator();
        ui.checkbox(&mut state.show_world_grid, &t.render_world_grid);
        color_row(
            ui,
            &t.render_world_grid_color,
            &mut state.world_grid_color,
            &t.render_world_grid_hint,
        );
    }

    fn needs_world(&self) -> bool {
        true
    }

    fn default_visible(&self) -> bool {
        true
    }
}

fn color_row(ui: &mut egui::Ui, label: &str, color: &mut [u8; 4], hint: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new(label).strong());
        let mut rgba = *color;
        if ui.color_edit_button_srgba_unmultiplied(&mut rgba).changed() {
            *color = rgba;
        }
    });
    ui.small(hint);
}

pub(crate) fn apply_preview_render_settings(
    settings: Res<RenderSettingsState>,
    mut preview_camera: Query<&mut Camera, With<PreviewCamera>>,
) {
    if !settings.is_changed() {
        return;
    }

    let Ok(mut camera) = preview_camera.single_mut() else {
        return;
    };

    camera.clear_color =
        bevy::camera::ClearColorConfig::Custom(color_from_rgba(settings.preview_background));
}

pub(crate) fn draw_preview_gizmos(
    settings: Res<RenderSettingsState>,
    mut gizmos: Gizmos,
    map_assets: Res<Assets<TiledMapAsset>>,
    world_assets: Res<Assets<TiledWorldAsset>>,
    maps: Query<PreviewMapEntry<'_>>,
    worlds: Query<PreviewWorldEntry<'_>>,
) {
    if !settings.show_preview_grid && !settings.show_world_grid {
        return;
    }

    if settings.show_preview_grid {
        let color = color_from_rgba(settings.preview_grid_color);
        for (map, child_of, anchor, transform, visibility) in &maps {
            if child_of.is_some() || is_hidden(visibility) {
                continue;
            }

            let Some(asset) = map_assets.get(&map.0) else {
                continue;
            };
            draw_map_grid(
                &mut gizmos,
                asset,
                anchor.copied().unwrap_or_default(),
                transform,
                color,
            );
        }
    }

    if settings.show_world_grid {
        let color = color_from_rgba(settings.world_grid_color);
        for (world, anchor, transform, visibility) in &worlds {
            if is_hidden(visibility) {
                continue;
            }

            let Some(asset) = world_assets.get(&world.0) else {
                continue;
            };
            draw_world_grid(
                &mut gizmos,
                asset,
                anchor.copied().unwrap_or_default(),
                transform,
                color,
            );
        }
    }
}

pub(crate) fn ensure_render_settings_dock_layout(
    mut tile_state: ResMut<TileLayoutState>,
    mut applied: Local<bool>,
) {
    if *applied {
        return;
    }

    let Some(details_panel_id) = find_panel_id(&tile_state, "map_details_inspector") else {
        return;
    };
    let Some(settings_panel_id) = find_panel_id(&tile_state, "render_settings_inspector") else {
        return;
    };
    let Some(tree) = tile_state.tree.as_mut() else {
        return;
    };
    let Some(details_tile) = find_panel_tile(tree, details_panel_id) else {
        return;
    };
    let Some(settings_tile) = find_panel_tile(tree, settings_panel_id) else {
        return;
    };

    let Some(root_id) = tree.root else {
        return;
    };
    let main_row_id = match tree.tiles.get(root_id) {
        Some(egui_tiles::Tile::Container(egui_tiles::Container::Linear(linear)))
            if linear.dir == egui_tiles::LinearDir::Vertical =>
        {
            *linear.children.first().unwrap_or(&root_id)
        }
        _ => root_id,
    };

    let Some(right_tile) = ancestor_child_of(&tree.tiles, main_row_id, details_tile) else {
        return;
    };

    let Some(settings_parent) = tree.tiles.parent_of(settings_tile) else {
        return;
    };
    if settings_parent == right_tile {
        // Still in the same right-side tab group: move it into its own bottom dock.
    } else if let Some(ancestor) = ancestor_child_of(&tree.tiles, main_row_id, settings_tile)
        && ancestor != right_tile
    {
        // The panel is still elsewhere in the layout, pull it into the right-side stack once.
    }

    if let Some(egui_tiles::Tile::Container(container)) = tree.tiles.get_mut(settings_parent) {
        container.remove_child(settings_tile);
    }

    let settings_tab = tree.tiles.insert_tab_tile(vec![settings_tile]);
    let new_right = tree
        .tiles
        .insert_vertical_tile(vec![right_tile, settings_tab]);
    if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Linear(linear))) =
        tree.tiles.get_mut(new_right)
    {
        linear.shares.set_share(right_tile, 2.3);
        linear.shares.set_share(settings_tab, 1.2);
    }

    if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Linear(linear))) =
        tree.tiles.get_mut(main_row_id)
    {
        if let Some(slot) = linear
            .children
            .iter_mut()
            .find(|child| **child == right_tile)
        {
            *slot = new_right;
        }
        linear.shares.replace_with(right_tile, new_right);
    }

    *applied = true;
}

fn find_panel_id(tile_state: &TileLayoutState, panel_str_id: &str) -> Option<usize> {
    tile_state
        .panels
        .iter()
        .find_map(|(panel_id, panel)| (panel.id() == panel_str_id).then_some(*panel_id))
}

fn find_panel_tile(
    tree: &egui_tiles::Tree<bevy_workbench::dock::PaneEntry>,
    panel_id: usize,
) -> Option<egui_tiles::TileId> {
    tree.tiles.iter().find_map(|(tile_id, tile)| match tile {
        egui_tiles::Tile::Pane(pane) if pane.panel_id == panel_id => Some(*tile_id),
        _ => None,
    })
}

fn draw_map_grid(
    gizmos: &mut Gizmos,
    asset: &TiledMapAsset,
    anchor: TilemapAnchor,
    transform: &GlobalTransform,
    color: Color,
) {
    let map_type = tilemap_type_from_map(&asset.map);
    if map_type != TilemapType::Square {
        return;
    }

    let grid = grid_size_from_map(&asset.map);
    let offset = anchor.as_offset(
        &asset.tilemap_size,
        &grid,
        &asset.largest_tile_size,
        &map_type,
    );
    let min = offset - Vec2::new(grid.x, grid.y) * 0.5;
    let max = min
        + Vec2::new(
            asset.tilemap_size.x as f32 * grid.x,
            asset.tilemap_size.y as f32 * grid.y,
        );

    for x in 0..=asset.tilemap_size.x {
        let x_pos = min.x + x as f32 * grid.x;
        gizmos.line_2d(
            transform_point_2d(transform, Vec2::new(x_pos, min.y)),
            transform_point_2d(transform, Vec2::new(x_pos, max.y)),
            color,
        );
    }
    for y in 0..=asset.tilemap_size.y {
        let y_pos = min.y + y as f32 * grid.y;
        gizmos.line_2d(
            transform_point_2d(transform, Vec2::new(min.x, y_pos)),
            transform_point_2d(transform, Vec2::new(max.x, y_pos)),
            color,
        );
    }
}

fn draw_world_grid(
    gizmos: &mut Gizmos,
    asset: &TiledWorldAsset,
    anchor: TilemapAnchor,
    transform: &GlobalTransform,
    color: Color,
) {
    let offset = world_anchor_offset(asset, &anchor);
    for (rect, _) in &asset.maps {
        let tl = transform_point_2d(transform, Vec2::new(rect.min.x, rect.max.y) + offset);
        let tr = transform_point_2d(transform, rect.max + offset);
        let br = transform_point_2d(transform, Vec2::new(rect.max.x, rect.min.y) + offset);
        let bl = transform_point_2d(transform, rect.min + offset);
        gizmos.lineloop_2d([tl, tr, br, bl], color);
    }
}

fn transform_point_2d(transform: &GlobalTransform, point: Vec2) -> Vec2 {
    transform.transform_point(point.extend(0.0)).truncate()
}

fn color_from_rgba(color: [u8; 4]) -> Color {
    Color::srgba_u8(color[0], color[1], color[2], color[3])
}

fn is_hidden(visibility: Option<&Visibility>) -> bool {
    matches!(visibility, Some(Visibility::Hidden))
}

fn ancestor_child_of(
    tiles: &egui_tiles::Tiles<bevy_workbench::dock::PaneEntry>,
    ancestor: egui_tiles::TileId,
    descendant: egui_tiles::TileId,
) -> Option<egui_tiles::TileId> {
    let mut current = descendant;
    loop {
        let parent = tiles.parent_of(current)?;
        if parent == ancestor {
            return Some(current);
        }
        current = parent;
    }
}

fn world_anchor_offset(asset: &TiledWorldAsset, anchor: &TilemapAnchor) -> Vec2 {
    let min = asset.rect.min;
    let max = asset.rect.max;
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
