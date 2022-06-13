use cgmath::Vector2;

#[derive(Copy, Clone, Debug)]
pub enum UserEvent {
    Exit,
    CameraZoom(f32),
    CameraRotate(f32),
    ToggleFrameLimit,
    OpenMenuWindow,
    OpenGraphicsSettingsWindow,
    OpenAudioSettingsWindow,
    ReloadTheme,
    SaveTheme,
    #[cfg(feature = "debug")]
    OpenRenderSettingsWindow,
    #[cfg(feature = "debug")]
    OpenMapDataWindow,
    #[cfg(feature = "debug")]
    OpenThemeViewerWindow,
    #[cfg(feature = "debug")]
    OpenProfilerWindow,
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
    ToggleShowFramesPerSecond,
    #[cfg(feature = "debug")]
    ToggleShowMap,
    #[cfg(feature = "debug")]
    ToggleShowObjects,
    #[cfg(feature = "debug")]
    ToggleShowEntities,
    #[cfg(feature = "debug")]
    ToggleShowAmbientLight,
    #[cfg(feature = "debug")]
    ToggleShowDirectionalLight,
    #[cfg(feature = "debug")]
    ToggleShowPointLights,
    #[cfg(feature = "debug")]
    ToggleShowParticleLights,
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
