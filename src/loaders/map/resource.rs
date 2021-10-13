#[derive(Copy, Clone, Debug)]
pub enum ResourceType {
    Object,
    LightSource,
    SoundSource,
    EffectSource
}

impl ResourceType {

    pub fn from(index: i32) -> Self {
        match index {
            1 => return ResourceType::Object,
            2 => return ResourceType::LightSource,
            3 => return ResourceType::SoundSource,
            4 => return ResourceType::EffectSource,
            invalid => panic!("invalid object type {}", invalid),
        }
    }
}
