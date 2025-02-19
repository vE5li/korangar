use std::cmp::PartialEq;
use std::sync::{Arc, Mutex};

use hashbrown::HashMap;
#[cfg(feature = "debug")]
use korangar_debug::logging::print_debug;
#[cfg(feature = "debug")]
use korangar_debug::profiling::Profiler;
#[cfg(feature = "debug")]
use korangar_util::texture_atlas::AtlasAllocation;
use ragnarok_packets::{EntityId, ItemId, TilePosition};
use rayon::{ThreadPool, ThreadPoolBuilder};

use crate::graphics::Texture;
use crate::init_tls_rand;
use crate::loaders::error::LoadError;
use crate::loaders::{
    ActionLoader, AnimationLoader, Cache, ImageType, MapLoader, ModelLoader, SpriteLoader, TextureAtlas, TextureLoader,
    UncompressedTextureAtlas,
};
#[cfg(feature = "debug")]
use crate::threads;
use crate::world::{AnimationData, EntityType, Map};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum ItemLocation {
    Inventory,
    Shop,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum LoaderId {
    AnimationData(EntityId),
    ItemSprite(ItemId),
    Map(String),
}

pub enum LoadableResource {
    AnimationData(Arc<AnimationData>),
    ItemSprite {
        texture: Arc<Texture>,
        location: ItemLocation,
    },
    Map {
        map: Box<Map>,
        player_position: Option<TilePosition>,
    },
}

enum LoadStatus {
    Loading,
    Completed(LoadableResource),
    Failed(LoadError),
}

impl PartialEq for LoadStatus {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

pub struct AsyncLoader {
    cache: Arc<Cache>,
    action_loader: Arc<ActionLoader>,
    animation_loader: Arc<AnimationLoader>,
    map_loader: Arc<MapLoader>,
    model_loader: Arc<ModelLoader>,
    sprite_loader: Arc<SpriteLoader>,
    texture_loader: Arc<TextureLoader>,
    pending_loads: Arc<Mutex<HashMap<LoaderId, LoadStatus>>>,
    thread_pool: ThreadPool,
}

impl AsyncLoader {
    pub fn new(
        cache: Arc<Cache>,
        action_loader: Arc<ActionLoader>,
        animation_loader: Arc<AnimationLoader>,
        map_loader: Arc<MapLoader>,
        model_loader: Arc<ModelLoader>,
        sprite_loader: Arc<SpriteLoader>,
        texture_loader: Arc<TextureLoader>,
    ) -> Self {
        let thread_pool = ThreadPoolBuilder::new()
            .thread_name(|number| format!("light task thread pool {number}"))
            .num_threads(1)
            .start_handler(|_| init_tls_rand())
            .build()
            .unwrap();

        Self {
            cache,
            action_loader,
            animation_loader,
            map_loader,
            model_loader,
            sprite_loader,
            texture_loader,
            pending_loads: Arc::new(Mutex::new(HashMap::new())),
            thread_pool,
        }
    }

    #[must_use]
    pub fn request_animation_data_load(
        &self,
        entity_id: EntityId,
        entity_type: EntityType,
        entity_part_files: Vec<String>,
    ) -> Option<Arc<AnimationData>> {
        match self.animation_loader.get(&entity_part_files) {
            Some(animation_data) => Some(animation_data),
            None => {
                let sprite_loader = self.sprite_loader.clone();
                let action_loader = self.action_loader.clone();
                let animation_loader = self.animation_loader.clone();

                self.request_load(LoaderId::AnimationData(entity_id), move || {
                    #[cfg(feature = "debug")]
                    let _load_measurement = Profiler::start_measurement("animation data load");

                    let animation_data = match animation_loader.get(&entity_part_files) {
                        Some(animation_data) => animation_data,
                        None => animation_loader.load(&sprite_loader, &action_loader, entity_type, &entity_part_files)?,
                    };
                    Ok(LoadableResource::AnimationData(animation_data))
                });

                None
            }
        }
    }

    #[must_use]
    pub fn request_item_sprite_load(
        &self,
        item_location: ItemLocation,
        item_id: ItemId,
        path: &str,
        image_type: ImageType,
    ) -> Option<Arc<Texture>> {
        match self.texture_loader.get(path, image_type) {
            Some(texture) => Some(texture),
            None => {
                let texture_loader = self.texture_loader.clone();
                let path = path.to_string();

                self.request_load(LoaderId::ItemSprite(item_id), move || {
                    #[cfg(feature = "debug")]
                    let _load_measurement = Profiler::start_measurement("item sprite load");

                    let texture = match texture_loader.get(&path, image_type) {
                        None => texture_loader.load(&path, image_type)?,
                        Some(texture) => texture,
                    };
                    Ok(LoadableResource::ItemSprite {
                        texture,
                        location: item_location,
                    })
                });

                None
            }
        }
    }

    pub fn request_map_load(
        &self,
        map_name: String,
        player_position: Option<TilePosition>,
        #[cfg(feature = "debug")] tile_texture_mapping: Arc<Vec<AtlasAllocation>>,
    ) {
        let cache = self.cache.clone();
        let map_loader = self.map_loader.clone();
        let model_loader = self.model_loader.clone();
        let texture_loader = self.texture_loader.clone();

        self.request_load(LoaderId::Map(map_name.clone()), move || {
            #[cfg(feature = "debug")]
            let _load_measurement = Profiler::start_measurement("map load");

            let mut texture_atlas: Box<dyn TextureAtlas> = match cache.load_texture_atlas(&map_name) {
                Some(texture_atlas) => Box::new(texture_atlas),
                None => Box::new(UncompressedTextureAtlas::new(
                    texture_loader.clone(),
                    map_name.clone(),
                    true,
                    true,
                    false,
                )),
            };

            let map = map_loader.load(
                &mut (*texture_atlas),
                map_name,
                &model_loader,
                texture_loader,
                #[cfg(feature = "debug")]
                &tile_texture_mapping,
            )?;

            Ok(LoadableResource::Map { map, player_position })
        });
    }

    fn request_load<F>(&self, id: LoaderId, load_function: F)
    where
        F: FnOnce() -> Result<LoadableResource, LoadError> + Send + 'static,
    {
        let pending_loads = Arc::clone(&self.pending_loads);

        pending_loads.lock().unwrap().insert(id.clone(), LoadStatus::Loading);

        self.thread_pool.spawn(move || {
            #[cfg(feature = "debug")]
            let _measurement = threads::Loader::start_frame();

            let result = load_function();

            let mut pending_loads = pending_loads.lock().unwrap();

            if !pending_loads.contains_key(&id) {
                return;
            }

            let status = match result {
                Ok(resource) => LoadStatus::Completed(resource),
                Err(err) => LoadStatus::Failed(err),
            };

            pending_loads.insert(id, status);
        });
    }

    pub fn take_completed(&self) -> impl Iterator<Item = (LoaderId, LoadableResource)> + '_ {
        std::iter::from_fn({
            let pending_loads = Arc::clone(&self.pending_loads);

            move || {
                let mut pending_loads = pending_loads.lock().unwrap();

                let completed_id = pending_loads
                    .iter()
                    .find(|(_, status)| matches!(status, LoadStatus::Completed(_) | LoadStatus::Failed(_)))
                    .map(|(id, _)| id.clone());

                if let Some(id) = completed_id {
                    match pending_loads.remove(&id).unwrap() {
                        LoadStatus::Failed(_error) => {
                            #[cfg(feature = "debug")]
                            print_debug!("Async load error: {:?}", _error);
                            None
                        }
                        LoadStatus::Completed(resource) => Some((id, resource)),
                        _ => unreachable!(),
                    }
                } else {
                    None
                }
            }
        })
    }
}
