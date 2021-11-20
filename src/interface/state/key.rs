#[derive(Copy, Clone, Debug)]
pub enum StateKey {
    ShowFramesPerSecond,
    ShowMap,
    ShowObjects,
    ShowEntities,
    ShowAmbientLight,
    ShowDirectionalLight,
    ShowPointLights,
    ShowParticleLights,
    #[cfg(feature = "debug")]
    UseDebugCamera,
    #[cfg(feature = "debug")]
    ShowObjectMarkers,
    #[cfg(feature = "debug")]
    ShowLightMarkers,
    #[cfg(feature = "debug")]
    ShowSoundMarkers,
    #[cfg(feature = "debug")]
    ShowEffectMarkers,
    #[cfg(feature = "debug")]
    ShowParticleMarkers,
    #[cfg(feature = "debug")]
    ShowMapTiles,
    #[cfg(feature = "debug")]
    ShowPathing,
    #[cfg(feature = "debug")]
    ShowDiffuseBuffer,
    #[cfg(feature = "debug")]
    ShowNormalBuffer,
    #[cfg(feature = "debug")]
    ShowDepthBuffer,
    PlayerMaximumHealthPoints,
    PlayerCurrentHealthPoints,
    PlayerMaximumSpellPoints,
    PlayerCurrentSpellPoints,
    PlayerMaximumActivityPoints,
    PlayerCurrentActivityPoints,
}
