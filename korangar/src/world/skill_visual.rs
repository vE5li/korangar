use ragnarok_packets::{ClientTick, EffectId, SkillId, UnitId};

use crate::graphics::Color;

const FIRE_WALL_SKILL: SkillId = SkillId(18);
const FIRE_BOLT_SKILL: SkillId = SkillId(19);
const LIGHTNING_BOLT_SKILL: SkillId = SkillId(20);
const SIGHT_SKILL: SkillId = SkillId(10);
const FIRE_BALL_SKILL: SkillId = SkillId(17);
const HEAL_SKILL: SkillId = SkillId(28);
const BLESSING_SKILL: SkillId = SkillId(34);
const COLD_BOLT_SKILL: SkillId = SkillId(14);
const FROST_DIVER_SKILL: SkillId = SkillId(15);

pub const SIGHT_ATTACHMENT_KEY: u32 = SIGHT_SKILL.0 as u32;
pub const OPTION_SIGHT: u32 = 0x0000_0001;
pub const SKILL_SOUND_PATHS: &[&str] = &[
    "_heal_effect.wav",
    "effect\\ef_blessing.wav",
    "effect\\ef_fireball.wav",
    "effect\\ef_firehit.wav",
    "effect\\ef_firewall.wav",
    "effect\\ef_frostdiver.wav",
    "effect\\ef_frostdiver2.wav",
    "effect\\ef_icearrow.wav",
    "effect\\ef_lightbolt.wav",
    "effect\\ef_sight.wav",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillVisualAnchor {
    SourceEntity,
    DestinationEntity,
    GroundPosition,
    SkillUnit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkillEffectLight {
    pub offset: [f32; 3],
    pub color: Color,
    pub intensity: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkillVisualRecipe {
    pub effect_path: &'static str,
    pub sound_path: Option<&'static str>,
    pub sound_range: f32,
    pub anchor: SkillVisualAnchor,
    pub effect_offset: [f32; 3],
    pub light: Option<SkillEffectLight>,
    pub repeating: bool,
    pub hit_interval: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkillSpriteVisualRecipe {
    pub sprite_path: &'static str,
    pub action_path: &'static str,
    pub sound_path: Option<&'static str>,
    pub sound_range: f32,
    pub anchor: SkillVisualAnchor,
    pub attachment_key: Option<u32>,
    pub position_offset: [f32; 3],
    pub action_index: usize,
    pub repeating: bool,
    pub maximum_duration: Option<f32>,
    pub scaling: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkillSoundRecipe {
    pub sound_path: &'static str,
    pub sound_range: f32,
    pub anchor: SkillVisualAnchor,
    pub hit_interval: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillProceduralVisualKind {
    ColdBolt,
    ColdImpact,
    FrostDiver,
    FrostDiverPreview,
    FrostDiverImpact,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkillProceduralVisualRecipe {
    pub kind: SkillProceduralVisualKind,
    pub hit_interval: Option<f32>,
}

pub const FROST_DIVER_FOLLOWUP_DELAY: f32 = 0.64;

const FIRE_IMPACT_LIGHT: SkillEffectLight = SkillEffectLight {
    offset: [0.0, 6.0, 0.0],
    color: Color::rgb_u8(255, 72, 20),
    intensity: 45.0,
};

const FIRE_BOLT_IMPACT: SkillVisualRecipe = SkillVisualRecipe {
    effect_path: "firehit2.str",
    sound_path: None,
    sound_range: 55.0,
    anchor: SkillVisualAnchor::DestinationEntity,
    effect_offset: [0.0, 5.0, 0.0],
    light: Some(FIRE_IMPACT_LIGHT),
    repeating: false,
    hit_interval: Some(0.12),
};

const FIRE_WALL_IMPACT: SkillVisualRecipe = SkillVisualRecipe {
    sound_path: Some("effect\\ef_firehit.wav"),
    hit_interval: None,
    ..FIRE_BOLT_IMPACT
};

const FIRE_WALL_GROUND: SkillVisualRecipe = SkillVisualRecipe {
    effect_path: "firewall2.str",
    sound_path: Some("effect\\ef_firewall.wav"),
    sound_range: 65.0,
    anchor: SkillVisualAnchor::GroundPosition,
    effect_offset: [0.0, 0.0, 0.0],
    light: Some(FIRE_IMPACT_LIGHT),
    repeating: false,
    hit_interval: None,
};

const FIRE_WALL_UNIT: SkillVisualRecipe = SkillVisualRecipe {
    effect_path: "firewall.str",
    sound_path: None,
    sound_range: 65.0,
    anchor: SkillVisualAnchor::SkillUnit,
    effect_offset: [0.0, 0.0, 0.0],
    light: Some(SkillEffectLight {
        offset: [0.0, 6.0, 0.0],
        color: Color::rgb_u8(255, 30, 0),
        intensity: 60.0,
    }),
    repeating: true,
    hit_interval: None,
};

const PNEUMA_UNIT: SkillVisualRecipe = SkillVisualRecipe {
    effect_path: "pneuma1.str",
    sound_path: None,
    sound_range: 50.0,
    anchor: SkillVisualAnchor::SkillUnit,
    effect_offset: [0.0, 0.0, 0.0],
    light: Some(SkillEffectLight {
        offset: [0.0, 6.0, 0.0],
        color: Color::rgb_u8(83, 220, 108),
        intensity: 40.0,
    }),
    repeating: false,
    hit_interval: None,
};

const LIGHTNING_BOLT_IMPACT: SkillVisualRecipe = SkillVisualRecipe {
    effect_path: "lightning.str",
    sound_path: Some("effect\\ef_lightbolt.wav"),
    sound_range: 55.0,
    anchor: SkillVisualAnchor::DestinationEntity,
    effect_offset: [0.0, 5.0, 0.0],
    light: Some(SkillEffectLight {
        offset: [0.0, 8.0, 0.0],
        color: Color::rgb_u8(184, 210, 255),
        intensity: 50.0,
    }),
    repeating: false,
    hit_interval: None,
};

const HEAL_TARGET: SkillVisualRecipe = SkillVisualRecipe {
    effect_path: "recovery.str",
    sound_path: None,
    sound_range: 55.0,
    anchor: SkillVisualAnchor::DestinationEntity,
    effect_offset: [0.0, 0.0, 0.0],
    light: None,
    repeating: false,
    hit_interval: None,
};

const FIRE_BALL_IMPACT: SkillSpriteVisualRecipe = SkillSpriteVisualRecipe {
    sprite_path: "이팩트\\fireball.spr",
    action_path: "이팩트\\fireball.act",
    sound_path: Some("effect\\ef_fireball.wav"),
    sound_range: 60.0,
    anchor: SkillVisualAnchor::DestinationEntity,
    attachment_key: None,
    position_offset: [0.0, 5.0, 0.0],
    action_index: 0,
    repeating: false,
    maximum_duration: None,
    scaling: 1.0,
};

const BLESSING_TARGET: SkillSpriteVisualRecipe = SkillSpriteVisualRecipe {
    sprite_path: "이팩트\\축복.spr",
    action_path: "이팩트\\축복.act",
    sound_path: Some("effect\\ef_blessing.wav"),
    sound_range: 55.0,
    anchor: SkillVisualAnchor::DestinationEntity,
    attachment_key: None,
    position_offset: [0.0, 0.0, 0.0],
    action_index: 0,
    repeating: false,
    maximum_duration: None,
    scaling: 1.0,
};

const SIGHT_SOURCE: SkillSpriteVisualRecipe = SkillSpriteVisualRecipe {
    sprite_path: "이팩트\\sight.spr",
    action_path: "이팩트\\sight.act",
    sound_path: Some("effect\\ef_sight.wav"),
    sound_range: 55.0,
    anchor: SkillVisualAnchor::SourceEntity,
    attachment_key: Some(SIGHT_ATTACHMENT_KEY),
    position_offset: [0.0, 0.0, 0.0],
    action_index: 0,
    repeating: true,
    maximum_duration: Some(10.0),
    scaling: 1.0,
};

const HEAL_SOUND: SkillSoundRecipe = SkillSoundRecipe {
    sound_path: "_heal_effect.wav",
    sound_range: 55.0,
    anchor: SkillVisualAnchor::DestinationEntity,
    hit_interval: None,
};

const COLD_BOLT_SOUND: SkillSoundRecipe = SkillSoundRecipe {
    sound_path: "effect\\ef_icearrow.wav",
    sound_range: 55.0,
    anchor: SkillVisualAnchor::DestinationEntity,
    hit_interval: Some(0.12),
};

const FIRE_BOLT_SOUND: SkillSoundRecipe = SkillSoundRecipe {
    sound_path: "effect\\ef_firehit.wav",
    sound_range: 55.0,
    anchor: SkillVisualAnchor::DestinationEntity,
    hit_interval: Some(0.12),
};

const FIRE_HIT_DIRECT_SOUND: SkillSoundRecipe = SkillSoundRecipe {
    anchor: SkillVisualAnchor::SourceEntity,
    hit_interval: None,
    ..FIRE_BOLT_SOUND
};

const FROST_DIVER_SOUND: SkillSoundRecipe = SkillSoundRecipe {
    sound_path: "effect\\ef_frostdiver.wav",
    sound_range: 55.0,
    anchor: SkillVisualAnchor::SourceEntity,
    hit_interval: None,
};

const FROST_DIVER_HIT_SOUND: SkillSoundRecipe = SkillSoundRecipe {
    sound_path: "effect\\ef_frostdiver2.wav",
    sound_range: 55.0,
    anchor: SkillVisualAnchor::SourceEntity,
    hit_interval: None,
};

const FROST_DIVER_TARGET_HIT_SOUND: SkillSoundRecipe = SkillSoundRecipe {
    anchor: SkillVisualAnchor::DestinationEntity,
    ..FROST_DIVER_HIT_SOUND
};

const COLD_BOLT_PROCEDURAL: SkillProceduralVisualRecipe = SkillProceduralVisualRecipe {
    kind: SkillProceduralVisualKind::ColdBolt,
    hit_interval: Some(0.12),
};

const COLD_IMPACT_PROCEDURAL: SkillProceduralVisualRecipe = SkillProceduralVisualRecipe {
    kind: SkillProceduralVisualKind::ColdImpact,
    hit_interval: None,
};

const FROST_DIVER_PROCEDURAL: SkillProceduralVisualRecipe = SkillProceduralVisualRecipe {
    kind: SkillProceduralVisualKind::FrostDiver,
    hit_interval: None,
};

const FROST_DIVER_PREVIEW_PROCEDURAL: SkillProceduralVisualRecipe = SkillProceduralVisualRecipe {
    kind: SkillProceduralVisualKind::FrostDiverPreview,
    hit_interval: None,
};

const FROST_DIVER_IMPACT_PROCEDURAL: SkillProceduralVisualRecipe = SkillProceduralVisualRecipe {
    kind: SkillProceduralVisualKind::FrostDiverImpact,
    hit_interval: None,
};

pub fn skill_damage_visual(skill_id: SkillId) -> Option<SkillVisualRecipe> {
    match skill_id {
        FIRE_WALL_SKILL => Some(FIRE_WALL_IMPACT),
        FIRE_BOLT_SKILL => Some(FIRE_BOLT_IMPACT),
        LIGHTNING_BOLT_SKILL => Some(LIGHTNING_BOLT_IMPACT),
        _ => None,
    }
}

pub fn ground_skill_visual(skill_id: SkillId) -> Option<SkillVisualRecipe> {
    match skill_id {
        FIRE_WALL_SKILL => Some(FIRE_WALL_GROUND),
        _ => None,
    }
}

pub fn skill_damage_sprite_visual(skill_id: SkillId, action: i8) -> Option<SkillSpriteVisualRecipe> {
    match skill_id {
        // Splash victims receive their own damage packet. The classic client
        // plays the Fire Ball animation only for the primary impact. rAthena
        // sends DMG_SPLASH (5) or DMG_SPLASH_ENDURE (14) for surrounding
        // targets, while the primary target uses the normal single-hit types.
        FIRE_BALL_SKILL if !matches!(action, 5 | 14) => Some(FIRE_BALL_IMPACT),
        _ => None,
    }
}

pub fn no_damage_sprite_visual(skill_id: SkillId) -> Option<SkillSpriteVisualRecipe> {
    match skill_id {
        SIGHT_SKILL => Some(SIGHT_SOURCE),
        BLESSING_SKILL => Some(BLESSING_TARGET),
        _ => None,
    }
}

pub fn no_damage_skill_visual(skill_id: SkillId) -> Option<SkillVisualRecipe> {
    match skill_id {
        HEAL_SKILL => Some(HEAL_TARGET),
        _ => None,
    }
}

pub fn sight_sprite_visual() -> SkillSpriteVisualRecipe {
    SIGHT_SOURCE
}

pub fn no_damage_skill_sound(skill_id: SkillId) -> Option<SkillSoundRecipe> {
    match skill_id {
        HEAL_SKILL => Some(HEAL_SOUND),
        _ => None,
    }
}

pub fn should_display_heal_number(skill_id: SkillId) -> bool {
    skill_id == HEAL_SKILL
}

pub fn skill_damage_sound(skill_id: SkillId) -> Option<SkillSoundRecipe> {
    match skill_id {
        // The classic Cold Bolt visual is procedural and has no complete
        // STR/SPR/ACT asset, but its authoritative GRF sound is available.
        COLD_BOLT_SKILL => Some(COLD_BOLT_SOUND),
        FIRE_BOLT_SKILL => Some(FIRE_BOLT_SOUND),
        FROST_DIVER_SKILL => Some(FROST_DIVER_SOUND),
        _ => None,
    }
}

pub fn skill_damage_procedural_visual(skill_id: SkillId) -> Option<SkillProceduralVisualRecipe> {
    match skill_id {
        COLD_BOLT_SKILL => Some(COLD_BOLT_PROCEDURAL),
        FROST_DIVER_SKILL => Some(FROST_DIVER_PROCEDURAL),
        _ => None,
    }
}

pub fn skill_damage_followup_sound(skill_id: SkillId) -> Option<(SkillSoundRecipe, f32)> {
    match skill_id {
        FROST_DIVER_SKILL => Some((FROST_DIVER_TARGET_HIT_SOUND, FROST_DIVER_FOLLOWUP_DELAY)),
        _ => None,
    }
}

pub fn skill_damage_number_interval(skill_id: SkillId) -> Option<f32> {
    match skill_id {
        // Lightning Bolt's STR and WAV already contain the complete multi-hit
        // sequence, so only its damage numbers should be paced per hit.
        LIGHTNING_BOLT_SKILL => Some(0.12),
        _ => None,
    }
}

pub fn skill_effect_initial_delay(start_time: ClientTick, current_time: ClientTick) -> f32 {
    // Client ticks are wrapping u32 millisecond counters. Interpreting the
    // wrapped difference as signed preserves ordering across the wrap point;
    // packets whose timestamp is already in the past start immediately.
    (start_time.0.wrapping_sub(current_time.0) as i32).max(0) as f32 / 1000.0
}

pub fn skill_effect_repeat_delays(hit_interval: Option<f32>, hit_count: i16) -> Vec<f32> {
    let Some(hit_interval) = hit_interval else {
        return Vec::new();
    };
    let hit_count = usize::try_from(hit_count).unwrap_or(1).clamp(1, 32);
    (1..hit_count).map(|hit_index| hit_interval * hit_index as f32).collect()
}

pub fn skill_unit_visual(unit_id: UnitId) -> Option<SkillVisualRecipe> {
    match unit_id {
        UnitId::Firewall => Some(FIRE_WALL_UNIT),
        UnitId::Pneuma => Some(PNEUMA_UNIT),
        _ => None,
    }
}

pub fn special_effect_visual(effect_id: EffectId) -> Option<SkillVisualRecipe> {
    let mut recipe = match effect_id {
        EffectId::Firehit => FIRE_BOLT_IMPACT,
        EffectId::Firewall => FIRE_WALL_GROUND,
        EffectId::Lightbolt => LIGHTNING_BOLT_IMPACT,
        EffectId::Pneuma => PNEUMA_UNIT,
        EffectId::Heal => HEAL_TARGET,
        _ => return None,
    };

    // ZC_NOTIFY_EFFECT carries one entity id and no ground coordinate or
    // skill-unit lifecycle. Treat it as a one-shot entity effect even when the
    // same asset is also used by a persistent ground recipe.
    recipe.anchor = SkillVisualAnchor::SourceEntity;
    recipe.repeating = false;
    Some(recipe)
}

pub fn special_effect_sprite_visual(effect_id: EffectId) -> Option<SkillSpriteVisualRecipe> {
    let mut recipe = match effect_id {
        EffectId::Sight => SIGHT_SOURCE,
        EffectId::Fireball => FIRE_BALL_IMPACT,
        EffectId::Blessing => BLESSING_TARGET,
        _ => return None,
    };

    // ZC_NOTIFY_EFFECT has only one entity. Direct debug effects are one-shot
    // previews and must not claim the keyed lifecycle used by real Sight.
    recipe.anchor = SkillVisualAnchor::SourceEntity;
    recipe.attachment_key = None;
    recipe.repeating = false;
    recipe.maximum_duration = None;
    Some(recipe)
}

pub fn special_effect_sound(effect_id: EffectId) -> Option<SkillSoundRecipe> {
    match effect_id {
        EffectId::Firehit => Some(FIRE_HIT_DIRECT_SOUND),
        EffectId::Icearrow => Some(COLD_BOLT_SOUND),
        EffectId::Frostdiver => Some(FROST_DIVER_SOUND),
        EffectId::Frostdiver2 => Some(FROST_DIVER_HIT_SOUND),
        EffectId::Heal => Some(HEAL_SOUND),
        _ => None,
    }
}

pub fn special_effect_procedural_visual(effect_id: EffectId) -> Option<SkillProceduralVisualRecipe> {
    match effect_id {
        // EF_ICEARROW is intentionally absent: the reference effect is
        // sound-only. The actual Cold Bolt visual comes from SkillDamage.
        EffectId::Frostdiver => Some(FROST_DIVER_PREVIEW_PROCEDURAL),
        EffectId::Frostdiver2 => Some(FROST_DIVER_IMPACT_PROCEDURAL),
        EffectId::Coldhit => Some(COLD_IMPACT_PROCEDURAL),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targeted_bolts_attach_impacts_to_the_destination() {
        let fire_bolt = skill_damage_visual(FIRE_BOLT_SKILL).unwrap();
        let lightning_bolt = skill_damage_visual(LIGHTNING_BOLT_SKILL).unwrap();

        assert_eq!(fire_bolt.anchor, SkillVisualAnchor::DestinationEntity);
        assert_eq!(fire_bolt.effect_path, "firehit2.str");
        assert_eq!(fire_bolt.sound_path, None);
        assert_eq!(skill_damage_sound(FIRE_BOLT_SKILL), Some(FIRE_BOLT_SOUND));
        assert_eq!(lightning_bolt.anchor, SkillVisualAnchor::DestinationEntity);
        assert_eq!(lightning_bolt.hit_interval, None);
        assert_eq!(skill_damage_number_interval(LIGHTNING_BOLT_SKILL), Some(0.12));

        let cold_bolt = skill_damage_procedural_visual(COLD_BOLT_SKILL).unwrap();
        assert_eq!(cold_bolt.kind, SkillProceduralVisualKind::ColdBolt);
        assert_eq!(cold_bolt.hit_interval, Some(0.12));

        let frost_diver = skill_damage_procedural_visual(FROST_DIVER_SKILL).unwrap();
        assert_eq!(frost_diver.kind, SkillProceduralVisualKind::FrostDiver);
        assert_eq!(
            skill_damage_sound(FROST_DIVER_SKILL).unwrap().anchor,
            SkillVisualAnchor::SourceEntity
        );
        assert_eq!(
            skill_damage_followup_sound(FROST_DIVER_SKILL),
            Some((FROST_DIVER_TARGET_HIT_SOUND, FROST_DIVER_FOLLOWUP_DELAY))
        );
    }

    #[test]
    fn fire_ball_only_renders_for_the_primary_impact() {
        assert!(skill_damage_sprite_visual(FIRE_BALL_SKILL, 6).is_some());
        assert!(skill_damage_sprite_visual(FIRE_BALL_SKILL, 4).is_some());
        assert_eq!(skill_damage_sprite_visual(FIRE_BALL_SKILL, 5), None);
        assert_eq!(skill_damage_sprite_visual(FIRE_BALL_SKILL, 14), None);
    }

    #[test]
    fn support_and_self_skills_attach_to_the_affected_entity() {
        assert_eq!(
            no_damage_sprite_visual(BLESSING_SKILL).unwrap().anchor,
            SkillVisualAnchor::DestinationEntity
        );
        assert_eq!(no_damage_sprite_visual(SIGHT_SKILL).unwrap().maximum_duration, Some(10.0));
        assert_eq!(
            no_damage_skill_sound(HEAL_SKILL).unwrap().anchor,
            SkillVisualAnchor::DestinationEntity
        );
        assert!(should_display_heal_number(HEAL_SKILL));
        assert!(!should_display_heal_number(BLESSING_SKILL));
        assert_eq!(no_damage_skill_visual(HEAL_SKILL).unwrap().effect_path, "recovery.str");
    }

    #[test]
    fn multi_hit_recipes_are_paced_and_bounded() {
        let delays = skill_effect_repeat_delays(Some(0.12), 4);
        assert_eq!(delays.len(), 3);
        assert!((delays[0] - 0.12).abs() < f32::EPSILON);
        assert!((delays[1] - 0.24).abs() < f32::EPSILON);
        assert!((delays[2] - 0.36).abs() < 0.000_001);
        assert!(skill_effect_repeat_delays(None, 10).is_empty());
        assert!(skill_effect_repeat_delays(Some(0.12), -4).is_empty());
        assert_eq!(skill_effect_repeat_delays(Some(0.12), i16::MAX).len(), 31);
    }

    #[test]
    fn future_server_ticks_delay_effects_across_tick_wrap() {
        assert_eq!(skill_effect_initial_delay(ClientTick(1200), ClientTick(1000)), 0.2);
        assert_eq!(skill_effect_initial_delay(ClientTick(900), ClientTick(1000)), 0.0);
        assert_eq!(skill_effect_initial_delay(ClientTick(100), ClientTick(u32::MAX - 99)), 0.2);
    }

    #[test]
    fn fire_wall_separates_ground_unit_and_monster_impact_visuals() {
        let placement = ground_skill_visual(FIRE_WALL_SKILL).unwrap();
        let unit = skill_unit_visual(UnitId::Firewall).unwrap();
        let impact = skill_damage_visual(FIRE_WALL_SKILL).unwrap();

        assert_eq!(placement.anchor, SkillVisualAnchor::GroundPosition);
        assert_eq!(unit.anchor, SkillVisualAnchor::SkillUnit);
        assert!(unit.repeating);
        assert_eq!(impact.anchor, SkillVisualAnchor::DestinationEntity);
        assert_eq!(impact.sound_path, Some("effect\\ef_firehit.wav"));
        assert!(!impact.repeating);
    }

    #[test]
    fn unknown_skills_and_units_have_safe_visual_fallbacks() {
        assert_eq!(skill_damage_visual(SkillId(u16::MAX)), None);
        assert_eq!(ground_skill_visual(SkillId(u16::MAX)), None);
        assert_eq!(skill_unit_visual(UnitId(u32::MAX)), None);
    }

    #[test]
    fn direct_effect_debug_commands_reuse_verified_recipes() {
        assert!(special_effect_sprite_visual(EffectId::Sight).is_some());
        assert!(special_effect_sprite_visual(EffectId::Fireball).is_some());
        assert!(special_effect_sprite_visual(EffectId::Blessing).is_some());
        assert_eq!(
            special_effect_sound(EffectId::Firehit).unwrap().sound_path,
            "effect\\ef_firehit.wav"
        );
        assert_eq!(
            special_effect_sound(EffectId::Frostdiver2).unwrap().sound_path,
            "effect\\ef_frostdiver2.wav"
        );
        assert_eq!(special_effect_procedural_visual(EffectId::Icearrow), None);
        assert_eq!(
            special_effect_procedural_visual(EffectId::Frostdiver).unwrap().kind,
            SkillProceduralVisualKind::FrostDiverPreview
        );
        assert_eq!(
            special_effect_procedural_visual(EffectId::Frostdiver2).unwrap().kind,
            SkillProceduralVisualKind::FrostDiverImpact
        );
        assert_eq!(
            special_effect_procedural_visual(EffectId::Coldhit).unwrap().kind,
            SkillProceduralVisualKind::ColdImpact
        );
        assert_eq!(special_effect_visual(EffectId::Heal).unwrap().effect_path, "recovery.str");
    }
}
