#[cfg(feature = "debug")]
use korangar_container::SimpleKey;
use ragnarok_packets::EntityId;

#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

#[repr(u32)]
pub(super) enum PickerValueType {
    Nothing,
    Tile,
    Entity,
    #[cfg(feature = "debug")]
    ObjectMarker,
    #[cfg(feature = "debug")]
    LightSourceMarker,
    #[cfg(feature = "debug")]
    SoundSourceMarker,
    #[cfg(feature = "debug")]
    EffectSourceMarker,
    #[cfg(feature = "debug")]
    EntityMarker,
    #[cfg(feature = "debug")]
    ShadowMarker,
}

/// Encoding of a `PickerTarget` as `u64` has the following format:
///
/// The high 32 bits are the `PickerValueType`.
/// The low 32 bits are the data of the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerTarget {
    Nothing,
    Tile {
        x: u16,
        y: u16,
    },
    Entity(EntityId),
    #[cfg(feature = "debug")]
    Marker(MarkerIdentifier),
}

impl PickerTarget {
    pub(crate) const fn value_size() -> usize {
        size_of::<u64>()
    }
}

impl From<u64> for PickerTarget {
    fn from(data: u64) -> Self {
        if data >> 32 == PickerValueType::Tile as u64 {
            let x = (data >> 16) as u16;
            let y = data as u16;
            return Self::Tile { x, y };
        }

        if data >> 32 == PickerValueType::Entity as u64 {
            return Self::Entity(EntityId(data as u32));
        }

        #[cfg(feature = "debug")]
        if data >> 32 == PickerValueType::ObjectMarker as u64 {
            return Self::Marker(MarkerIdentifier::Object(data as u32));
        }

        #[cfg(feature = "debug")]
        if data >> 32 == PickerValueType::LightSourceMarker as u64 {
            return Self::Marker(MarkerIdentifier::LightSource(data as u32));
        }

        #[cfg(feature = "debug")]
        if data >> 32 == PickerValueType::SoundSourceMarker as u64 {
            return Self::Marker(MarkerIdentifier::SoundSource(data as u32));
        }

        #[cfg(feature = "debug")]
        if data >> 32 == PickerValueType::EffectSourceMarker as u64 {
            return Self::Marker(MarkerIdentifier::EffectSource(data as u32));
        }

        #[cfg(feature = "debug")]
        if data >> 32 == PickerValueType::EntityMarker as u64 {
            return Self::Marker(MarkerIdentifier::Entity(data as u32));
        }

        #[cfg(feature = "debug")]
        if data >> 32 == PickerValueType::ShadowMarker as u64 {
            return Self::Marker(MarkerIdentifier::Shadow(data as u32));
        }

        Self::Nothing
    }
}

impl From<PickerTarget> for u64 {
    fn from(picker_target: PickerTarget) -> Self {
        let (high, low) = <(u32, u32)>::from(picker_target);
        (u64::from(high) << 32) | u64::from(low)
    }
}

impl From<PickerTarget> for (u32, u32) {
    fn from(picker_target: PickerTarget) -> Self {
        match picker_target {
            PickerTarget::Nothing => (PickerValueType::Nothing as u32, 0),
            PickerTarget::Tile { x, y } => (PickerValueType::Tile as u32, ((x as u32) << 16) | y as u32),
            PickerTarget::Entity(EntityId(entity_id)) => (PickerValueType::Entity as u32, entity_id),
            #[cfg(feature = "debug")]
            PickerTarget::Marker(marker_identifier) => match marker_identifier {
                MarkerIdentifier::Object(index) => (PickerValueType::ObjectMarker as u32, index.key()),
                MarkerIdentifier::LightSource(index) => (PickerValueType::LightSourceMarker as u32, index),
                MarkerIdentifier::SoundSource(index) => (PickerValueType::SoundSourceMarker as u32, index),
                MarkerIdentifier::EffectSource(index) => (PickerValueType::EffectSourceMarker as u32, index),
                MarkerIdentifier::Entity(index) => (PickerValueType::EntityMarker as u32, index),
                MarkerIdentifier::Shadow(index) => (PickerValueType::ShadowMarker as u32, index),
                _ => panic!(),
            },
        }
    }
}

#[cfg(test)]
#[allow(clippy::unusual_byte_groupings)]
mod encoding {
    use ragnarok_packets::EntityId;

    use crate::graphics::PickerTarget;
    #[cfg(feature = "debug")]
    use crate::world::MarkerIdentifier;

    // Position
    const X: u16 = 7;
    const Y: u16 = 3;
    const ENCODED_POSITION: u64 = 0x00000001_00070003;

    // Markers
    #[cfg(feature = "debug")]
    const OBJECT_MARKER: MarkerIdentifier = MarkerIdentifier::Object(255);
    #[cfg(feature = "debug")]
    const LIGHT_SOURCE_MARKER: MarkerIdentifier = MarkerIdentifier::LightSource(254);
    #[cfg(feature = "debug")]
    const SOUND_SOURCE_MARKER: MarkerIdentifier = MarkerIdentifier::SoundSource(253);
    #[cfg(feature = "debug")]
    const EFFECT_SOURCE_MARKER: MarkerIdentifier = MarkerIdentifier::EffectSource(252);
    #[cfg(feature = "debug")]
    const ENTITY_MARKER: MarkerIdentifier = MarkerIdentifier::Entity(251);
    #[cfg(feature = "debug")]
    const SHADOW_MARKER: MarkerIdentifier = MarkerIdentifier::Shadow(250);
    #[cfg(feature = "debug")]
    const ENCODED_OBJECT_MARKER: u64 = 0x00000003_000000FF;
    #[cfg(feature = "debug")]
    const ENCODED_LIGHT_SOURCE_MARKER: u64 = 0x00000004_000000FE;
    #[cfg(feature = "debug")]
    const ENCODED_SOUND_SOURCE_MARKER: u64 = 0x00000005_000000FD;
    #[cfg(feature = "debug")]
    const ENCODED_EFFECT_SOURCE_MARKER: u64 = 0x00000006_000000FC;
    #[cfg(feature = "debug")]
    const ENCODED_ENTITY_MARKER: u64 = 0x00000007_000000FB;
    #[cfg(feature = "debug")]
    const ENCODED_SHADOW_MARKER: u64 = 0x00000008_000000FA;

    // Entity
    const ENTITY_ID: EntityId = EntityId(7);
    const ENCODED_ENTITY_ID: u64 = 0x00000002_00000007;

    #[test]
    fn from_u64() {
        let target = PickerTarget::Tile { x: X, y: Y };

        let (high, low) = <(u32, u32)>::from(target);
        let encoded = u64::from(target);

        assert_eq!(((high as u64) << 32) | low as u64, encoded);
    }

    #[test]
    fn encode_tile() {
        assert_eq!(u64::from(PickerTarget::Tile { x: X, y: Y }), ENCODED_POSITION);
    }

    #[test]
    fn decode_tile() {
        assert_eq!(PickerTarget::from(ENCODED_POSITION), PickerTarget::Tile { x: X, y: Y });
    }

    #[test]
    #[cfg(feature = "debug")]
    fn encode_object_marker() {
        assert_eq!(u64::from(PickerTarget::Marker(OBJECT_MARKER)), ENCODED_OBJECT_MARKER);
    }

    #[test]
    #[cfg(feature = "debug")]
    fn decode_object_marker() {
        assert_eq!(PickerTarget::from(ENCODED_OBJECT_MARKER), PickerTarget::Marker(OBJECT_MARKER),);
    }

    #[test]
    #[cfg(feature = "debug")]
    fn encode_light_source_marker() {
        assert_eq!(
            u64::from(PickerTarget::Marker(LIGHT_SOURCE_MARKER)),
            ENCODED_LIGHT_SOURCE_MARKER
        );
    }

    #[test]
    #[cfg(feature = "debug")]
    fn decode_light_source_marker() {
        assert_eq!(
            PickerTarget::from(ENCODED_LIGHT_SOURCE_MARKER),
            PickerTarget::Marker(LIGHT_SOURCE_MARKER),
        );
    }

    #[test]
    #[cfg(feature = "debug")]
    fn encode_sound_source_marker() {
        assert_eq!(
            u64::from(PickerTarget::Marker(SOUND_SOURCE_MARKER)),
            ENCODED_SOUND_SOURCE_MARKER
        );
    }

    #[test]
    #[cfg(feature = "debug")]
    fn decode_sound_source_marker() {
        assert_eq!(
            PickerTarget::from(ENCODED_SOUND_SOURCE_MARKER),
            PickerTarget::Marker(SOUND_SOURCE_MARKER),
        );
    }

    #[test]
    #[cfg(feature = "debug")]
    fn encode_effect_source_marker() {
        assert_eq!(
            u64::from(PickerTarget::Marker(EFFECT_SOURCE_MARKER)),
            ENCODED_EFFECT_SOURCE_MARKER
        );
    }

    #[test]
    #[cfg(feature = "debug")]
    fn decode_effect_source_marker() {
        assert_eq!(
            PickerTarget::from(ENCODED_EFFECT_SOURCE_MARKER),
            PickerTarget::Marker(EFFECT_SOURCE_MARKER),
        );
    }

    #[test]
    #[cfg(feature = "debug")]
    fn encode_entity_marker() {
        assert_eq!(u64::from(PickerTarget::Marker(ENTITY_MARKER)), ENCODED_ENTITY_MARKER);
    }

    #[test]
    #[cfg(feature = "debug")]
    fn decode_entity_marker() {
        assert_eq!(PickerTarget::from(ENCODED_ENTITY_MARKER), PickerTarget::Marker(ENTITY_MARKER),);
    }

    #[test]
    #[cfg(feature = "debug")]
    fn encode_shadow_marker() {
        assert_eq!(u64::from(PickerTarget::Marker(SHADOW_MARKER)), ENCODED_SHADOW_MARKER);
    }

    #[test]
    #[cfg(feature = "debug")]
    fn decode_shadow_marker() {
        assert_eq!(PickerTarget::from(ENCODED_SHADOW_MARKER), PickerTarget::Marker(SHADOW_MARKER),);
    }

    #[test]
    fn encode_entity() {
        assert_eq!(u64::from(PickerTarget::Entity(ENTITY_ID)), ENCODED_ENTITY_ID);
    }

    #[test]
    fn decode_entity() {
        assert_eq!(PickerTarget::from(ENCODED_ENTITY_ID), PickerTarget::Entity(ENTITY_ID));
    }
}
