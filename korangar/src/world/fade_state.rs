use ragnarok_packets::ClientTick;

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
pub enum FadeDirection {
    In,
    Out,
}

#[derive(Copy, Clone, Debug)]
pub enum FadeState {
    Opaque,
    Fading {
        direction: FadeDirection,
        start_time: ClientTick,
        duration_ms: u32,
    },
}

impl FadeState {
    pub fn new(duration_ms: u32, client_tick: ClientTick) -> Self {
        Self::Fading {
            direction: FadeDirection::In,
            start_time: client_tick,
            duration_ms,
        }
    }

    pub fn calculate_alpha(&self, client_tick: ClientTick) -> f32 {
        match self {
            FadeState::Opaque => 1.0,
            FadeState::Fading {
                direction,
                start_time,
                duration_ms,
            } => match *direction {
                FadeDirection::In => {
                    if *duration_ms == 0 {
                        return 1.0;
                    }
                    let elapsed = client_tick.0.wrapping_sub(start_time.0);
                    (elapsed as f32 / *duration_ms as f32).min(1.0)
                }
                FadeDirection::Out => {
                    if *duration_ms == 0 {
                        return 0.0;
                    }
                    let elapsed = client_tick.0.wrapping_sub(start_time.0);
                    1.0 - (elapsed as f32 / *duration_ms as f32).min(1.0)
                }
            },
        }
    }

    pub fn is_fading(&self) -> bool {
        matches!(self, FadeState::Fading { .. })
    }

    fn is_complete(start_time: ClientTick, duration_ms: u32, client_tick: ClientTick) -> bool {
        let elapsed = client_tick.0.wrapping_sub(start_time.0);
        duration_ms == 0 || elapsed >= duration_ms
    }

    pub fn is_done_fading_in(&self, client_tick: ClientTick) -> bool {
        match *self {
            FadeState::Opaque => true,
            FadeState::Fading {
                direction,
                start_time,
                duration_ms,
            } => direction == FadeDirection::In && Self::is_complete(start_time, duration_ms, client_tick),
        }
    }

    pub fn is_done_fading_out(&self, client_tick: ClientTick) -> bool {
        match *self {
            FadeState::Opaque => false,
            FadeState::Fading {
                direction,
                start_time,
                duration_ms,
            } => direction == FadeDirection::Out && Self::is_complete(start_time, duration_ms, client_tick),
        }
    }

    /// Creates a new fade state starting from a specific alpha value.
    /// This allows smooth transitions between fade states by preserving the
    /// current alpha.
    pub fn from_alpha(alpha: f32, direction: FadeDirection, client_tick: ClientTick, duration_ms: u32) -> Self {
        let alpha = alpha.clamp(0.0, 1.0);
        let elapsed = match direction {
            FadeDirection::In => (alpha * duration_ms as f32) as u32,
            FadeDirection::Out => ((1.0 - alpha) * duration_ms as f32) as u32,
        };
        let start_time = ClientTick(client_tick.0.wrapping_sub(elapsed));
        FadeState::Fading {
            direction,
            start_time,
            duration_ms,
        }
    }
}
