use std::string::String;
use std::sync::Arc;

use arrayvec::ArrayVec;
use cgmath::{EuclideanSpace, Point3, Vector2, VectorSpace};
use korangar_audio::{AudioEngine, SoundEffectKey};
#[cfg(feature = "debug")]
use korangar_debug::logging::Colorize;
use korangar_interface::element::StateElement;
use korangar_interface::window::{StateWindow, Window};
use korangar_networking::EntityData;
use ragnarok_packets::{AccountId, CharacterInformation, ClientTick, Direction, EntityId, Sex, StatType, TilePosition, WorldPosition};
use rust_state::{Path, RustState, VecItem};
#[cfg(feature = "debug")]
use smallvec::smallvec_inline;
#[cfg(feature = "debug")]
use wgpu::{BufferUsages, Device, Queue};

#[cfg(feature = "debug")]
use crate::graphics::reduce_vertices;
#[cfg(feature = "debug")]
use crate::graphics::{BindlessSupport, DebugRectangleInstruction};
use crate::graphics::{EntityInstruction, ScreenPosition, ScreenSize};
use crate::loaders::GameFileLoader;
#[cfg(feature = "debug")]
use crate::loaders::{GAT_TILE_SIZE, split_mesh_by_texture};
use crate::renderer::GameInterfaceRenderer;
#[cfg(feature = "debug")]
use crate::renderer::MarkerRenderer;
use crate::state::ClientState;
use crate::state::theme::{InterfaceThemeType, WorldTheme};
use crate::world::{ActionEvent, AnimationData, AnimationState, Camera, Library, MAX_WALK_PATH_SIZE, Map, PathFinder};
#[cfg(feature = "debug")]
use crate::world::{MarkerIdentifier, SubMesh};
#[cfg(feature = "debug")]
use crate::{Buffer, Color, ModelVertex};

const MALE_HAIR_LOOKUP: &[usize] = &[2, 2, 1, 7, 5, 4, 3, 6, 8, 9, 10, 12, 11];
const FEMALE_HAIR_LOOKUP: &[usize] = &[2, 2, 4, 7, 1, 5, 3, 6, 12, 10, 9, 11, 8];
const SOUND_COOLDOWN_DURATION: u32 = 200;
const SPATIAL_SOUND_RANGE: f32 = 250.0;

#[derive(Clone)]
pub enum ResourceState<T> {
    Available(T),
    Unavailable,
    Requested,
}

impl<T> ResourceState<T> {
    pub fn as_option(&self) -> Option<&T> {
        match self {
            ResourceState::Available(value) => Some(value),
            _requested_or_unavailable => None,
        }
    }
}

#[derive(Clone, RustState, StateElement)]
pub struct Movement {
    #[hidden_element]
    steps: ArrayVec<Step, MAX_WALK_PATH_SIZE>,
    starting_timestamp: u32,
    #[cfg(feature = "debug")]
    #[hidden_element]
    pub pathing: Option<Pathing>,
}

impl Movement {
    pub fn new(steps: ArrayVec<Step, MAX_WALK_PATH_SIZE>, starting_timestamp: u32) -> Self {
        Self {
            steps,
            starting_timestamp,
            #[cfg(feature = "debug")]
            pathing: None,
        }
    }
}

#[cfg(feature = "debug")]
#[derive(Clone)]
pub struct Pathing {
    pub vertex_buffer: Arc<Buffer<ModelVertex>>,
    pub index_buffer: Arc<Buffer<u32>>,
    pub submeshes: Vec<SubMesh>,
}

#[derive(Copy, Clone)]
pub struct Step {
    arrival_position: TilePosition,
    arrival_timestamp: u32,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum EntityType {
    Hidden,
    Monster,
    Npc,
    Player,
    Warp,
}

impl From<usize> for EntityType {
    fn from(value: usize) -> Self {
        match value {
            45 => EntityType::Warp,
            111 => EntityType::Hidden, // TODO: check that this is correct
            0..=44 | 4000..=5999 => EntityType::Player,
            46..=999 | 10000..=19999 => EntityType::Npc,
            1000..=3999 | 20000..=29999 => EntityType::Monster,
            _ => EntityType::Npc,
        }
    }
}

#[derive(Copy, Clone, Default)]
pub struct SoundState {
    previous_key: Option<SoundEffectKey>,
    last_played_at: Option<ClientTick>,
}

impl SoundState {
    pub fn update(
        &mut self,
        audio_engine: &AudioEngine<GameFileLoader>,
        position: Point3<f32>,
        sound_effect_key: SoundEffectKey,
        client_tick: ClientTick,
    ) {
        let should_play = if Some(sound_effect_key) == self.previous_key
            && let Some(last_tick) = self.last_played_at
        {
            (client_tick.0.wrapping_sub(last_tick.0)) >= SOUND_COOLDOWN_DURATION
        } else {
            true
        };

        if should_play {
            audio_engine.play_spatial_sound_effect(sound_effect_key, position, SPATIAL_SOUND_RANGE);
            self.last_played_at = Some(client_tick);
            self.previous_key = Some(sound_effect_key);
        }
    }
}

const FADE_DURATION: u32 = 500;

#[derive(Copy, Clone, Debug)]
pub enum FadeDirection {
    In,
    Out,
}

#[derive(Copy, Clone, Debug)]
pub enum FadeState {
    None,
    FadingIn { start_time: ClientTick },
    FadingOut { start_time: ClientTick },
}

impl FadeState {
    pub fn calculate_alpha(&self, client_tick: ClientTick) -> f32 {
        match self {
            FadeState::None => 1.0,
            FadeState::FadingIn { start_time } => {
                let elapsed = client_tick.0.wrapping_sub(start_time.0);
                (elapsed as f32 / FADE_DURATION as f32).min(1.0)
            }
            FadeState::FadingOut { start_time } => {
                let elapsed = client_tick.0.wrapping_sub(start_time.0);
                1.0 - (elapsed as f32 / FADE_DURATION as f32).min(1.0)
            }
        }
    }

    pub fn is_complete(&self, client_tick: ClientTick) -> bool {
        match self {
            FadeState::None => true,
            FadeState::FadingIn { start_time } | FadeState::FadingOut { start_time } => {
                let elapsed = client_tick.0.wrapping_sub(start_time.0);
                elapsed >= FADE_DURATION
            }
        }
    }

    /// Creates a new fade state starting from a specific alpha value.
    /// This allows smooth transitions between fade states by preserving the
    /// current alpha.
    pub fn from_alpha(alpha: f32, direction: FadeDirection, client_tick: ClientTick) -> Self {
        let alpha = alpha.clamp(0.0, 1.0);
        let elapsed = match direction {
            FadeDirection::In => (alpha * FADE_DURATION as f32) as u32,
            FadeDirection::Out => ((1.0 - alpha) * FADE_DURATION as f32) as u32,
        };
        let start_time = ClientTick(client_tick.0.wrapping_sub(elapsed));
        match direction {
            FadeDirection::In => FadeState::FadingIn { start_time },
            FadeDirection::Out => FadeState::FadingOut { start_time },
        }
    }
}

#[derive(Clone, RustState, StateElement)]
pub struct Common {
    pub entity_id: EntityId,
    pub job_id: usize,
    pub health_points: usize,
    pub maximum_health_points: usize,
    pub movement_speed: usize,
    pub direction: Direction,
    pub head_direction: usize,
    pub sex: Sex,

    #[hidden_element]
    pub entity_type: EntityType,
    pub active_movement: Option<Movement>,
    pub animation_data: Option<Arc<AnimationData>>,
    pub tile_position: TilePosition,
    pub world_position: Point3<f32>,
    #[hidden_element]
    details: ResourceState<String>,
    #[hidden_element]
    animation_state: AnimationState,
    stopped_moving: bool,
    #[hidden_element]
    sound_state: SoundState,
    #[hidden_element]
    fade_state: FadeState,
}

#[cfg_attr(feature = "debug", korangar_debug::profile)]
#[allow(clippy::invisible_characters)]
fn get_sprite_path_for_player_job(job_id: usize) -> &'static str {
    match job_id {
        0 => "초보자",             // NOVICE
        1 => "검사",               // SWORDMAN
        2 => "마법사",             // MAGICIAN
        3 => "궁수",               // ARCHER
        4 => "성직자",             // ACOLYTE
        5 => "상인",               // MERCHANT
        6 => "도둑",               // THIEF
        7 => "기사",               // KNIGHT
        8 => "성투사",             // PRIEST
        9 => "위저드",             // WIZARD
        10 => "제철공",            // BLACKSMITH
        11 => "헌터",              // HUNTER
        12 => "어세신",            // ASSASSIN
        13 => "페코페코_기사",     // KNIGHT2
        14 => "크루세이더",        // CRUSADER
        15 => "몽크",              // MONK
        16 => "세이지",            // SAGE
        17 => "로그",              // ROGUE
        18 => "연금술사",          // ALCHEMIST
        19 => "바드",              // BARD
        20 => "무희",              // DANCER
        23 => "슈퍼노비스",        // SUPERNOVICE
        24 => "건너",              // GUNSLINGER
        25 => "닌자",              // NINJA
        4001 => "초보자",          // NOVICE_H
        4002 => "검사",            // SWORDMAN_H
        4003 => "마법사",          // MAGICIAN_H
        4004 => "궁수",            // ARCHER_H
        4005 => "성직자",          // ACOLYTE_H
        4006 => "상인",            // MERCHANT_H
        4007 => "도둑",            // THIEF_H
        4008 => "로드나이트",      // KNIGHT_H
        4009 => "하이프리",        // PRIEST_H
        4010 => "하이위저드",      // WIZARD_H
        4011 => "화이트스미스",    // BLACKSMITH_H
        4012 => "스나이퍼",        // HUNTER_H
        4013 => "어쌔신크로스",    // ASSASSIN_H
        4014 => "엔대운",          // CHICKEN_H
        4015 => "크루세이더",      // CRUSADER_H
        4016 => "몽크",            // MONK_H
        4017 => "세이지",          // SAGE_H
        4018 => "로그",            // ROGUE_H
        4019 => "연금술사",        // ALCHEMIST_H
        4020 => "바드",            // BARD_H
        4021 => "무희",            // DANCER_H
        4023 => "슈퍼노비스",      // NOVICE_B
        4024 => "검뽀",            // SWORDMAN_B
        4025 => "위저드",          // MAGICIAN_B
        4026 => "궁솔",            // ARCHER_B
        4027 => "성직자",          // ACOLYTE_B
        4028 => "뿐인",            // MERCHANT_B
        4029 => "도둑",            // THIEF_B
        4030 => "기사",            // KNIGHT_B
        4031 => "성투사",          // PRIEST_B
        4032 => "마반뽀",          // WIZARD_B
        4033 => "제철공",          // BLACKSMITH_B
        4034 => "헌터",            // HUNTER_B
        4035 => "어세신",          // ASSASSIN_B
        4037 => "크루세이더",      // CRUSADER_B
        4038 => "몽크",            // MONK_B
        4039 => "세이지",          // SAGE_B
        4040 => "로그",            // ROGUE_B
        4041 => "연금술사",        // ALCHEMIST_B
        4042 => "바드",            // BARD_B
        4043 => "무희",            // DANCER_B
        4045 => "슈퍼노비스",      // SUPERNOVICE_B
        4054 => "룬나이트",        // RUNE_KNIGHT
        4055 => "워록",            // WARLOCK
        4056 => "레인져",          // RANGER
        4057 => "아크비숍",        // ARCH_BISHOP
        4058 => "미케닉",          // MECHANIC
        4059 => "길로틴크로스",    // GUILLOTINE_CROSS
        4066 => "가드",            // ROYAL_GUARD
        4067 => "소서러",          // SORCERER
        4068 => "민스트럴",        // MINSTREL
        4069 => "원더러",          // WANDERER
        4070 => "슈라",            // SURA
        4071 => "제네릭",          // GENETIC
        4072 => "쉐도우체이서",    // SHADOW_CHASER
        4060 => "룬나이트",        // RUNE_KNIGHT_H
        4061 => "워록",            // WARLOCK_H
        4062 => "레인져",          // RANGER_H
        4063 => "아크비숍",        // ARCH_BISHOP_H
        4064 => "미케닉",          // MECHANIC_H
        4065 => "길로틴크로스",    // GUILLOTINE_CROSS_H
        4073 => "가돼",            // ROYAL_GUARD_H
        4074 => "소서러",          // SORCERER_H
        4075 => "민스트럴",        // MINSTREL_H
        4076 => "원더러",          // WANDERER_H
        4077 => "슈라",            // SURA_H
        4078 => "제네릭",          // GENETIC_H
        4079 => "쉐도우체이서",    // SHADOW_CHASER_H
        4096 => "룬나이트",        // RUNE_KNIGHT_B
        4097 => "워록",            // WARLOCK_B
        4098 => "레인져",          // RANGER_B
        4099 => "아크비숍",        // ARCHBISHOP_B
        4100 => "미케닉",          // MECHANIC_B
        4101 => "길로틴크로스",    // GUILLOTINE_CROSS_B
        4102 => "가드",            // ROYAL_GUARD_B
        4103 => "소서러",          // SORCERER_B
        4104 => "민스트럴",        // MINSTREL_B
        4105 => "원더러",          // WANDERER_B
        4106 => "슈라",            // SURA_B
        4107 => "제네릭",          // GENETIC_B
        4108 => "쉐도우체이서",    // SHADOW_CHASER_B
        4046 => "태권소년",        // TAEKWON
        4047 => "권성",            // STAR
        4049 => "소울링커",        // LINKER
        4190 => "슈퍼노비스",      // SUPERNOVICE2
        4211 => "KAGEROU",         // KAGEROU
        4212 => "OBORO",           // OBORO
        4215 => "REBELLION",       // REBELLION
        4222 => "닌자",            // NINJA_B
        4223 => "KAGEROU",         // KAGEROU_B
        4224 => "OBORO",           // OBORO_B
        4225 => "태권소년",        // TAEKWON_B
        4226 => "권성",            // STAR_B
        4227 => "소울링커",        // LINKER_B
        4228 => "건너",            // GUNSLINGER_B
        4229 => "REBELLION",       // REBELLION_B
        4239 => "성제",            // STAR EMPEROR
        4240 => "소울리퍼",        // SOUL REAPER
        4241 => "성제",            // STAR_EMPEROR_B
        4242 => "소울리퍼",        // SOUL_REAPER_B
        4252 => "DRAGON_KNIGHT",   // DRAGON KNIGHT
        4253 => "MEISTER",         // MEISTER
        4254 => "SHADOW_CROSS",    // SHADOW CROSS
        4255 => "ARCH_MAGE",       // ARCH MAGE
        4256 => "CARDINAL",        // CARDINAL
        4257 => "WINDHAWK",        // WINDHAWK
        4258 => "IMPERIAL_GUARD",  // IMPERIAL GUARD
        4259 => "BIOLO",           // BIOLO
        4260 => "ABYSS_CHASER",    // ABYSS CHASER
        4261 => "ELEMETAL_MASTER", // ELEMENTAL MASTER
        4262 => "INQUISITOR",      // INQUISITOR
        4263 => "TROUBADOUR",      // TROUBADOUR
        4264 => "TROUVERE",        // TROUVERE
        4302 => "SKY_EMPEROR",     // SKY EMPEROR
        4303 => "SOUL_ASCETIC",    // SOUL ASCETIC
        4304 => "SHINKIRO",        // SHINKIRO
        4305 => "SHIRANUI",        // SHIRANUI
        4306 => "NIGHT_WATCH",     // NIGHT WATCH
        4307 => "HYPER_NOVICE",    // HYPER NOVICE
        _ => "초보자",             // NOVICE,
    }
}

fn get_entity_part_files(library: &Library, entity_type: EntityType, job_id: usize, sex: Sex, head: Option<usize>) -> Vec<String> {
    let sex_sprite_path = match sex == Sex::Female {
        true => "여",
        false => "남",
    };

    fn player_body_path(sex_sprite_path: &str, job_id: usize) -> String {
        format!(
            "인간족\\몸통\\{}\\{}_{}",
            sex_sprite_path,
            get_sprite_path_for_player_job(job_id),
            sex_sprite_path
        )
    }

    fn player_head_path(sex_sprite_path: &str, head_id: usize) -> String {
        format!("인간족\\머리통\\{}\\{}_{}", sex_sprite_path, head_id, sex_sprite_path)
    }

    let head_id = match (sex, head) {
        (Sex::Male, Some(head)) if (0..MALE_HAIR_LOOKUP.len()).contains(&head) => MALE_HAIR_LOOKUP[head],
        (Sex::Male, Some(head)) => head,
        (Sex::Female, Some(head)) if (0..FEMALE_HAIR_LOOKUP.len()).contains(&head) => FEMALE_HAIR_LOOKUP[head],
        (Sex::Female, Some(head)) => head,
        _ => 1,
    };

    match entity_type {
        EntityType::Player => vec![
            player_body_path(sex_sprite_path, job_id),
            player_head_path(sex_sprite_path, head_id),
        ],
        EntityType::Npc => vec![format!("npc\\{}", library.get_job_identity_from_id(job_id))],
        EntityType::Monster => vec![format!("몬스터\\{}", library.get_job_identity_from_id(job_id))],
        EntityType::Warp | EntityType::Hidden => vec![format!("npc\\{}", library.get_job_identity_from_id(job_id))], // TODO: change
    }
}

impl Common {
    pub fn new(entity_data: &EntityData, tile_position: TilePosition, world_position: Point3<f32>, client_tick: ClientTick) -> Self {
        let entity_id = entity_data.entity_id;
        let job_id = entity_data.job as usize;
        let head_direction = entity_data.head_direction;
        let direction = entity_data.position.direction;

        let movement_speed = entity_data.movement_speed as usize;
        let health_points = entity_data.health_points as usize;
        let maximum_health_points = entity_data.maximum_health_points as usize;
        let sex = entity_data.sex;

        let active_movement = None;
        let entity_type = job_id.into();

        let details = ResourceState::Unavailable;
        let animation_state = AnimationState::new(entity_type, client_tick);

        Self {
            tile_position,
            world_position,
            entity_id,
            job_id,
            direction,
            head_direction,
            sex,
            active_movement,
            entity_type,
            movement_speed,
            health_points,
            maximum_health_points,
            animation_data: None,
            details,
            animation_state,
            stopped_moving: false,
            sound_state: SoundState::default(),
            fade_state: FadeState::FadingIn { start_time: client_tick },
        }
    }

    pub fn get_entity_part_files(&self, library: &Library) -> Vec<String> {
        get_entity_part_files(library, self.entity_type, self.job_id, self.sex, None)
    }

    pub fn update(&mut self, audio_engine: &AudioEngine<GameFileLoader>, map: &Map, camera: &dyn Camera, client_tick: ClientTick) {
        self.update_movement(map, client_tick);
        self.animation_state.update(client_tick);

        if let FadeState::FadingIn { .. } = self.fade_state
            && self.fade_state.is_complete(client_tick)
        {
            self.fade_state = FadeState::None;
        }

        if let Some(animation_data) = self.animation_data.as_ref() {
            if animation_data.is_animation_over(&self.animation_state) && self.animation_state.is_attack() {
                self.animation_state.idle(self.entity_type, client_tick);
            }

            let frame = animation_data.get_frame(&self.animation_state, camera, self.direction);

            match frame.event {
                Some(ActionEvent::Sound { key }) => {
                    self.sound_state.update(audio_engine, self.world_position, key, client_tick);
                }
                Some(ActionEvent::Attack) => {
                    // TODO: NHA What do we need to do at this event? Other
                    //       clients are playing the attackers weapon attack
                    //       sound using this event.
                }
                None | Some(ActionEvent::Unknown) => { /* Nothing to do */ }
            }
        }
    }

    fn update_movement(&mut self, map: &Map, client_tick: ClientTick) {
        self.stopped_moving = false;

        if let Some(active_movement) = self.active_movement.take() {
            let last_step = active_movement.steps.last().unwrap();

            if client_tick.0 > last_step.arrival_timestamp {
                self.set_position(map, last_step.arrival_position, client_tick);
                self.stopped_moving = true;
            } else {
                let mut last_step_index = 0;
                while active_movement.steps[last_step_index + 1].arrival_timestamp < client_tick.0 {
                    last_step_index += 1;
                }

                let last_step = active_movement.steps[last_step_index];
                let next_step = active_movement.steps[last_step_index + 1];

                self.tile_position = next_step.arrival_position;

                let last_step_position = Vector2::new(last_step.arrival_position.x as isize, last_step.arrival_position.y as isize);
                let next_step_position = Vector2::new(next_step.arrival_position.x as isize, next_step.arrival_position.y as isize);

                let array = last_step_position - next_step_position;
                let array: &[isize; 2] = array.as_ref();
                self.direction = (*array).try_into().unwrap();

                let Some(last_step_position) = map.get_world_position(last_step.arrival_position) else {
                    self.active_movement = active_movement.into();
                    return;
                };
                let Some(next_step_position) = map.get_world_position(next_step.arrival_position) else {
                    self.active_movement = active_movement.into();
                    return;
                };

                let clamped_tick = u32::max(last_step.arrival_timestamp, client_tick.0);
                let total = next_step.arrival_timestamp - last_step.arrival_timestamp;
                let offset = clamped_tick - last_step.arrival_timestamp;

                let movement_elapsed = (1.0 / total as f32) * offset as f32;
                let position = last_step_position.to_vec().lerp(next_step_position.to_vec(), movement_elapsed);

                self.world_position = Point3::from_vec(position);
                self.active_movement = active_movement.into();
            }
        }
    }

    fn set_position(&mut self, map: &Map, position: TilePosition, client_tick: ClientTick) {
        let Some(world_position) = map.get_world_position(position) else {
            #[cfg(feature = "debug")]
            korangar_debug::logging::print_debug!("[{}] entity position is out of map bounds", "error".red());
            return;
        };

        self.tile_position = position;
        self.world_position = world_position;
        self.active_movement = None;
        self.animation_state.idle(self.entity_type, client_tick);
    }

    pub fn move_from_to(
        &mut self,
        map: &Map,
        path_finder: &mut PathFinder,
        start: TilePosition,
        goal: TilePosition,
        starting_timestamp: ClientTick,
    ) {
        if let Some(path) = path_finder.find_walkable_path(map, start, goal) {
            if path.len() <= 1 {
                return;
            }

            let mut last_timestamp = starting_timestamp.0;
            let mut last_position: Option<TilePosition> = None;

            let steps: ArrayVec<Step, MAX_WALK_PATH_SIZE> = path
                .iter()
                .map(|&step| {
                    if let Some(position) = last_position {
                        const DIAGONAL_MULTIPLIER: f32 = 1.4;

                        let speed = match position.x == step.x || position.y == step.y {
                            // `true` means we are moving orthogonally
                            true => self.movement_speed as u32,
                            // `false` means we are moving diagonally
                            false => (self.movement_speed as f32 * DIAGONAL_MULTIPLIER) as u32,
                        };

                        let arrival_position = step;
                        let arrival_timestamp = last_timestamp + speed;

                        last_timestamp = arrival_timestamp;
                        last_position = Some(arrival_position);

                        Step {
                            arrival_position,
                            arrival_timestamp,
                        }
                    } else {
                        last_position = Some(start);

                        Step {
                            arrival_position: start,
                            arrival_timestamp: last_timestamp,
                        }
                    }
                })
                .collect();

            // If there is only a single step the player is already on the correct tile.
            if steps.len() > 1 {
                self.active_movement = Movement::new(steps, starting_timestamp.0).into();

                if !self.animation_state.is_walking() {
                    self.animation_state.walk(self.entity_type, self.movement_speed, starting_timestamp);
                }
            }
        }
    }

    #[cfg(feature = "debug")]
    fn pathing_texture_coordinates(steps: &[Step], step: Vector2<usize>, index: usize) -> ([Vector2<f32>; 4], i32) {
        if steps.len() - 1 == index {
            return (
                [
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(1.0, 0.0),
                ],
                0,
            );
        }

        let arrival_position = steps[index + 1].arrival_position;
        let delta = Vector2::new(
            arrival_position.x as isize - step.x as isize,
            arrival_position.y as isize - step.y as isize,
        );

        match delta {
            Vector2 { x: 1, y: 0 } => (
                [
                    Vector2::new(0.0, 0.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 1.0),
                ],
                1,
            ),
            Vector2 { x: -1, y: 0 } => (
                [
                    Vector2::new(1.0, 0.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(0.0, 1.0),
                ],
                1,
            ),
            Vector2 { x: 0, y: 1 } => (
                [
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(1.0, 1.0),
                ],
                1,
            ),
            Vector2 { x: 0, y: -1 } => (
                [
                    Vector2::new(1.0, 0.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                ],
                1,
            ),
            Vector2 { x: 1, y: 1 } => (
                [
                    Vector2::new(0.0, 1.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(1.0, 0.0),
                ],
                2,
            ),
            Vector2 { x: -1, y: 1 } => (
                [
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(1.0, 1.0),
                ],
                2,
            ),
            Vector2 { x: 1, y: -1 } => (
                [
                    Vector2::new(1.0, 1.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(0.0, 0.0),
                ],
                2,
            ),
            Vector2 { x: -1, y: -1 } => (
                [
                    Vector2::new(1.0, 0.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                ],
                2,
            ),
            _other => panic!("incorrent pathing"),
        }
    }

    #[cfg(feature = "debug")]
    pub fn generate_pathing_mesh(&mut self, device: &Device, queue: &Queue, bindless_support: BindlessSupport, map: &Map) {
        use crate::{Color, NativeModelVertex};

        const PATHING_MESH_OFFSET: f32 = 0.95;

        let mut pathing_native_vertices = Vec::new();

        let Some(active_movement) = self.active_movement.as_mut() else {
            return;
        };

        let mesh_color = match self.entity_type {
            EntityType::Player => Color::rgb_u8(25, 250, 225),
            EntityType::Npc => Color::rgb_u8(170, 250, 25),
            EntityType::Monster => Color::rgb_u8(250, 100, 25),
            _ => Color::WHITE,
        };

        for (index, Step { arrival_position, .. }) in active_movement.steps.iter().copied().enumerate() {
            let Some(tile) = map.get_tile(arrival_position) else {
                korangar_debug::logging::print_debug!("[{}] movement is out of map bounds", "error".red());
                continue;
            };

            let offset = Vector2::new(
                arrival_position.x as f32 * GAT_TILE_SIZE,
                arrival_position.y as f32 * GAT_TILE_SIZE,
            );

            let first_position = Point3::new(offset.x, tile.southwest_corner_height + PATHING_MESH_OFFSET, offset.y);
            let second_position = Point3::new(
                offset.x + GAT_TILE_SIZE,
                tile.southeast_corner_height + PATHING_MESH_OFFSET,
                offset.y,
            );
            let third_position = Point3::new(
                offset.x,
                tile.northwest_corner_height + PATHING_MESH_OFFSET,
                offset.y + GAT_TILE_SIZE,
            );
            let fourth_position = Point3::new(
                offset.x + GAT_TILE_SIZE,
                tile.northeast_corner_height + PATHING_MESH_OFFSET,
                offset.y + GAT_TILE_SIZE,
            );

            let first_normal = NativeModelVertex::calculate_normal(first_position, second_position, third_position);
            let second_normal = NativeModelVertex::calculate_normal(third_position, second_position, fourth_position);

            let (texture_coordinates, texture_index) = Self::pathing_texture_coordinates(
                &active_movement.steps,
                Vector2::new(arrival_position.x as usize, arrival_position.y as usize),
                index,
            );

            if let Some(first_normal) = first_normal {
                pathing_native_vertices.push(NativeModelVertex::new(
                    first_position,
                    first_normal,
                    texture_coordinates[0],
                    texture_index,
                    mesh_color,
                    0.0,
                    smallvec_inline![0; 3],
                ));
                pathing_native_vertices.push(NativeModelVertex::new(
                    second_position,
                    first_normal,
                    texture_coordinates[1],
                    texture_index,
                    mesh_color,
                    0.0,
                    smallvec_inline![0; 3],
                ));
                pathing_native_vertices.push(NativeModelVertex::new(
                    third_position,
                    first_normal,
                    texture_coordinates[2],
                    texture_index,
                    mesh_color,
                    0.0,
                    smallvec_inline![0; 3],
                ));
            }

            if let Some(second_normal) = second_normal {
                pathing_native_vertices.push(NativeModelVertex::new(
                    third_position,
                    second_normal,
                    texture_coordinates[2],
                    texture_index,
                    mesh_color,
                    0.0,
                    smallvec_inline![0; 3],
                ));
                pathing_native_vertices.push(NativeModelVertex::new(
                    second_position,
                    second_normal,
                    texture_coordinates[1],
                    texture_index,
                    mesh_color,
                    0.0,
                    smallvec_inline![0; 3],
                ));
                pathing_native_vertices.push(NativeModelVertex::new(
                    fourth_position,
                    second_normal,
                    texture_coordinates[3],
                    texture_index,
                    mesh_color,
                    0.0,
                    smallvec_inline![0; 3],
                ));
            }
        }

        let pathing_vertices = NativeModelVertex::convert_to_model_vertices(pathing_native_vertices, None);
        let (pathing_vertices, mut pathing_indices) = reduce_vertices(&pathing_vertices);

        let submeshes = match bindless_support {
            BindlessSupport::Full | BindlessSupport::Limited => {
                vec![SubMesh {
                    index_offset: 0,
                    index_count: pathing_indices.len() as u32,
                    base_vertex: 0,
                    texture_index: 0,
                    transparent: true,
                }]
            }
            BindlessSupport::None => split_mesh_by_texture(&pathing_vertices, &mut pathing_indices, None, None, None),
        };

        if let Some(pathing) = active_movement.pathing.as_mut() {
            pathing.vertex_buffer.write_exact(queue, pathing_vertices.as_slice());
            pathing.submeshes = submeshes;
        } else {
            let pathing_vertex_buffer = Arc::new(Buffer::with_data(
                device,
                queue,
                "pathing vertex buffer",
                BufferUsages::VERTEX | BufferUsages::COPY_DST,
                &pathing_vertices,
            ));

            let pathing_index_buffer = Arc::new(Buffer::with_data(
                device,
                queue,
                "pathing index buffer",
                BufferUsages::INDEX | BufferUsages::COPY_DST,
                &pathing_indices,
            ));

            active_movement.pathing = Some(Pathing {
                vertex_buffer: pathing_vertex_buffer,
                index_buffer: pathing_index_buffer,
                submeshes,
            });
        }
    }

    pub fn render(&self, instructions: &mut Vec<EntityInstruction>, camera: &dyn Camera, add_to_picker: bool, client_tick: ClientTick) {
        if let Some(animation_data) = self.animation_data.as_ref() {
            let fade_alpha = self.fade_state.calculate_alpha(client_tick);
            animation_data.render(
                instructions,
                camera,
                add_to_picker,
                self.entity_id,
                self.world_position,
                &self.animation_state,
                self.direction,
                fade_alpha,
            );
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_debug(&self, instructions: &mut Vec<DebugRectangleInstruction>, camera: &dyn Camera) {
        if let Some(animation_data) = self.animation_data.as_ref() {
            animation_data.render_debug(
                instructions,
                camera,
                self.world_position,
                &self.animation_state,
                self.direction,
                Color::rgb_u8(255, 0, 0),
                Color::rgb_u8(0, 255, 0),
            );
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_marker(
        &self,
        renderer: &mut impl MarkerRenderer,
        camera: &dyn Camera,
        marker_identifier: MarkerIdentifier,
        hovered: bool,
    ) {
        renderer.render_marker(camera, marker_identifier, self.world_position, hovered);
    }
}

#[derive(Clone, RustState, StateWindow)]
pub struct Player {
    common: Common,
    pub hair_id: usize,
    pub spell_points: usize,
    pub activity_points: usize,
    pub maximum_spell_points: usize,
    pub maximum_activity_points: usize,
    pub base_level: usize,
    pub job_level: usize,
    pub stat_points: u32,
    pub strength: i32,
    pub bonus_strength: i32,
    pub strength_stat_points_cost: u8,
    pub agility: i32,
    pub bonus_agility: i32,
    pub agility_stat_points_cost: u8,
    pub vitality: i32,
    pub bonus_vitality: i32,
    pub vitality_stat_points_cost: u8,
    pub intelligence: i32,
    pub bonus_intelligence: i32,
    pub intelligence_stat_points_cost: u8,
    pub dexterity: i32,
    pub bonus_dexterity: i32,
    pub dexterity_stat_points_cost: u8,
    pub luck: i32,
    pub bonus_luck: i32,
    pub luck_stat_points_cost: u8,
    pub attack_speed: u32,
}

impl Player {
    /// This function creates the player entity free-floating in the
    /// "void". When a new map is loaded on map change, the server sends
    /// the correct position we need to position the player to.
    pub fn new(account_id: AccountId, character_information: &CharacterInformation, client_tick: ClientTick) -> Self {
        let hair_id = character_information.head as usize;
        let spell_points = character_information.spell_points as usize;
        let activity_points = 0;
        let maximum_spell_points = character_information.maximum_spell_points as usize;
        let maximum_activity_points = 0;
        let base_level = character_information.base_level as usize;
        let job_level = character_information.job_level as usize;
        let stat_points = character_information.stat_points as u32;

        let entity_data = EntityData::from_character(account_id, character_information, WorldPosition::origin());
        let tile_position = TilePosition::new(0, 0);
        let position = Point3::origin();

        let mut common = Common::new(&entity_data, tile_position, position, client_tick);
        // Player's own character should not fade in.
        common.fade_state = FadeState::None;

        Self {
            common,
            hair_id,
            spell_points,
            activity_points,
            maximum_spell_points,
            maximum_activity_points,
            base_level,
            job_level,
            stat_points,
            strength: character_information.strength as i32,
            bonus_strength: 0,
            strength_stat_points_cost: 0,
            agility: character_information.agility as i32,
            bonus_agility: 0,
            agility_stat_points_cost: 0,
            vitality: character_information.vitality as i32,
            bonus_vitality: 0,
            vitality_stat_points_cost: 0,
            intelligence: character_information.intelligence as i32,
            bonus_intelligence: 0,
            intelligence_stat_points_cost: 0,
            dexterity: character_information.dexterity as i32,
            bonus_dexterity: 0,
            dexterity_stat_points_cost: 0,
            luck: character_information.luck as i32,
            bonus_luck: 0,
            luck_stat_points_cost: 0,
            attack_speed: 0,
        }
    }

    pub fn get_common(&self) -> &Common {
        &self.common
    }

    pub fn get_common_mut(&mut self) -> &mut Common {
        &mut self.common
    }

    pub fn update_stat(&mut self, stat_type: StatType) {
        match stat_type {
            StatType::MaximumHealthPoints(value) => self.common.maximum_health_points = value as usize,
            StatType::MaximumSpellPoints(value) => self.maximum_spell_points = value as usize,
            StatType::HealthPoints(value) => self.common.health_points = value as usize,
            StatType::SpellPoints(value) => self.spell_points = value as usize,
            StatType::ActivityPoints(value) => self.activity_points = value as usize,
            StatType::MaximumActivityPoints(value) => self.maximum_activity_points = value as usize,
            StatType::MovementSpeed(value) => self.common.movement_speed = value as usize,
            StatType::BaseLevel(value) => self.base_level = value as usize,
            StatType::JobLevel(value) => self.job_level = value as usize,
            StatType::StatPoints(stat_points) => self.stat_points = stat_points,
            StatType::Strength(base, bonus) => {
                self.strength = base;
                self.bonus_strength = bonus;
            }
            StatType::Agility(base, bonus) => {
                self.agility = base;
                self.bonus_agility = bonus;
            }
            StatType::Vitality(base, bonus) => {
                self.vitality = base;
                self.bonus_vitality = bonus;
            }
            StatType::Intelligence(base, bonus) => {
                self.intelligence = base;
                self.bonus_intelligence = bonus;
            }
            StatType::Dexterity(base, bonus) => {
                self.dexterity = base;
                self.bonus_dexterity = bonus;
            }
            StatType::Luck(base, bonus) => {
                self.luck = base;
                self.bonus_luck = bonus;
            }
            StatType::StrengthStatPointCost(cost) => self.strength_stat_points_cost = cost,
            StatType::AgilityStatPointCost(cost) => self.agility_stat_points_cost = cost,
            StatType::VitalityStatPointCost(cost) => self.vitality_stat_points_cost = cost,
            StatType::IntelligenceStatPointCost(cost) => self.intelligence_stat_points_cost = cost,
            StatType::DexterityStatPointCost(cost) => self.dexterity_stat_points_cost = cost,
            StatType::LuckStatPointCost(cost) => self.luck_stat_points_cost = cost,
            StatType::AttackSpeed(attack_speed) => self.attack_speed = attack_speed,
            _ => {}
        }
    }

    pub fn render_status(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, theme: &WorldTheme, window_size: ScreenSize) {
        let clip_space_position = camera.view_projection_matrix() * self.common.world_position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height + 5.0,
        };

        let bar_width = theme.status_bar.player_bar_width;
        let gap = theme.status_bar.gap;
        let total_height =
            theme.status_bar.health_height + theme.status_bar.spell_point_height + theme.status_bar.activity_point_height + gap * 2.0;

        let mut offset = 0.0;

        let background_position = final_position - theme.status_bar.border_size - ScreenSize::only_width(bar_width / 2.0);

        let background_size = ScreenSize {
            width: bar_width,
            height: total_height,
        } + theme.status_bar.border_size * 2.0;

        renderer.render_rectangle(background_position, background_size, theme.status_bar.background_color);

        renderer.render_bar(
            final_position,
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.health_height,
            },
            theme.status_bar.player_health_color,
            self.common.maximum_health_points as f32,
            self.common.health_points as f32,
        );

        offset += gap + theme.status_bar.health_height;

        renderer.render_bar(
            final_position + ScreenPosition::only_top(offset),
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.spell_point_height,
            },
            theme.status_bar.spell_point_color,
            self.maximum_spell_points as f32,
            self.spell_points as f32,
        );

        offset += gap + theme.status_bar.spell_point_height;

        renderer.render_bar(
            final_position + ScreenPosition::only_top(offset),
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.activity_point_height,
            },
            theme.status_bar.activity_point_color,
            self.maximum_activity_points as f32,
            self.activity_points as f32,
        );
    }

    pub fn get_entity_part_files(&self, library: &Library) -> Vec<String> {
        let common = self.get_common();
        get_entity_part_files(library, common.entity_type, common.job_id, common.sex, Some(self.hair_id))
    }
}

#[derive(Clone, RustState, StateWindow)]
pub struct Npc {
    common: Common,
}

impl Npc {
    pub fn new(map: &Map, path_finder: &mut PathFinder, entity_data: EntityData, client_tick: ClientTick) -> Option<Self> {
        let Some(position) = map.get_world_position(entity_data.position.tile_position()) else {
            #[cfg(feature = "debug")]
            korangar_debug::logging::print_debug!(
                "[{}] NPC with id {:?} is out of map bounds",
                "error".red(),
                entity_data.entity_id
            );
            return None;
        };

        let mut common = Common::new(&entity_data, entity_data.position.tile_position(), position, client_tick);

        if let Some(destination) = entity_data.destination {
            common.move_from_to(
                map,
                path_finder,
                entity_data.position.tile_position(),
                destination.tile_position(),
                client_tick,
            );
        }

        Some(Self { common })
    }

    pub fn get_common(&self) -> &Common {
        &self.common
    }

    pub fn get_common_mut(&mut self) -> &mut Common {
        &mut self.common
    }

    pub fn render_status(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, theme: &WorldTheme, window_size: ScreenSize) {
        if self.common.entity_type != EntityType::Monster {
            return;
        }

        let clip_space_position = camera.view_projection_matrix() * self.common.world_position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height + 5.0,
        };

        let bar_width = theme.status_bar.enemy_bar_width;

        renderer.render_rectangle(
            final_position - theme.status_bar.border_size - ScreenSize::only_width(bar_width / 2.0),
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.enemy_health_height,
            } + (theme.status_bar.border_size * 2.0),
            theme.status_bar.background_color,
        );

        renderer.render_bar(
            final_position,
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.enemy_health_height,
            },
            theme.status_bar.enemy_health_color,
            self.common.maximum_health_points as f32,
            self.common.health_points as f32,
        );
    }
}

#[derive(Clone, StateElement)]
pub enum Entity {
    Player(Player),
    Npc(Npc),
}

impl Entity {
    fn get_common(&self) -> &Common {
        match self {
            Self::Player(player) => player.get_common(),
            Self::Npc(npc) => npc.get_common(),
        }
    }

    fn get_common_mut(&mut self) -> &mut Common {
        match self {
            Self::Player(player) => player.get_common_mut(),
            Self::Npc(npc) => npc.get_common_mut(),
        }
    }

    pub fn get_entity_id(&self) -> EntityId {
        self.get_common().entity_id
    }

    pub fn get_entity_type(&self) -> EntityType {
        self.get_common().entity_type
    }

    pub fn get_fade_state(&self) -> FadeState {
        self.get_common().fade_state
    }

    pub fn set_fade_state(&mut self, fade_state: FadeState) {
        self.get_common_mut().fade_state = fade_state;
    }

    pub fn is_fading_out_complete(&self, client_tick: ClientTick) -> bool {
        matches!(self.get_common().fade_state, FadeState::FadingOut { .. }) && self.get_common().fade_state.is_complete(client_tick)
    }

    pub fn are_details_unavailable(&self) -> bool {
        match &self.get_common().details {
            ResourceState::Unavailable => true,
            _requested_or_available => false,
        }
    }

    pub fn set_job(&mut self, job_id: usize) {
        self.get_common_mut().job_id = job_id;
    }

    pub fn set_hair(&mut self, hair_id: usize) {
        if let Self::Player(player) = self {
            player.hair_id = hair_id
        }
    }

    pub fn set_animation_data(&mut self, animation_data: Arc<AnimationData>) {
        self.get_common_mut().animation_data = Some(animation_data)
    }

    pub fn get_entity_part_files(&self, library: &Library) -> Vec<String> {
        match self {
            Self::Player(player) => player.get_entity_part_files(library),
            Self::Npc(npc) => npc.get_common().get_entity_part_files(library),
        }
    }

    pub fn set_details_requested(&mut self) {
        self.get_common_mut().details = ResourceState::Requested;
    }

    pub fn set_details(&mut self, details: String) {
        self.get_common_mut().details = ResourceState::Available(details);
    }

    pub fn get_details(&self) -> Option<&String> {
        self.get_common().details.as_option()
    }

    pub fn get_tile_position(&self) -> TilePosition {
        self.get_common().tile_position
    }

    pub fn get_position(&self) -> Point3<f32> {
        self.get_common().world_position
    }

    pub fn set_position(&mut self, map: &Map, position: TilePosition, client_tick: ClientTick) {
        self.get_common_mut().set_position(map, position, client_tick);
    }

    pub fn set_dead(&mut self, client_tick: ClientTick) {
        let entity_type = self.get_entity_type();
        self.get_common_mut().animation_state.dead(entity_type, client_tick);
    }

    pub fn set_idle(&mut self, client_tick: ClientTick) {
        let entity_type = self.get_entity_type();
        self.get_common_mut().animation_state.idle(entity_type, client_tick);
    }

    pub fn rotate_towards(&mut self, target_position: TilePosition) {
        let common = self.get_common_mut();

        // FIX: This check is a little bit broken. This will prefer rotation diagonally
        // over rotating straight.
        if let Ok(direction) = Direction::try_from([
            (common.tile_position.x as isize - target_position.x as isize).clamp(-1, 1),
            (common.tile_position.y as isize - target_position.y as isize).clamp(-1, 1),
        ]) {
            common.direction = direction;
        }
    }

    pub fn set_attack(&mut self, attack_duration: u32, critical: bool, client_tick: ClientTick) {
        let entity_type = self.get_entity_type();
        self.get_common_mut()
            .animation_state
            .attack(entity_type, attack_duration, critical, client_tick);
    }

    pub fn stopped_moving(&self) -> bool {
        self.get_common().stopped_moving
    }

    pub fn stop_movement(&mut self) {
        self.get_common_mut().active_movement = None;
    }

    pub fn update_health(&mut self, health_points: usize, maximum_health_points: usize) {
        let common = self.get_common_mut();
        common.health_points = health_points;
        common.maximum_health_points = maximum_health_points;
    }

    pub fn update(&mut self, audio_engine: &AudioEngine<GameFileLoader>, map: &Map, camera: &dyn Camera, client_tick: ClientTick) {
        self.get_common_mut().update(audio_engine, map, camera, client_tick);
    }

    pub fn move_from_to(
        &mut self,
        map: &Map,
        path_finder: &mut PathFinder,
        from: TilePosition,
        to: TilePosition,
        starting_timestamp: ClientTick,
    ) {
        self.get_common_mut().move_from_to(map, path_finder, from, to, starting_timestamp);
    }

    #[cfg(feature = "debug")]
    pub fn generate_pathing_mesh(&mut self, device: &Device, queue: &Queue, bindless_support: BindlessSupport, map: &Map) {
        self.get_common_mut().generate_pathing_mesh(device, queue, bindless_support, map);
    }

    pub fn render(&self, instructions: &mut Vec<EntityInstruction>, camera: &dyn Camera, add_to_picker: bool, client_tick: ClientTick) {
        self.get_common().render(instructions, camera, add_to_picker, client_tick);
    }

    #[cfg(feature = "debug")]
    pub fn render_debug(&self, instructions: &mut Vec<DebugRectangleInstruction>, camera: &dyn Camera) {
        self.get_common().render_debug(instructions, camera);
    }

    #[cfg(feature = "debug")]
    pub fn get_pathing(&self) -> Option<&Pathing> {
        self.get_common()
            .active_movement
            .as_ref()
            .and_then(|movement| movement.pathing.as_ref())
    }

    #[cfg(feature = "debug")]
    pub fn render_marker(
        &self,
        renderer: &mut impl MarkerRenderer,
        camera: &dyn Camera,
        marker_identifier: MarkerIdentifier,
        hovered: bool,
    ) {
        self.get_common().render_marker(renderer, camera, marker_identifier, hovered);
    }

    pub fn render_status(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, theme: &WorldTheme, window_size: ScreenSize) {
        match self {
            Self::Player(player) => player.render_status(renderer, camera, theme, window_size),
            Self::Npc(npc) => npc.render_status(renderer, camera, theme, window_size),
        }
    }
}

impl VecItem for Entity {
    type Id = EntityId;

    fn get_id(&self) -> Self::Id {
        self.get_entity_id()
    }
}

// TODO: Derive this
impl StateWindow<ClientState> for Entity {
    fn to_window<'a>(_self_path: impl Path<ClientState, Self>) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Entity",
            theme: InterfaceThemeType::InGame,
            closable: true,
            // TODO: This is gonna be a bit hacky but we want to have this save path possibly be
            // None and dispaly a message if the entity disappeared.
            elements: (),
        }
    }

    fn to_window_mut<'a>(_self_path: impl Path<ClientState, Self>) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Entity",
            theme: InterfaceThemeType::InGame,
            closable: true,
            // TODO: This is gonna be a bit hacky but we want to have this save path possibly be
            // None and dispaly a message if the entity disappeared.
            elements: (),
        }
    }
}
