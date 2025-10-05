use std::cmp::PartialEq;
use std::sync::{Arc, Mutex};

use hashbrown::HashMap;
#[cfg(feature = "debug")]
use korangar_debug::logging::print_debug;
#[cfg(feature = "debug")]
use korangar_debug::profiling::Profiler;
use korangar_networking::{InventoryItem, NoMetadata, ShopItem};
use ragnarok_packets::{ClientTick, EntityId, ItemId, JobId, SkillId, TilePosition};
use rayon::{ThreadPool, ThreadPoolBuilder};

use crate::graphics::Texture;
use crate::init_tls_rand;
use crate::loaders::error::LoadError;
use crate::loaders::{ActionLoader, AnimationLoader, ImageType, MapLoader, ModelLoader, Sprite, SpriteLoader, TextureLoader, VideoLoader};
use crate::state::skills::{LearnableSkill, SkillTabLayout, SkillTreeLayout};
#[cfg(feature = "debug")]
use crate::threads;
use crate::world::{
    Actions, AnimationData, EntityType, ItemName, ItemNameKey, ItemResource, ItemResourceKey, Library, Map, ResourceMetadata,
    SkillListInformation, SkillListKey, SkillListRequirements, SpriteAnimationState,
};

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum LoaderId {
    AnimationData(EntityId),
    ItemSprite(ItemId),
    Map(String),
    SkillSprite(SkillId),
    SkillActions(SkillId),
}

pub enum LoadableResource {
    AnimationData(Arc<AnimationData>),
    ItemSprite { texture: Arc<Texture> },
    SkillSprite { sprite: Arc<Sprite> },
    SkillActions { actions: Arc<Actions> },
    Map { map: Box<Map>, position: Option<TilePosition> },
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
    action_loader: Arc<ActionLoader>,
    animation_loader: Arc<AnimationLoader>,
    map_loader: Arc<MapLoader>,
    model_loader: Arc<ModelLoader>,
    sprite_loader: Arc<SpriteLoader>,
    texture_loader: Arc<TextureLoader>,
    video_loader: Arc<VideoLoader>,
    library: Arc<Library>,
    pending_loads: Arc<Mutex<HashMap<LoaderId, LoadStatus>>>,
    thread_pool: ThreadPool,
}

impl AsyncLoader {
    pub fn new(
        action_loader: Arc<ActionLoader>,
        animation_loader: Arc<AnimationLoader>,
        map_loader: Arc<MapLoader>,
        model_loader: Arc<ModelLoader>,
        sprite_loader: Arc<SpriteLoader>,
        texture_loader: Arc<TextureLoader>,
        video_loader: Arc<VideoLoader>,
        library: Arc<Library>,
    ) -> Self {
        let thread_pool = ThreadPoolBuilder::new()
            .thread_name(|number| format!("light task thread pool {number}"))
            .num_threads(1)
            .start_handler(|_| init_tls_rand())
            .build()
            .unwrap();

        Self {
            action_loader,
            animation_loader,
            map_loader,
            model_loader,
            sprite_loader,
            texture_loader,
            video_loader,
            library,
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
    pub fn request_skill_sprite_load(&self, skill_id: SkillId, path: &str) -> Option<Arc<Sprite>> {
        let sprite_path = format!("{path}.spr");

        match self.sprite_loader.get(&sprite_path) {
            Some(sprite) => Some(sprite),
            None => {
                let sprite_loader = self.sprite_loader.clone();

                self.request_load(LoaderId::SkillSprite(skill_id), move || {
                    #[cfg(feature = "debug")]
                    let _load_measurement = Profiler::start_measurement("skill sprite load");

                    let sprite = match sprite_loader.get(&sprite_path) {
                        Some(sprite) => sprite,
                        None => sprite_loader.load(&sprite_path)?,
                    };
                    Ok(LoadableResource::SkillSprite { sprite })
                });

                None
            }
        }
    }

    #[must_use]
    pub fn request_skill_actions_load(&self, skill_id: SkillId, path: &str) -> Option<Arc<Actions>> {
        let actions_path = format!("{path}.act");

        match self.action_loader.get(&actions_path) {
            Some(actions) => Some(actions),
            None => {
                let action_loader = self.action_loader.clone();

                self.request_load(LoaderId::SkillActions(skill_id), move || {
                    #[cfg(feature = "debug")]
                    let _load_measurement = Profiler::start_measurement("skill actions load");

                    let actions = match action_loader.get(&actions_path) {
                        Some(actions) => actions,
                        None => action_loader.load(&actions_path)?,
                    };
                    Ok(LoadableResource::SkillActions { actions })
                });

                None
            }
        }
    }

    #[must_use]
    pub fn request_item_sprite_load(&self, item_id: ItemId, path: &str, image_type: ImageType) -> Option<Arc<Texture>> {
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
                    Ok(LoadableResource::ItemSprite { texture })
                });

                None
            }
        }
    }

    pub fn request_inventory_item_metadata_load(&self, item: InventoryItem<NoMetadata>) -> InventoryItem<ResourceMetadata> {
        let is_identified = item.is_identified();

        let resource_name = self.library.get::<ItemResource>(ItemResourceKey {
            item_id: item.item_id,
            is_identified,
        });
        let full_path = format!("유저인터페이스\\item\\{resource_name}.bmp");
        let texture = self.request_item_sprite_load(item.item_id, &full_path, ImageType::Color);
        let name = self
            .library
            .get::<ItemName>(ItemNameKey {
                item_id: item.item_id,
                is_identified,
            })
            .to_string();

        let metadata = ResourceMetadata { texture, name };

        InventoryItem { metadata, ..item }
    }

    pub fn request_shop_item_metadata_load(&self, item: ShopItem<NoMetadata>) -> ShopItem<ResourceMetadata> {
        let resource_name = self.library.get::<ItemResource>(ItemResourceKey {
            item_id: item.item_id,
            is_identified: true,
        });
        let full_path = format!("유저인터페이스\\item\\{resource_name}.bmp");
        let texture = self.request_item_sprite_load(item.item_id, &full_path, ImageType::Color);
        let name = self
            .library
            .get::<ItemName>(ItemNameKey {
                item_id: item.item_id,
                is_identified: true,
            })
            .to_string();

        let metadata = ResourceMetadata { texture, name };

        ShopItem { metadata, ..item }
    }

    pub fn request_learnable_skill_load(&self, job_id: JobId, skill_id: SkillId, client_tick: ClientTick) -> LearnableSkill {
        let skill_information = self.library.get::<SkillListInformation>(skill_id);
        let skill_requirements = self.library.get::<SkillListRequirements>(SkillListKey::with_job(job_id, skill_id));

        let path = format!("아이템\\{}", skill_information.file_name);
        let sprite = self.request_skill_sprite_load(skill_id, &path);
        let actions = self.request_skill_actions_load(skill_id, &path);

        LearnableSkill {
            skill_id,
            maximum_level: skill_information.maximum_level,
            file_name: skill_information.file_name.clone(),
            skill_name: skill_information.name.clone(),
            can_select_level: skill_information.can_select_level,
            acquisition: skill_information.acquisition,
            required_skills: skill_requirements.required_skills.clone(),
            required_for_skills: skill_requirements.required_for_skills.clone(),
            sprite,
            actions,
            animation_state: SpriteAnimationState::new(client_tick),
        }
    }

    pub fn request_map_load(&self, map_name: String, position: Option<TilePosition>) {
        let map_loader = self.map_loader.clone();
        let model_loader = self.model_loader.clone();
        let texture_loader = self.texture_loader.clone();
        let video_loader = self.video_loader.clone();
        let library = self.library.clone();

        self.request_load(LoaderId::Map(map_name.clone()), move || {
            #[cfg(feature = "debug")]
            let _load_measurement = Profiler::start_measurement("map load");
            let map = map_loader.load(map_name, &model_loader, texture_loader, video_loader.clone(), &library)?;
            Ok(LoadableResource::Map { map, position })
        });
    }

    pub fn request_skill_tree_layout_load(&self, job_id: JobId, client_tick: ClientTick) -> SkillTreeLayout {
        let layout = self.library.get::<crate::world::SkillTreeLayout>(job_id);

        let tabs = layout
            .tabs
            .iter()
            .map(|tab| {
                let name = tab.name.clone();
                let skills = HashMap::from_iter(
                    tab.skills
                        .iter()
                        .map(|(slot, skill_id)| (*slot, self.request_learnable_skill_load(job_id, *skill_id, client_tick))),
                );

                SkillTabLayout { name, skills }
            })
            .collect();

        SkillTreeLayout { tabs }
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
