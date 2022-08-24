#[derive(Copy, Clone, Debug)]
pub enum ResourceType {
    Object,
    LightSource,
    SoundSource,
    EffectSource,
}

impl ResourceType {

    pub fn from(index: i32) -> Self {
        match index {
            1 => ResourceType::Object,
            2 => ResourceType::LightSource,
            3 => ResourceType::SoundSource,
            4 => ResourceType::EffectSource,
            invalid => panic!("invalid object type {}", invalid),
        }
    }
}
