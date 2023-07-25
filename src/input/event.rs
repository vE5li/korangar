use cgmath::Vector2;

use crate::interface::ItemMove;
use crate::network::{CharacterId, EntityId};
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

#[derive(Clone, Debug)]
pub enum UserEvent {
    LogIn(String, String),
    LogOut,
    Exit,
    ToggleRemeberUsername,
    ToggleRemeberPassword,
    CameraZoom(f32),
    CameraRotate(f32),
    ToggleFrameLimit,
    ToggleShowInterface,
    OpenMenuWindow,
    OpenInventoryWindow,
    OpenEquipmentWindow,
    OpenGraphicsSettingsWindow,
    OpenAudioSettingsWindow,
    ReloadTheme,
    SaveTheme,
    SelectCharacter(usize),
    OpenCharacterCreationWindow(usize),
    CreateCharacter(usize, String),
    DeleteCharacter(CharacterId),
    RequestSwitchCharacterSlot(usize),
    CancelSwitchCharacterSlot,
    SwitchCharacterSlot(usize),
    RequestPlayerMove(Vector2<usize>),
    RequestPlayerInteract(EntityId),
    RequestWarpToMap(String, Vector2<usize>),
    SendMessage(String),
    NextDialog(EntityId),
    CloseDialog(EntityId),
    ChooseDialogOption(EntityId, i8),
    MoveItem(ItemMove),
    #[cfg(feature = "debug")]
    ToggleFrustumCulling,
    #[cfg(feature = "debug")]
    ToggleShowBoundingBoxes,
    #[cfg(feature = "debug")]
    OpenMarkerDetails(MarkerIdentifier),
    #[cfg(feature = "debug")]
    OpenRenderSettingsWindow,
    #[cfg(feature = "debug")]
    OpenMapDataWindow,
    #[cfg(feature = "debug")]
    OpenMapsWindow,
    #[cfg(feature = "debug")]
    OpenCommandsWindow,
    #[cfg(feature = "debug")]
    OpenTimeWindow,
    #[cfg(feature = "debug")]
    SetDawn,
    #[cfg(feature = "debug")]
    SetNoon,
    #[cfg(feature = "debug")]
    SetDusk,
    #[cfg(feature = "debug")]
    SetMidnight,
    #[cfg(feature = "debug")]
    OpenThemeViewerWindow,
    #[cfg(feature = "debug")]
    OpenProfilerWindow,
    #[cfg(feature = "debug_network")]
    OpenPacketWindow,
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
    ToggleShowWireframe,
    #[cfg(feature = "debug")]
    ToggleShowMap,
    #[cfg(feature = "debug")]
    ToggleShowObjects,
    #[cfg(feature = "debug")]
    ToggleShowEntities,
    #[cfg(feature = "debug")]
    ToggleShowWater,
    #[cfg(feature = "debug")]
    ToggleShowIndicators,
    #[cfg(feature = "debug")]
    ToggleShowAmbientLight,
    #[cfg(feature = "debug")]
    ToggleShowDirectionalLight,
    #[cfg(feature = "debug")]
    ToggleShowPointLights,
    #[cfg(feature = "debug")]
    ToggleShowParticleLights,
    #[cfg(feature = "debug")]
    ToggleShowDirectionalShadows,
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
    ToggleShowEntityMarkers,
    #[cfg(feature = "debug")]
    ToggleShowMapTiles,
    #[cfg(feature = "debug")]
    ToggleShowPathing,
    #[cfg(feature = "debug")]
    ToggleShowDiffuseBuffer,
    #[cfg(feature = "debug")]
    ToggleShowNormalBuffer,
    #[cfg(feature = "debug")]
    ToggleShowWaterBuffer,
    #[cfg(feature = "debug")]
    ToggleShowDepthBuffer,
    #[cfg(feature = "debug")]
    ToggleShowShadowBuffer,
    #[cfg(feature = "debug")]
    ToggleShowPickerBuffer,
    #[cfg(feature = "debug")]
    ToggleShowFontAtlas,
}
