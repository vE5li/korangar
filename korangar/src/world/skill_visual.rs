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
/// Launch sounds of the Fire Bolt projectile. The official client picks one
/// at random per bolt.
pub const FIRE_ARROW_LAUNCH_SOUND_PATHS: [&str; 3] = [
    "effect\\ef_firearrow1.wav",
    "effect\\ef_firearrow2.wav",
    "effect\\ef_firearrow3.wav",
];

/// Launch sounds of the Cold Bolt projectile. The official client attaches
/// these to the projectile rather than to the hit, and picks one per bolt.
pub const ICE_ARROW_LAUNCH_SOUND_PATHS: [&str; 3] = ["effect\\ef_icearrow1.wav", "effect\\ef_icearrow2.wav", "effect\\ef_icearrow3.wav"];

pub const SKILL_SOUND_PATHS: &[&str] = &[
    "_heal_effect.wav",
    "effect\\ef_blessing.wav",
    "effect\\ef_fireball.wav",
    "effect\\ef_firearrow1.wav",
    "effect\\ef_firearrow2.wav",
    "effect\\ef_firearrow3.wav",
    "effect\\ef_firehit.wav",
    "effect\\ef_firewall.wav",
    "effect\\ef_frostdiver.wav",
    "effect\\ef_frostdiver2.wav",
    "effect\\ef_icearrow1.wav",
    "effect\\ef_icearrow2.wav",
    "effect\\ef_icearrow3.wav",
    "effect\\ef_lightbolt.wav",
    "effect\\ef_sight.wav",
    "_hit_fist3.wav",
    "_hit_fist4.wav",
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
    /// Interchangeable alternatives to `effect_path`. The official client
    /// varies several impact animations per hit rather than replaying one.
    /// Empty means `effect_path` is always used.
    pub effect_path_variants: &'static [&'static str],
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
    /// Interchangeable alternatives to `sound_path`, picked per hit. Empty
    /// means `sound_path` is always used.
    pub sound_path_variants: &'static [&'static str],
    pub sound_range: f32,
    pub anchor: SkillVisualAnchor,
    pub hit_interval: Option<f32>,
}

/// Picks one of `variants` using a roll in `[0, 1)`, falling back to `single`
/// when there are no variants.
///
/// The roll is a parameter rather than drawn inside so the selection is
/// testable and so a caller can keep one roll consistent across the assets of
/// a single hit.
pub fn pick_variant(single: &'static str, variants: &'static [&'static str], roll: f32) -> &'static str {
    if variants.is_empty() {
        return single;
    }

    let index = (roll.clamp(0.0, 1.0) * variants.len() as f32) as usize;
    variants[index.min(variants.len() - 1)]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillProceduralVisualKind {
    ColdBolt,
    ColdImpact,
    FireBoltProjectile,
    FrostDiver,
    FrostDiverPreview,
    FrostDiverImpact,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkillProceduralVisualRecipe {
    pub kind: SkillProceduralVisualKind,
    pub hit_interval: Option<f32>,
    /// Whether this visual is a projectile that must land before the hit
    /// resolves. Hit feedback for such skills is delayed by the projectile's
    /// flight time so the impact never precedes the projectile.
    pub leads_impact: bool,
}

pub const FROST_DIVER_FOLLOWUP_DELAY: f32 = 0.64;

/// Bounds applied to the server's attack motion when it is reused as a
/// projectile flight time.
///
/// `ZC_NOTIFY_SKILL` carries rAthena's `sdelay`, which is the caster's attack
/// motion and therefore ASPD dependent. It is authoritative for when the hit
/// resolves, but unbounded as a flight duration: zero would leave the
/// projectile invisible and a slow caster would leave it hanging in the air.
pub const PROJECTILE_FLIGHT_MINIMUM: f32 = 0.18;
pub const PROJECTILE_FLIGHT_MAXIMUM: f32 = 0.60;

/// Flight time of a projectile whose hit is scheduled `source_motion`
/// milliseconds after the packet.
pub fn skill_projectile_flight_time(source_motion: i32) -> f32 {
    (source_motion.max(0) as f32 / 1000.0).clamp(PROJECTILE_FLIGHT_MINIMUM, PROJECTILE_FLIGHT_MAXIMUM)
}

/// Delay that hit feedback must wait so it resolves after the projectile
/// lands. Skills without a leading projectile are unaffected.
pub fn skill_impact_lead_time(recipe: Option<SkillProceduralVisualRecipe>, source_motion: i32) -> f32 {
    match recipe {
        Some(recipe) if recipe.leads_impact => skill_projectile_flight_time(source_motion),
        _ => 0.0,
    }
}

const FIRE_IMPACT_LIGHT: SkillEffectLight = SkillEffectLight {
    offset: [0.0, 6.0, 0.0],
    color: Color::rgb_u8(255, 72, 20),
    intensity: 45.0,
};

/// Spacing between the hits of a Fire Bolt volley.
///
/// This is the official client's `PLUSATTACKED_MOTIONTIME`. Both reference
/// implementations pace multi-hit feedback at 200ms.
const BOLT_HIT_INTERVAL: f32 = 0.20;

/// The official client varies the fire impact per hit instead of replaying
/// one animation. Fire Wall's hit shares the same effect, so it inherits
/// these through `FIRE_WALL_IMPACT`.
const FIRE_HIT_VARIANTS: &[&str] = &["firehit1.str", "firehit2.str", "firehit3.str"];

const FIRE_BOLT_IMPACT: SkillVisualRecipe = SkillVisualRecipe {
    effect_path: "firehit2.str",
    effect_path_variants: FIRE_HIT_VARIANTS,
    sound_path: None,
    sound_range: 55.0,
    anchor: SkillVisualAnchor::DestinationEntity,
    effect_offset: [0.0, 5.0, 0.0],
    light: Some(FIRE_IMPACT_LIGHT),
    repeating: false,
    hit_interval: Some(BOLT_HIT_INTERVAL),
};

const FIRE_WALL_IMPACT: SkillVisualRecipe = SkillVisualRecipe {
    sound_path: Some("effect\\ef_firehit.wav"),
    hit_interval: None,
    ..FIRE_BOLT_IMPACT
};

const FIRE_WALL_GROUND: SkillVisualRecipe = SkillVisualRecipe {
    effect_path: "firewall2.str",
    effect_path_variants: &[],
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
    effect_path_variants: &[],
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
    effect_path_variants: &[],
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
    effect_path_variants: &[],
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
    effect_path_variants: &[],
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
    sound_path_variants: &[],
    sound_range: 55.0,
    anchor: SkillVisualAnchor::DestinationEntity,
    hit_interval: None,
};

/// `EF_ICEARROW` fired as a direct effect is the projectile's own sound.
const ICE_ARROW_DIRECT_SOUND: SkillSoundRecipe = SkillSoundRecipe {
    sound_path: "effect\\ef_icearrow1.wav",
    sound_path_variants: &ICE_ARROW_LAUNCH_SOUND_PATHS,
    sound_range: 55.0,
    anchor: SkillVisualAnchor::DestinationEntity,
    hit_interval: None,
};

/// Cold Bolt's impact sound.
///
/// The ice-arrow sounds belong to the projectile, not the hit: the official
/// client attaches them to `ef_coldbolt` and plays a generic elemental hit on
/// arrival. Those launch sounds now live on the projectile art.
const COLD_BOLT_IMPACT_SOUND: SkillSoundRecipe = SkillSoundRecipe {
    sound_path: "_hit_fist3.wav",
    sound_path_variants: &["_hit_fist3.wav", "_hit_fist4.wav"],
    sound_range: 55.0,
    anchor: SkillVisualAnchor::DestinationEntity,
    hit_interval: Some(BOLT_HIT_INTERVAL),
};

const FIRE_BOLT_SOUND: SkillSoundRecipe = SkillSoundRecipe {
    sound_path: "effect\\ef_firehit.wav",
    sound_path_variants: &[],
    sound_range: 55.0,
    anchor: SkillVisualAnchor::DestinationEntity,
    hit_interval: Some(BOLT_HIT_INTERVAL),
};

const FIRE_HIT_DIRECT_SOUND: SkillSoundRecipe = SkillSoundRecipe {
    anchor: SkillVisualAnchor::SourceEntity,
    hit_interval: None,
    ..FIRE_BOLT_SOUND
};

const FROST_DIVER_SOUND: SkillSoundRecipe = SkillSoundRecipe {
    sound_path: "effect\\ef_frostdiver.wav",
    sound_path_variants: &[],
    sound_range: 55.0,
    anchor: SkillVisualAnchor::SourceEntity,
    hit_interval: None,
};

const FROST_DIVER_HIT_SOUND: SkillSoundRecipe = SkillSoundRecipe {
    sound_path: "effect\\ef_frostdiver2.wav",
    sound_path_variants: &[],
    sound_range: 55.0,
    anchor: SkillVisualAnchor::SourceEntity,
    hit_interval: None,
};

const FROST_DIVER_TARGET_HIT_SOUND: SkillSoundRecipe = SkillSoundRecipe {
    anchor: SkillVisualAnchor::DestinationEntity,
    ..FROST_DIVER_HIT_SOUND
};

/// Cold Bolt is the same falling-projectile shape as Fire Bolt, so it shares
/// the cadence and the leading behaviour.
const COLD_BOLT_PROCEDURAL: SkillProceduralVisualRecipe = SkillProceduralVisualRecipe {
    kind: SkillProceduralVisualKind::ColdBolt,
    hit_interval: Some(BOLT_HIT_INTERVAL),
    leads_impact: true,
};

const COLD_IMPACT_PROCEDURAL: SkillProceduralVisualRecipe = SkillProceduralVisualRecipe {
    kind: SkillProceduralVisualKind::ColdImpact,
    hit_interval: None,
    leads_impact: false,
};

/// Fire Bolt's projectile stage.
///
/// The official client models Fire Bolt as a projectile that falls onto the
/// target followed by a separate impact animation, so the projectile leads
/// the hit rather than sharing its schedule.
const FIRE_BOLT_PROJECTILE_PROCEDURAL: SkillProceduralVisualRecipe = SkillProceduralVisualRecipe {
    kind: SkillProceduralVisualKind::FireBoltProjectile,
    hit_interval: Some(BOLT_HIT_INTERVAL),
    leads_impact: true,
};

const FROST_DIVER_PROCEDURAL: SkillProceduralVisualRecipe = SkillProceduralVisualRecipe {
    kind: SkillProceduralVisualKind::FrostDiver,
    hit_interval: None,
    leads_impact: false,
};

const FROST_DIVER_PREVIEW_PROCEDURAL: SkillProceduralVisualRecipe = SkillProceduralVisualRecipe {
    kind: SkillProceduralVisualKind::FrostDiverPreview,
    hit_interval: None,
    leads_impact: false,
};

const FROST_DIVER_IMPACT_PROCEDURAL: SkillProceduralVisualRecipe = SkillProceduralVisualRecipe {
    kind: SkillProceduralVisualKind::FrostDiverImpact,
    hit_interval: None,
    leads_impact: false,
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
        COLD_BOLT_SKILL => Some(COLD_BOLT_IMPACT_SOUND),
        FIRE_BOLT_SKILL => Some(FIRE_BOLT_SOUND),
        FROST_DIVER_SKILL => Some(FROST_DIVER_SOUND),
        _ => None,
    }
}

pub fn skill_damage_procedural_visual(skill_id: SkillId) -> Option<SkillProceduralVisualRecipe> {
    match skill_id {
        COLD_BOLT_SKILL => Some(COLD_BOLT_PROCEDURAL),
        FIRE_BOLT_SKILL => Some(FIRE_BOLT_PROJECTILE_PROCEDURAL),
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
        EffectId::Icearrow => Some(ICE_ARROW_DIRECT_SOUND),
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

        // Both bolts are the same falling-projectile shape, so they share the
        // cadence and the leading behaviour.
        let cold_bolt = skill_damage_procedural_visual(COLD_BOLT_SKILL).unwrap();
        assert_eq!(cold_bolt.kind, SkillProceduralVisualKind::ColdBolt);
        assert_eq!(cold_bolt.hit_interval, Some(BOLT_HIT_INTERVAL));
        assert!(cold_bolt.leads_impact);

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
    fn fire_bolt_projectile_leads_its_own_impact() {
        let projectile = skill_damage_procedural_visual(FIRE_BOLT_SKILL).unwrap();

        assert_eq!(projectile.kind, SkillProceduralVisualKind::FireBoltProjectile);
        assert!(projectile.leads_impact);
        // Projectile and impact must fan out on the same cadence, otherwise
        // the pairing drifts apart across a multi-hit volley.
        assert_eq!(projectile.hit_interval, Some(BOLT_HIT_INTERVAL));
        assert_eq!(
            skill_damage_visual(FIRE_BOLT_SKILL).unwrap().hit_interval,
            Some(BOLT_HIT_INTERVAL)
        );
        assert_eq!(
            skill_damage_sound(FIRE_BOLT_SKILL).unwrap().hit_interval,
            Some(BOLT_HIT_INTERVAL)
        );
    }

    #[test]
    fn variant_selection_covers_every_entry_and_never_indexes_out_of_range() {
        const VARIANTS: &[&str] = &["a", "b", "c"];

        // A roll of exactly 1.0 must not run past the end.
        assert_eq!(pick_variant("fallback", VARIANTS, 1.0), "c");
        assert_eq!(pick_variant("fallback", VARIANTS, 0.0), "a");
        assert_eq!(pick_variant("fallback", VARIANTS, 0.5), "b");
        // Out-of-contract rolls are clamped rather than panicking.
        assert_eq!(pick_variant("fallback", VARIANTS, -5.0), "a");
        assert_eq!(pick_variant("fallback", VARIANTS, 99.0), "c");

        // No variants means the single path is authoritative.
        assert_eq!(pick_variant("fallback", &[], 0.7), "fallback");

        // Every entry must be reachable, or a variant would be dead weight.
        let mut seen = [false; 3];
        for step in 0..300 {
            let picked = pick_variant("fallback", VARIANTS, step as f32 / 300.0);
            seen[VARIANTS.iter().position(|v| *v == picked).unwrap()] = true;
        }
        assert!(seen.iter().all(|hit| *hit));
    }

    #[test]
    fn varied_impacts_are_declared_and_preloaded() {
        // Fire Bolt and Fire Wall share EF_FIREHIT, so both vary per hit.
        let fire_bolt = skill_damage_visual(FIRE_BOLT_SKILL).unwrap();
        let fire_wall = skill_damage_visual(FIRE_WALL_SKILL).unwrap();
        assert_eq!(fire_bolt.effect_path_variants, FIRE_HIT_VARIANTS);
        assert_eq!(fire_wall.effect_path_variants, FIRE_HIT_VARIANTS);
        assert_eq!(FIRE_HIT_VARIANTS.len(), 3);

        // The declared default must itself be one of the variants, otherwise
        // an empty-variant fallback would render something inconsistent.
        assert!(FIRE_HIT_VARIANTS.contains(&fire_bolt.effect_path));

        // Cold Bolt's hit sound varies; its launch sounds live on the
        // projectile, not on the hit.
        let cold_bolt_sound = skill_damage_sound(COLD_BOLT_SKILL).unwrap();
        assert_eq!(cold_bolt_sound.sound_path_variants.len(), 2);
        assert!(cold_bolt_sound.sound_path_variants.contains(&cold_bolt_sound.sound_path));

        // Everything a variant list can select must be preloaded, or the
        // first use of a rarely-picked variant would stall.
        for path in FIRE_HIT_VARIANTS {
            assert!(path.ends_with(".str"), "impact variants are animations, not sounds");
        }
        for path in cold_bolt_sound.sound_path_variants {
            assert!(SKILL_SOUND_PATHS.contains(path), "{path} must be preloaded");
        }
        for path in ICE_ARROW_LAUNCH_SOUND_PATHS {
            assert!(SKILL_SOUND_PATHS.contains(&path), "{path} must be preloaded");
        }
    }

    #[test]
    fn projectile_flight_time_is_clamped_to_a_visible_range() {
        // The server's attack motion is ASPD dependent and unbounded as a
        // flight duration.
        assert_eq!(skill_projectile_flight_time(0), PROJECTILE_FLIGHT_MINIMUM);
        assert_eq!(skill_projectile_flight_time(-5000), PROJECTILE_FLIGHT_MINIMUM);
        assert_eq!(skill_projectile_flight_time(i32::MIN), PROJECTILE_FLIGHT_MINIMUM);
        assert_eq!(skill_projectile_flight_time(10_000), PROJECTILE_FLIGHT_MAXIMUM);
        assert_eq!(skill_projectile_flight_time(i32::MAX), PROJECTILE_FLIGHT_MAXIMUM);
        assert_eq!(skill_projectile_flight_time(420), 0.42);
        assert!(PROJECTILE_FLIGHT_MINIMUM > 0.0 && PROJECTILE_FLIGHT_MINIMUM < PROJECTILE_FLIGHT_MAXIMUM);
    }

    #[test]
    fn only_projectile_skills_delay_their_hit_feedback() {
        // Both bolts lead, so their hits wait for the projectile to land.
        for skill_id in [FIRE_BOLT_SKILL, COLD_BOLT_SKILL] {
            let recipe = skill_damage_procedural_visual(skill_id);
            assert_eq!(skill_impact_lead_time(recipe, 420), 0.42);
        }

        // Frost Diver travels from the caster rather than falling, and is not
        // modelled as a leading projectile, so it keeps a zero lead.
        assert_eq!(
            skill_impact_lead_time(skill_damage_procedural_visual(FROST_DIVER_SKILL), 420),
            0.0
        );
        assert_eq!(skill_impact_lead_time(None, 420), 0.0);
        assert_eq!(
            skill_impact_lead_time(skill_damage_procedural_visual(SkillId(u16::MAX)), 420),
            0.0
        );
    }

    #[test]
    fn every_fire_bolt_hit_resolves_after_its_projectile_lands() {
        // The property the split exists for: for every hit of the volley the
        // impact must never precede the projectile that produced it.
        let recipe = skill_damage_procedural_visual(FIRE_BOLT_SKILL).unwrap();
        let initial_delay = 0.3;

        for source_motion in [0, 120, 420, 900, i32::MAX] {
            let flight_time = skill_impact_lead_time(Some(recipe), source_motion);
            let impact_delay = initial_delay + flight_time;
            let launches = std::iter::once(0.0).chain(skill_effect_repeat_delays(recipe.hit_interval, 10));
            let impacts = std::iter::once(0.0).chain(skill_effect_repeat_delays(recipe.hit_interval, 10));

            for (launch_offset, impact_offset) in launches.zip(impacts) {
                let launch_at = initial_delay + launch_offset;
                let impact_at = impact_delay + impact_offset;
                let lands_at = launch_at + flight_time;

                assert!(impact_at > launch_at, "impact must not precede its projectile launch");
                assert!(
                    (impact_at - lands_at).abs() < 1.0e-6,
                    "impact must coincide with the projectile landing"
                );
            }
        }
    }

    #[test]
    fn fire_bolt_launch_sounds_are_indexable_and_present_in_the_preload_list() {
        assert_eq!(FIRE_ARROW_LAUNCH_SOUND_PATHS.len(), 3);
        for path in FIRE_ARROW_LAUNCH_SOUND_PATHS {
            assert!(SKILL_SOUND_PATHS.contains(&path), "{path} must be preloaded");
        }
        // Guards the index derivation used at the spawn site.
        for raw in [0.0_f32, 0.5, 0.999_999, 1.0] {
            let index = ((raw * FIRE_ARROW_LAUNCH_SOUND_PATHS.len() as f32) as usize).min(FIRE_ARROW_LAUNCH_SOUND_PATHS.len() - 1);
            assert!(index < FIRE_ARROW_LAUNCH_SOUND_PATHS.len());
        }
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
