pub enum StateKey {
    ShowFramesPerSecond,
    ShowMap,
    ShowObjects,
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
}
