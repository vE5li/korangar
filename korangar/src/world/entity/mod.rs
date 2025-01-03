use std::string::String;
use std::sync::Arc;

use arrayvec::ArrayVec;
use cgmath::{EuclideanSpace, Point3, Vector2, VectorSpace, Zero};
use derive_new::new;
use korangar_audio::{AudioEngine, SoundEffectKey};
use korangar_interface::elements::PrototypeElement;
use korangar_interface::windows::{PrototypeWindow, Window};
use korangar_networking::EntityData;
use korangar_util::pathing::{PathFinder, MAX_WALK_PATH_SIZE};
#[cfg(feature = "debug")]
use korangar_util::texture_atlas::AtlasAllocation;
use ragnarok_packets::{AccountId, CharacterInformation, ClientTick, Direction, EntityId, Sex, StatusType, WorldPosition};
#[cfg(feature = "debug")]
use wgpu::{BufferUsages, Device, Queue};

#[cfg(feature = "debug")]
use crate::graphics::DebugRectangleInstruction;
use crate::graphics::EntityInstruction;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ScreenPosition, ScreenSize};
use crate::interface::theme::GameTheme;
use crate::interface::windows::WindowCache;
use crate::loaders::GameFileLoader;
use crate::renderer::GameInterfaceRenderer;
#[cfg(feature = "debug")]
use crate::renderer::MarkerRenderer;
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;
use crate::world::{ActionEvent, AnimationActionType, AnimationData, AnimationState, Camera, Library, Map};
#[cfg(feature = "debug")]
use crate::{Buffer, Color, ModelVertex};

const MALE_HAIR_LOOKUP: &[usize] = &[2, 2, 1, 7, 5, 4, 3, 6, 8, 9, 10, 12, 11];
const FEMALE_HAIR_LOOKUP: &[usize] = &[2, 2, 4, 7, 1, 5, 3, 6, 12, 10, 9, 11, 8];
const SOUND_COOLDOWN_DURATION: u32 = 200;
const SPATIAL_SOUND_RANGE: f32 = 250.0;

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

#[derive(new, PrototypeElement)]
pub struct Movement {
    #[hidden_element]
    steps: ArrayVec<Step, MAX_WALK_PATH_SIZE>,
    starting_timestamp: u32,
    #[cfg(feature = "debug")]
    #[new(default)]
    #[hidden_element]
    pub pathing_vertex_buffer: Option<Arc<Buffer<ModelVertex>>>,
}

#[derive(Copy, Clone)]
pub struct Step {
    arrival_position: Vector2<usize>,
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

#[derive(PrototypeElement)]
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
    pub grid_position: Vector2<usize>,
    pub position: Point3<f32>,
    #[hidden_element]
    details: ResourceState<String>,
    #[hidden_element]
    animation_state: AnimationState,
    #[hidden_element]
    sound_state: SoundState,
}

#[cfg_attr(feature = "debug", korangar_debug::profile)]
#[allow(clippy::invisible_characters)]
fn get_sprite_path_for_player_job(job_id: usize) -> &'static str {
    match job_id {
        0 => "ÃÊº¸ÀÚ",             // NOVICE
        1 => "°Ë»Ç",               // SWORDMAN
        2 => "À§Àúµå",             // MAGICIAN
        3 => "±Ã¼Ö",               // ARCHER
        4 => "¼ºÁ÷ÀÚ",             // ACOLYTE
        5 => "»ÓÀÎ",               // MERCHANT
        6 => "µµµÏ",               // THIEF
        7 => "±â»ç",               // KNIGHT
        8 => "¼ºÅõ»ç",             // PRIEST
        9 => "¸¶¹Ý»Ç",             // WIZARD
        10 => "Á¦Ã¶°ø",            // BLACKSMITH
        11 => "ÇåÅÍ",              // HUNTER
        12 => "¾î¼¼½Å",            // ASSASSIN
        13 => "¿£´ë¿î",            // CHICKEN
        14 => "Å©·ç¼¼ÀÌ´õ",        // CRUSADER
        15 => "¸ùÅ©",              // MONK
        16 => "¼¼ÀÌÁö",            // SAGE
        17 => "·Î±×",              // ROGUE
        18 => "¿¬±Ý¼ú»ç",          // ALCHEMIST
        19 => "¹Ùµå",              // BARD
        20 => "¹«Èñ",              // DANCER
        23 => "½´ÆÛ³ëºñ½º",        // SUPERNOVICE
        24 => "°Ç³Ê",              // GUNSLINGER
        25 => "´ÑÀÚ",              // NINJA
        4001 => "ÃÊº¸ÀÚ",          // NOVICE_H
        4002 => "°Ë»Ç",            // SWORDMAN_H
        4003 => "À§Àúµå",          // MAGICIAN_H
        4004 => "±Ã¼Ö",            // ARCHER_H
        4005 => "¼ºÁ÷ÀÚ",          // ACOLYTE_H
        4006 => "»ÓÀÎ",            // MERCHANT_H
        4007 => "µµµÏ",            // THIEF_H
        4008 => "·Îµå³ªÀÌÆ®",      // KNIGHT_H
        4009 => "ÇÏÀÌÇÁ¸®",        // PRIEST_H
        4010 => "ÇÏÀÌÀ§Àúµå",      // WIZARD_H
        4011 => "È­ÀÌÆ®½º¹Ì½º",     // BLACKSMITH_H
        4012 => "½º³ªÀÌÆÛ",        // HUNTER_H
        4013 => "¾î½Ø½ÅÅ©·Î½º",    // ASSASSIN_H
        4014 => "¿£´ë¿î",          // CHICKEN_H
        4015 => "Å©·ç¼¼ÀÌ´õ",      // CRUSADER_H
        4016 => "¸ùÅ©",            // MONK_H
        4017 => "¼¼ÀÌÁö",          // SAGE_H
        4018 => "·Î±×",            // ROGUE_H
        4019 => "¿¬±Ý¼ú»ç",        // ALCHEMIST_H
        4020 => "¹Ùµå",            // BARD_H
        4021 => "¹«Èñ",            // DANCER_H
        4023 => "½´ÆÛ³ëºñ½º",      // NOVICE_B
        4024 => "°Ë»Ç",            // SWORDMAN_B
        4025 => "À§Àúµå",          // MAGICIAN_B
        4026 => "±Ã¼Ö",            // ARCHER_B
        4027 => "¼ºÁ÷ÀÚ",          // ACOLYTE_B
        4028 => "»ÓÀÎ",            // MERCHANT_B
        4029 => "µµµÏ",            // THIEF_B
        4030 => "±â»ç",            // KNIGHT_B
        4031 => "¼ºÅõ»ç",          // PRIEST_B
        4032 => "¸¶¹Ý»Ç",          // WIZARD_B
        4033 => "Á¦Ã¶°ø",          // BLACKSMITH_B
        4034 => "ÇåÅÍ",            // HUNTER_B
        4035 => "¾î¼¼½Å",          // ASSASSIN_B
        4037 => "Å©·ç¼¼ÀÌ´õ",      // CRUSADER_B
        4038 => "¸ùÅ©",            // MONK_B
        4039 => "¼¼ÀÌÁö",          // SAGE_B
        4040 => "·Î±×",            // ROGUE_B
        4041 => "¿¬±Ý¼ú»ç",        // ALCHEMIST_B
        4042 => "¹Ùµå",            // BARD_B
        4043 => "¹«Èñ",            // DANCER_B
        4045 => "½´ÆÛ³ëºñ½º",      // SUPERNOVICE_B
        4054 => "·é³ªÀÌÆ®",        // RUNE_KNIGHT
        4055 => "¿ö·Ï",            // WARLOCK
        4056 => "·¹ÀÎÁ®",          // RANGER
        4057 => "¾ÆÅ©ºñ¼ó",        // ARCH_BISHOP
        4058 => "¹ÌÄÉ´Ð",          // MECHANIC
        4059 => "±æ·ÎÆ¾Å©·Î½º",    // GUILLOTINE_CROSS
        4066 => "°¡µå",            // ROYAL_GUARD
        4067 => "¼Ò¼­·¯",           // SORCERER
        4068 => "¹Î½ºÆ®·²",        // MINSTREL
        4069 => "¿ø´õ·¯",          // WANDERER
        4070 => "½´¶ó",            // SURA
        4071 => "Á¦³×¸¯",          // GENETIC
        4072 => "½¦µµ¿ìÃ¼ÀÌ¼­",     // SHADOW_CHASER
        4060 => "·é³ªÀÌÆ®",        // RUNE_KNIGHT_H
        4061 => "¿ö·Ï",            // WARLOCK_H
        4062 => "·¹ÀÎÁ®",          // RANGER_H
        4063 => "¾ÆÅ©ºñ¼ó",        // ARCH_BISHOP_H
        4064 => "¹ÌÄÉ´Ð",          // MECHANIC_H
        4065 => "±æ·ÎÆ¾Å©·Î½º",    // GUILLOTINE_CROSS_H
        4073 => "°¡µÅ",            // ROYAL_GUARD_H
        4074 => "¼Ò¼­·¯",           // SORCERER_H
        4075 => "¹Î½ºÆ®·²",        // MINSTREL_H
        4076 => "¿ø´õ·¯",          // WANDERER_H
        4077 => "½´¶ó",            // SURA_H
        4078 => "Á¦³×¸¯",          // GENETIC_H
        4079 => "½¦µµ¿ìÃ¼ÀÌ¼­",     // SHADOW_CHASER_H
        4096 => "·é³ªÀÌÆ®",        // RUNE_KNIGHT_B
        4097 => "¿ö·Ï",            // WARLOCK_B
        4098 => "·¹ÀÎÁ®",          // RANGER_B
        4099 => "¾ÆÅ©ºñ¼ó",        // ARCHBISHOP_B
        4100 => "¹ÌÄÉ´Ð",          // MECHANIC_B
        4101 => "±æ·ÎÆ¾Å©·Î½º",    // GUILLOTINE_CROSS_B
        4102 => "°¡µå",            // ROYAL_GUARD_B
        4103 => "¼Ò¼­·¯",           // SORCERER_B
        4104 => "¹Î½ºÆ®·²",        // MINSTREL_B
        4105 => "¿ø´õ·¯",          // WANDERER_B
        4106 => "½´¶ó",            // SURA_B
        4107 => "Á¦³×¸¯",          // GENETIC_B
        4108 => "½¦µµ¿ìÃ¼ÀÌ¼­",     // SHADOW_CHASER_B
        4046 => "ÅÂ±Ç¼Ò³â",        // TAEKWON
        4047 => "±Ç¼º",            // STAR
        4049 => "¼Ò¿ï¸µÄ¿",        // LINKER
        4190 => "½´ÆÛ³ëºñ½º",      // SUPERNOVICE2
        4211 => "KAGEROU",         // KAGEROU
        4212 => "OBORO",           // OBORO
        4215 => "REBELLION",       // REBELLION
        4222 => "´ÑÀÚ",            // NINJA_B
        4223 => "KAGEROU",         // KAGEROU_B
        4224 => "OBORO",           // OBORO_B
        4225 => "ÅÂ±Ç¼Ò³â",        // TAEKWON_B
        4226 => "±Ç¼º",            // STAR_B
        4227 => "¼Ò¿ï¸µÄ¿",        // LINKER_B
        4228 => "°Ç³Ê",            // GUNSLINGER_B
        4229 => "REBELLION",       // REBELLION_B
        4239 => "¼ºÁ¦",            // STAR EMPEROR
        4240 => "¼Ò¿ï¸®ÆÛ",        // SOUL REAPER
        4241 => "¼ºÁ¦",            // STAR_EMPEROR_B
        4242 => "¼Ò¿ï¸®ÆÛ",        // SOUL_REAPER_B
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
        _ => "ÃÊº¸ÀÚ",             // NOVICE
    }
}

fn get_entity_part_files(library: &Library, entity_type: EntityType, job_id: usize, sex: Sex, head: Option<usize>) -> Vec<String> {
    let sex_sprite_path = match sex == Sex::Female {
        true => "¿©",
        false => "³²",
    };

    fn player_body_path(sex_sprite_path: &str, job_id: usize) -> String {
        format!(
            "ÀÎ°£Á·\\¸öÅë\\{}\\{}_{}",
            sex_sprite_path,
            get_sprite_path_for_player_job(job_id),
            sex_sprite_path
        )
    }

    fn player_head_path(sex_sprite_path: &str, head_id: usize) -> String {
        format!("ÀÎ°£Á·\\¸Ó¸®Åë\\{}\\{}_{}", sex_sprite_path, head_id, sex_sprite_path)
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
        EntityType::Monster => vec![format!("¸ó½ºÅÍ\\{}", library.get_job_identity_from_id(job_id))],
        EntityType::Warp | EntityType::Hidden => vec![format!("npc\\{}", library.get_job_identity_from_id(job_id))], // TODO: change
    }
}

impl Common {
    pub fn new(entity_data: &EntityData, grid_position: Vector2<usize>, position: Point3<f32>, client_tick: ClientTick) -> Self {
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
            grid_position,
            position,
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
            sound_state: SoundState::default(),
        }
    }

    pub fn get_entity_part_files(&self, library: &Library) -> Vec<String> {
        get_entity_part_files(library, self.entity_type, self.job_id, self.sex, None)
    }

    pub fn update(&mut self, audio_engine: &AudioEngine<GameFileLoader>, map: &Map, camera: &dyn Camera, client_tick: ClientTick) {
        self.update_movement(map, client_tick);
        self.animation_state.update(client_tick);

        if let Some(animation_data) = self.animation_data.as_ref() {
            let frame = animation_data.get_frame(&self.animation_state, camera, self.direction);
            match frame.event {
                Some(ActionEvent::Sound { key }) => {
                    self.sound_state.update(audio_engine, self.position, key, client_tick);
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
        if let Some(active_movement) = self.active_movement.take() {
            let last_step = active_movement.steps.last().unwrap();

            if client_tick.0 > last_step.arrival_timestamp {
                let position = Vector2::new(last_step.arrival_position.x, last_step.arrival_position.y);
                self.set_position(map, position, client_tick);
            } else {
                let mut last_step_index = 0;
                while active_movement.steps[last_step_index + 1].arrival_timestamp < client_tick.0 {
                    last_step_index += 1;
                }

                let last_step = active_movement.steps[last_step_index];
                let next_step = active_movement.steps[last_step_index + 1];

                let last_step_position = last_step.arrival_position.map(|value| value as isize);
                let next_step_position = next_step.arrival_position.map(|value| value as isize);

                let array = last_step_position - next_step_position;
                let array: &[isize; 2] = array.as_ref();
                self.direction = (*array).into();

                let last_step_position = map.get_world_position(last_step.arrival_position).to_vec();
                let next_step_position = map.get_world_position(next_step.arrival_position).to_vec();

                let clamped_tick = u32::max(last_step.arrival_timestamp, client_tick.0);
                let total = next_step.arrival_timestamp - last_step.arrival_timestamp;
                let offset = clamped_tick - last_step.arrival_timestamp;

                let movement_elapsed = (1.0 / total as f32) * offset as f32;
                let position = last_step_position.lerp(next_step_position, movement_elapsed);

                self.position = Point3::from_vec(position);
                self.active_movement = active_movement.into();
            }
        }
    }

    fn set_position(&mut self, map: &Map, position: Vector2<usize>, client_tick: ClientTick) {
        self.grid_position = position;
        self.position = map.get_world_position(position);
        self.active_movement = None;
        self.animation_state.idle(self.entity_type, client_tick);
    }

    pub fn move_from_to(
        &mut self,
        map: &Map,
        path_finder: &mut PathFinder,
        start: Vector2<usize>,
        goal: Vector2<usize>,
        starting_timestamp: ClientTick,
    ) {
        if let Some(path) = path_finder.find_walkable_path(map, start, goal) {
            if path.len() <= 1 {
                return;
            }

            let mut last_timestamp = starting_timestamp.0;
            let mut last_position: Option<Vector2<usize>> = None;

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

                if self.animation_state.action_type != AnimationActionType::Walk {
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
                    Vector2::new(1.0, 0.0),
                    Vector2::new(0.0, 0.0),
                ],
                0,
            );
        }

        let delta = steps[index + 1].arrival_position.map(|component| component as isize) - step.map(|component| component as isize);

        match delta {
            Vector2 { x: 1, y: 0 } => (
                [
                    Vector2::new(0.0, 0.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(0.0, 1.0),
                ],
                1,
            ),
            Vector2 { x: -1, y: 0 } => (
                [
                    Vector2::new(1.0, 0.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 1.0),
                ],
                1,
            ),
            Vector2 { x: 0, y: 1 } => (
                [
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(1.0, 0.0),
                ],
                1,
            ),
            Vector2 { x: 0, y: -1 } => (
                [
                    Vector2::new(1.0, 0.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(0.0, 0.0),
                ],
                1,
            ),
            Vector2 { x: 1, y: 1 } => (
                [
                    Vector2::new(0.0, 1.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(1.0, 1.0),
                ],
                2,
            ),
            Vector2 { x: -1, y: 1 } => (
                [
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(1.0, 0.0),
                ],
                2,
            ),
            Vector2 { x: 1, y: -1 } => (
                [
                    Vector2::new(1.0, 1.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                ],
                2,
            ),
            Vector2 { x: -1, y: -1 } => (
                [
                    Vector2::new(1.0, 0.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(0.0, 0.0),
                ],
                2,
            ),
            _other => panic!("incorrent pathing"),
        }
    }

    #[cfg(feature = "debug")]
    pub fn generate_pathing_mesh(&mut self, device: &Device, queue: &Queue, map: &Map, pathing_mapping: &[AtlasAllocation]) {
        use crate::{Color, NativeModelVertex, MAP_TILE_SIZE};

        const HALF_TILE_SIZE: f32 = MAP_TILE_SIZE / 2.0;
        const PATHING_MESH_OFFSET: f32 = 0.95;

        let mut native_pathing_vertices = Vec::new();
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
            let tile = map.get_tile(arrival_position);
            let offset = Vector2::new(
                arrival_position.x as f32 * HALF_TILE_SIZE,
                arrival_position.y as f32 * HALF_TILE_SIZE,
            );

            let first_position = Point3::new(offset.x, tile.upper_left_height + PATHING_MESH_OFFSET, offset.y);
            let second_position = Point3::new(
                offset.x + HALF_TILE_SIZE,
                tile.upper_right_height + PATHING_MESH_OFFSET,
                offset.y,
            );
            let third_position = Point3::new(
                offset.x + HALF_TILE_SIZE,
                tile.lower_right_height + PATHING_MESH_OFFSET,
                offset.y + HALF_TILE_SIZE,
            );
            let fourth_position = Point3::new(
                offset.x,
                tile.lower_left_height + PATHING_MESH_OFFSET,
                offset.y + HALF_TILE_SIZE,
            );

            let first_normal = NativeModelVertex::calculate_normal(first_position, second_position, third_position);
            let second_normal = NativeModelVertex::calculate_normal(fourth_position, first_position, third_position);

            let (texture_coordinates, texture_index) = Self::pathing_texture_coordinates(&active_movement.steps, arrival_position, index);

            native_pathing_vertices.push(NativeModelVertex::new(
                first_position,
                first_normal,
                texture_coordinates[0],
                texture_index,
                mesh_color,
                0.0,
            ));
            native_pathing_vertices.push(NativeModelVertex::new(
                second_position,
                first_normal,
                texture_coordinates[1],
                texture_index,
                mesh_color,
                0.0,
            ));
            native_pathing_vertices.push(NativeModelVertex::new(
                third_position,
                first_normal,
                texture_coordinates[2],
                texture_index,
                mesh_color,
                0.0,
            ));

            native_pathing_vertices.push(NativeModelVertex::new(
                first_position,
                second_normal,
                texture_coordinates[0],
                texture_index,
                mesh_color,
                0.0,
            ));
            native_pathing_vertices.push(NativeModelVertex::new(
                third_position,
                second_normal,
                texture_coordinates[2],
                texture_index,
                mesh_color,
                0.0,
            ));
            native_pathing_vertices.push(NativeModelVertex::new(
                fourth_position,
                second_normal,
                texture_coordinates[3],
                texture_index,
                mesh_color,
                0.0,
            ));
        }

        let pathing_vertices = NativeModelVertex::to_vertices(native_pathing_vertices, pathing_mapping);

        if let Some(steps_vertex_buffer) = &active_movement.pathing_vertex_buffer {
            steps_vertex_buffer.write_exact(queue, pathing_vertices.as_slice());
        } else {
            let vertex_buffer = Arc::new(Buffer::with_data(
                device,
                queue,
                "pathing vertex buffer",
                BufferUsages::VERTEX | BufferUsages::COPY_DST,
                &pathing_vertices,
            ));

            active_movement.pathing_vertex_buffer = Some(vertex_buffer);
        }
    }

    pub fn render(&self, instructions: &mut Vec<EntityInstruction>, camera: &dyn Camera, add_to_picker: bool) {
        if let Some(animation_data) = self.animation_data.as_ref() {
            animation_data.render(
                instructions,
                camera,
                add_to_picker,
                self.entity_id,
                self.position,
                &self.animation_state,
                self.direction,
            );
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_debug(&self, instructions: &mut Vec<DebugRectangleInstruction>, camera: &dyn Camera) {
        if let Some(animation_data) = self.animation_data.as_ref() {
            animation_data.render_debug(
                instructions,
                camera,
                self.position,
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
        renderer.render_marker(camera, marker_identifier, self.position, hovered);
    }
}

#[derive(PrototypeWindow)]
pub struct Player {
    common: Common,
    pub hair_id: usize,
    pub spell_points: usize,
    pub activity_points: usize,
    pub maximum_spell_points: usize,
    pub maximum_activity_points: usize,
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

        let entity_data = EntityData::from_character(account_id, character_information, WorldPosition::origin());
        let grid_position = Vector2::zero();
        let position = Point3::origin();

        let common = Common::new(&entity_data, grid_position, position, client_tick);

        Self {
            common,
            hair_id,
            spell_points,
            activity_points,
            maximum_spell_points,
            maximum_activity_points,
        }
    }

    pub fn get_common(&self) -> &Common {
        &self.common
    }

    pub fn get_common_mut(&mut self) -> &mut Common {
        &mut self.common
    }

    pub fn update_status(&mut self, status_type: StatusType) {
        match status_type {
            StatusType::MaximumHealthPoints(value) => self.common.maximum_health_points = value as usize,
            StatusType::MaximumSpellPoints(value) => self.maximum_spell_points = value as usize,
            StatusType::HealthPoints(value) => self.common.health_points = value as usize,
            StatusType::SpellPoints(value) => self.spell_points = value as usize,
            StatusType::ActivityPoints(value) => self.activity_points = value as usize,
            StatusType::MaximumActivityPoints(value) => self.maximum_activity_points = value as usize,
            _ => {}
        }
    }

    pub fn render_status(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, theme: &GameTheme, window_size: ScreenSize) {
        let clip_space_position = camera.view_projection_matrix() * self.common.position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height + 5.0,
        };

        let bar_width = theme.status_bar.player_bar_width.get();
        let gap = theme.status_bar.gap.get();
        let total_height = theme.status_bar.health_height.get()
            + theme.status_bar.spell_point_height.get()
            + theme.status_bar.activity_point_height.get()
            + gap * 2.0;

        let mut offset = 0.0;

        let background_position = final_position - theme.status_bar.border_size.get() - ScreenSize::only_width(bar_width / 2.0);

        let background_size = ScreenSize {
            width: bar_width,
            height: total_height,
        } + theme.status_bar.border_size.get() * 2.0;

        renderer.render_rectangle(background_position, background_size, theme.status_bar.background_color.get());

        renderer.render_bar(
            final_position,
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.health_height.get(),
            },
            theme.status_bar.player_health_color.get(),
            self.common.maximum_health_points as f32,
            self.common.health_points as f32,
        );

        offset += gap + theme.status_bar.health_height.get();

        renderer.render_bar(
            final_position + ScreenPosition::only_top(offset),
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.spell_point_height.get(),
            },
            theme.status_bar.spell_point_color.get(),
            self.maximum_spell_points as f32,
            self.spell_points as f32,
        );

        offset += gap + theme.status_bar.spell_point_height.get();

        renderer.render_bar(
            final_position + ScreenPosition::only_top(offset),
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.activity_point_height.get(),
            },
            theme.status_bar.activity_point_color.get(),
            self.maximum_activity_points as f32,
            self.activity_points as f32,
        );
    }

    pub fn get_entity_part_files(&self, library: &Library) -> Vec<String> {
        let common = self.get_common();
        get_entity_part_files(library, common.entity_type, common.job_id, common.sex, Some(self.hair_id))
    }
}

#[derive(PrototypeWindow)]
pub struct Npc {
    common: Common,
}

impl Npc {
    pub fn new(map: &Map, entity_data: EntityData, client_tick: ClientTick) -> Self {
        let grid_position = Vector2::new(entity_data.position.x, entity_data.position.y);
        let position = map.get_world_position(grid_position);

        let mut common = Common::new(&entity_data, grid_position, position, client_tick);

        if let Some(destination) = entity_data.destination {
            let mut path_finder = PathFinder::default();
            let position_from = Vector2::new(entity_data.position.x, entity_data.position.y);
            let position_to = Vector2::new(destination.x, destination.y);
            common.move_from_to(map, &mut path_finder, position_from, position_to, client_tick);
        }

        Self { common }
    }

    pub fn get_common(&self) -> &Common {
        &self.common
    }

    pub fn get_common_mut(&mut self) -> &mut Common {
        &mut self.common
    }

    pub fn render_status(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, theme: &GameTheme, window_size: ScreenSize) {
        if self.common.entity_type != EntityType::Monster {
            return;
        }

        let clip_space_position = camera.view_projection_matrix() * self.common.position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height + 5.0,
        };

        let bar_width = theme.status_bar.enemy_bar_width.get();

        renderer.render_rectangle(
            final_position - theme.status_bar.border_size.get() - ScreenSize::only_width(bar_width / 2.0),
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.enemy_health_height.get(),
            } + (theme.status_bar.border_size.get() * 2.0),
            theme.status_bar.background_color.get(),
        );

        renderer.render_bar(
            final_position,
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.enemy_health_height.get(),
            },
            theme.status_bar.enemy_health_color.get(),
            self.common.maximum_health_points as f32,
            self.common.health_points as f32,
        );
    }
}

// TODO:
//#[derive(PrototypeWindow)]
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

    pub fn get_grid_position(&self) -> Vector2<usize> {
        self.get_common().grid_position
    }

    pub fn get_position(&self) -> Point3<f32> {
        self.get_common().position
    }

    pub fn set_position(&mut self, map: &Map, position: Vector2<usize>, client_tick: ClientTick) {
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
        from: Vector2<usize>,
        to: Vector2<usize>,
        starting_timestamp: ClientTick,
    ) {
        self.get_common_mut().move_from_to(map, path_finder, from, to, starting_timestamp);
    }

    #[cfg(feature = "debug")]
    pub fn generate_pathing_mesh(&mut self, device: &Device, queue: &Queue, map: &Map, pathing_texture_mapping: &[AtlasAllocation]) {
        self.get_common_mut()
            .generate_pathing_mesh(device, queue, map, pathing_texture_mapping);
    }

    pub fn render(&self, instructions: &mut Vec<EntityInstruction>, camera: &dyn Camera, add_to_picker: bool) {
        self.get_common().render(instructions, camera, add_to_picker);
    }

    #[cfg(feature = "debug")]
    pub fn render_debug(&self, instructions: &mut Vec<DebugRectangleInstruction>, camera: &dyn Camera) {
        self.get_common().render_debug(instructions, camera);
    }

    #[cfg(feature = "debug")]
    pub fn get_pathing_vertex_buffer(&self) -> Option<&Arc<Buffer<ModelVertex>>> {
        self.get_common()
            .active_movement
            .as_ref()
            .and_then(|movement| movement.pathing_vertex_buffer.as_ref())
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

    pub fn render_status(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, theme: &GameTheme, window_size: ScreenSize) {
        match self {
            Self::Player(player) => player.render_status(renderer, camera, theme, window_size),
            Self::Npc(npc) => npc.render_status(renderer, camera, theme, window_size),
        }
    }
}

impl PrototypeWindow<InterfaceSettings> for Entity {
    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        match self {
            Entity::Player(player) => player.to_window(window_cache, application, available_space),
            Entity::Npc(npc) => npc.to_window(window_cache, application, available_space),
        }
    }
}
