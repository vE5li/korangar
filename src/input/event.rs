use cgmath::Vector2;

#[derive(Copy, Clone, Debug)]
pub enum UserEvent {
    CameraZoom(f32),
    CameraRotate(f32),
    ToggleShowFramesPerSecond,
    ToggleShowMap,
    ToggleShowObjects,
    ToggleShowEntities,
    ToggleShowAmbientLight,
    ToggleShowDirectionalLight,
    ToggleShowPointLights,
    ToggleShowParticleLights,
    MoveInterface(usize, Vector2<f32>),
    #[cfg(feature = "debug")]
    ToggleUseDebugCamera,
    #[cfg(feature = "debug")]
    CameraLookAround(Vector2<f32>),
    #[cfg(feature = "debug")]
    CameraMoveForward,
    #[cfg(feature = "debug")]
    CameraMoveBackward,
    #[cfg(feature = "debug")]
    CameraMoveLeft,
    #[cfg(feature = "debug")]
    CameraMoveRight,
    #[cfg(feature = "debug")]
    CameraMoveUp,
    #[cfg(feature = "debug")]
    CameraAccelerate,
    #[cfg(feature = "debug")]
    CameraDecelerate,
    #[cfg(feature = "debug")]
    ToggleShowObjectMarkers,
    #[cfg(feature = "debug")]
    ToggleShowLightMarkers,
    #[cfg(feature = "debug")]
    ToggleShowSoundMarkers,
    #[cfg(feature = "debug")]
    ToggleShowEffectMarkers,
    #[cfg(feature = "debug")]
    ToggleShowParticleMarkers,
    #[cfg(feature = "debug")]
    ToggleShowMapTiles,
    #[cfg(feature = "debug")]
    ToggleShowPathing,
    #[cfg(feature = "debug")]
    ToggleShowDiffuseBuffer,
    #[cfg(feature = "debug")]
    ToggleShowNormalBuffer,
    #[cfg(feature = "debug")]
    ToggleShowDepthBuffer,
}
