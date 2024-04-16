use ragnarok_networking::EntityId;

#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

#[cfg(feature = "debug")]
enum MarkerEncoding {
    Object = 10,
    LightSource = 11,
    SoundSource = 12,
    EffectSource = 13,
    Entity = 14,
}

/// Encoding a `PickerTarget` as `u32` has the following format:
///
/// If the most significant bit is set (`(1 << 31)`), it encodes a position. In
/// that case, the next 15 bits are used to store the X position, and the
/// remaining 16 bits contain the Y position. That gives us an effective maximum
/// value of 16384 for the X channel, which for Ragnarok Online is plenty.
///
/// If the `debug` feature is enabled, the most significant byte can contain a
/// value matching that of a `MarkerEncoding` (private enum). If that is the
/// case, the remaining 3 bytes are to be interpreted as the value of the
/// marker.
///
/// If the most significant bit is not set and the first byte does not match any
/// `MarkerEncoding`, an entity is encoded. This way we can use the full 32
/// bits to store the 32 bit [`EntityId`].
#[derive(Debug, PartialEq, Eq)]
pub enum PickerTarget {
    Tile {
        x: u16,
        y: u16,
    },
    Entity(EntityId),
    #[cfg(feature = "debug")]
    Marker(MarkerIdentifier),
}

impl From<u32> for PickerTarget {
    fn from(data: u32) -> Self {
        if data >> 31 == 1 {
            let x = ((data >> 16) as u16) ^ (1 << 15);
            let y = data as u16;
            return Self::Tile { x, y };
        }

        #[cfg(feature = "debug")]
        if data >> 24 == MarkerEncoding::Object as u32 {
            return Self::Marker(MarkerIdentifier::Object(data as usize & 0xFFF));
        }

        #[cfg(feature = "debug")]
        if data >> 24 == MarkerEncoding::LightSource as u32 {
            return Self::Marker(MarkerIdentifier::LightSource(data as usize & 0xFFF));
        }

        #[cfg(feature = "debug")]
        if data >> 24 == MarkerEncoding::SoundSource as u32 {
            return Self::Marker(MarkerIdentifier::SoundSource(data as usize & 0xFFF));
        }

        #[cfg(feature = "debug")]
        if data >> 24 == MarkerEncoding::EffectSource as u32 {
            return Self::Marker(MarkerIdentifier::EffectSource(data as usize & 0xFFF));
        }

        #[cfg(feature = "debug")]
        if data >> 24 == MarkerEncoding::Entity as u32 {
            return Self::Marker(MarkerIdentifier::Entity(data as usize & 0xFFF));
        }

        Self::Entity(EntityId(data))
    }
}

impl From<PickerTarget> for u32 {
    fn from(picker_target: PickerTarget) -> Self {
        match picker_target {
            PickerTarget::Tile { x, y } => {
                let mut encoded = ((x as u32) << 16) | y as u32;
                encoded |= 1 << 31;
                encoded
            }
            PickerTarget::Entity(EntityId(entity_id)) => entity_id,
            #[cfg(feature = "debug")]
            PickerTarget::Marker(marker_identifier) => match marker_identifier {
                MarkerIdentifier::Object(index) => ((MarkerEncoding::Object as u32) << 24) | (index as u32 & 0xFFF),
                MarkerIdentifier::LightSource(index) => ((MarkerEncoding::LightSource as u32) << 24) | (index as u32 & 0xFFF),
                MarkerIdentifier::SoundSource(index) => ((MarkerEncoding::SoundSource as u32) << 24) | (index as u32 & 0xFFF),
                MarkerIdentifier::EffectSource(index) => ((MarkerEncoding::EffectSource as u32) << 24) | (index as u32 & 0xFFF),
                MarkerIdentifier::Entity(index) => ((MarkerEncoding::Entity as u32) << 24) | (index as u32 & 0xFFF),
                _ => panic!(),
            },
        }
    }
}

#[cfg(test)]
#[allow(clippy::unusual_byte_groupings)]
mod test {
    use ragnarok_networking::EntityId;

    use crate::graphics::PickerTarget;
    #[cfg(feature = "debug")]
    use crate::world::MarkerIdentifier;

    // Position
    const X: u16 = 7;
    const Y: u16 = 3;
    const ENCODED_POSITION: u32 = 0b1_000000000000111_0000000000000011;

    // Markers
    #[cfg(feature = "debug")]
    const OBJECT_MARKER: MarkerIdentifier = MarkerIdentifier::Object(7);
    #[cfg(feature = "debug")]
    const LIGHT_SOURCE_MARKER: MarkerIdentifier = MarkerIdentifier::LightSource(7);
    #[cfg(feature = "debug")]
    const SOUND_SOURCE_MARKER: MarkerIdentifier = MarkerIdentifier::SoundSource(7);
    #[cfg(feature = "debug")]
    const EFFECT_SOURCE_MARKER: MarkerIdentifier = MarkerIdentifier::EffectSource(7);
    #[cfg(feature = "debug")]
    const ENTITY_MARKER: MarkerIdentifier = MarkerIdentifier::Entity(7);
    #[cfg(feature = "debug")]
    const ENCODED_OBJECT_MARKER: u32 = 0b00001010_000000000000000000000111;
    #[cfg(feature = "debug")]
    const ENCODED_LIGHT_SOURCE_MARKER: u32 = 0b00001011_000000000000000000000111;
    #[cfg(feature = "debug")]
    const ENCODED_SOUND_SOURCE_MARKER: u32 = 0b00001100_000000000000000000000111;
    #[cfg(feature = "debug")]
    const ENCODED_EFFECT_SOURCE_MARKER: u32 = 0b00001101_000000000000000000000111;
    #[cfg(feature = "debug")]
    const ENCODED_ENTITY_MARKER: u32 = 0b00001110_000000000000000000000111;

    // Entity
    const ENTITY_ID: EntityId = EntityId(7);
    const ENCODED_ENTITY_ID: u32 = 0b00000000000000000000000000000111;

    #[test]
    fn encode_tile() {
        assert_eq!(u32::from(PickerTarget::Tile { x: X, y: Y }), ENCODED_POSITION);
    }

    #[test]
    fn decode_tile() {
        assert_eq!(PickerTarget::from(ENCODED_POSITION), PickerTarget::Tile { x: X, y: Y });
    }

    #[test]
    #[cfg(feature = "debug")]
    fn encode_object_marker() {
        assert_eq!(u32::from(PickerTarget::Marker(OBJECT_MARKER)), ENCODED_OBJECT_MARKER);
    }

    #[test]
    #[cfg(feature = "debug")]
    fn decode_object_marker() {
        assert_eq!(PickerTarget::from(ENCODED_OBJECT_MARKER), PickerTarget::Marker(OBJECT_MARKER));
    }

    #[test]
    #[cfg(feature = "debug")]
    fn encode_light_source_marker() {
        assert_eq!(
            u32::from(PickerTarget::Marker(LIGHT_SOURCE_MARKER)),
            ENCODED_LIGHT_SOURCE_MARKER
        );
    }

    #[test]
    #[cfg(feature = "debug")]
    fn decode_light_source_marker() {
        assert_eq!(
            PickerTarget::from(ENCODED_LIGHT_SOURCE_MARKER),
            PickerTarget::Marker(LIGHT_SOURCE_MARKER)
        );
    }

    #[test]
    #[cfg(feature = "debug")]
    fn encode_sound_source_marker() {
        assert_eq!(
            u32::from(PickerTarget::Marker(SOUND_SOURCE_MARKER)),
            ENCODED_SOUND_SOURCE_MARKER
        );
    }

    #[test]
    #[cfg(feature = "debug")]
    fn decode_sound_source_marker() {
        assert_eq!(
            PickerTarget::from(ENCODED_SOUND_SOURCE_MARKER),
            PickerTarget::Marker(SOUND_SOURCE_MARKER)
        );
    }

    #[test]
    #[cfg(feature = "debug")]
    fn encode_effect_source_marker() {
        assert_eq!(
            u32::from(PickerTarget::Marker(EFFECT_SOURCE_MARKER)),
            ENCODED_EFFECT_SOURCE_MARKER
        );
    }

    #[test]
    #[cfg(feature = "debug")]
    fn decode_effect_source_marker() {
        assert_eq!(
            PickerTarget::from(ENCODED_EFFECT_SOURCE_MARKER),
            PickerTarget::Marker(EFFECT_SOURCE_MARKER)
        );
    }

    #[test]
    #[cfg(feature = "debug")]
    fn encode_entity_marker() {
        assert_eq!(u32::from(PickerTarget::Marker(ENTITY_MARKER)), ENCODED_ENTITY_MARKER);
    }

    #[test]
    #[cfg(feature = "debug")]
    fn decode_entity_marker() {
        assert_eq!(PickerTarget::from(ENCODED_ENTITY_MARKER), PickerTarget::Marker(ENTITY_MARKER));
    }

    #[test]
    fn encode_entity() {
        assert_eq!(u32::from(PickerTarget::Entity(ENTITY_ID)), ENCODED_ENTITY_ID);
    }

    #[test]
    fn decode_entity() {
        assert_eq!(PickerTarget::from(ENCODED_ENTITY_ID), PickerTarget::Entity(ENTITY_ID));
    }
}
