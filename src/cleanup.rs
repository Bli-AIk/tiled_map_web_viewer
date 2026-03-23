use bevy::prelude::*;
use bevy_ecs_tiled::prelude::{TiledMap, TiledWorld};
use std::collections::VecDeque;

use crate::{LoadPhase, MapLoadRequest, MapLoadingState};

const MAP_LAYER_CLEANUP_BATCH: usize = 4;
const WORLD_MAP_CLEANUP_BATCH: usize = 1;

type ExistingAssetEntry<'a> = (
    Entity,
    Has<TiledMap>,
    Has<TiledWorld>,
    Option<&'a ChildOf>,
    &'a mut Visibility,
);
type ExistingRootFilter = Or<(With<TiledMap>, With<TiledWorld>)>;

#[derive(Resource, Default)]
pub(crate) struct PendingCleanup {
    queue: VecDeque<CleanupTarget>,
}

impl PendingCleanup {
    pub(crate) fn clear(&mut self) {
        self.queue.clear();
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    fn push(&mut self, target: CleanupTarget) {
        self.queue.push_back(target);
    }
}

#[derive(Clone, Copy, Debug)]
enum CleanupTarget {
    MapRoot(Entity),
    WorldRoot(Entity),
    AwaitRemoval(Entity),
}

pub(crate) fn handle_map_load(
    mut load_request: ResMut<MapLoadRequest>,
    mut loading: ResMut<MapLoadingState>,
    mut cleanup: ResMut<PendingCleanup>,
    i18n: Res<bevy_workbench::i18n::I18n>,
    mut existing_assets: Query<ExistingAssetEntry<'_>>,
) {
    let Some(entry) = load_request.entry_to_load.take() else {
        return;
    };

    cleanup.clear();

    for (entity, has_map, has_world, child_of, mut visibility) in &mut existing_assets {
        let is_standalone_map = has_map && child_of.is_none();
        if has_world {
            *visibility = Visibility::Hidden;
            cleanup.push(CleanupTarget::WorldRoot(entity));
        } else if is_standalone_map {
            *visibility = Visibility::Hidden;
            cleanup.push(CleanupTarget::MapRoot(entity));
        }
    }

    loading.phase = LoadPhase::Cleanup;
    loading.current_entry = Some(entry);
    loading.pending_asset = None;
    loading.status_text = i18n.t("loading-cleanup");
}

pub(crate) fn process_pending_cleanup(
    mut commands: Commands,
    mut cleanup: ResMut<PendingCleanup>,
    map_roots: Query<(Entity, Option<&Children>), With<TiledMap>>,
    world_roots: Query<(Entity, Option<&Children>), With<TiledWorld>>,
    existing_roots: Query<Entity, ExistingRootFilter>,
) {
    if cleanup.is_empty() {
        return;
    }

    let mut next_queue = VecDeque::new();
    while let Some(target) = cleanup.queue.pop_front() {
        match target {
            CleanupTarget::MapRoot(entity) => {
                let Ok((map_entity, children)) = map_roots.get(entity) else {
                    continue;
                };

                let layers = children
                    .into_iter()
                    .flatten()
                    .take(MAP_LAYER_CLEANUP_BATCH)
                    .copied()
                    .collect::<Vec<_>>();

                for layer_entity in layers {
                    commands.entity(layer_entity).despawn();
                }

                if children.is_none_or(|children| children.is_empty()) {
                    commands.entity(map_entity).despawn();
                    next_queue.push_back(CleanupTarget::AwaitRemoval(map_entity));
                } else {
                    next_queue.push_back(CleanupTarget::MapRoot(map_entity));
                }
            }
            CleanupTarget::WorldRoot(entity) => {
                let Ok((world_entity, children)) = world_roots.get(entity) else {
                    continue;
                };

                let maps = children
                    .into_iter()
                    .flatten()
                    .take(WORLD_MAP_CLEANUP_BATCH)
                    .copied()
                    .collect::<Vec<_>>();

                for map_entity in maps {
                    commands.entity(map_entity).despawn();
                }

                if children.is_none_or(|children| children.is_empty()) {
                    commands.entity(world_entity).despawn();
                    next_queue.push_back(CleanupTarget::AwaitRemoval(world_entity));
                } else {
                    next_queue.push_back(CleanupTarget::WorldRoot(world_entity));
                }
            }
            CleanupTarget::AwaitRemoval(entity) => {
                if existing_roots.get(entity).is_ok() {
                    next_queue.push_back(CleanupTarget::AwaitRemoval(entity));
                }
            }
        }
    }

    cleanup.queue = next_queue;
}
