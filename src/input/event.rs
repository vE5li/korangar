#[cfg(feature = "debug")]
use cgmath::Vector2;

pub enum InputEvent {
    CameraZoom(f32),
    CameraRotate(f32),
    ToggleFramesPerSecond,
    #[cfg(feature = "debug")]
    ToggleDebugCamera,
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
    CameraMoveDown,
}
