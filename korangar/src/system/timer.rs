use std::time::Instant;

use ragnarok_packets::ClientTick;

pub struct GameTimer {
    global_timer: Instant,
    previous_elapsed: f64,
    accumulate_second: f64,
    frame_counter: usize,
    frames_per_second: usize,
    animation_timer_ms: f32,
    last_packet_receive_time: Instant,
    first_tick_received: bool,
    base_client_tick: f64,
    frequency: f64,
}

impl GameTimer {
    pub fn new() -> Self {
        Self {
            global_timer: Instant::now(),
            previous_elapsed: Default::default(),
            accumulate_second: Default::default(),
            frame_counter: Default::default(),
            frames_per_second: Default::default(),
            animation_timer_ms: Default::default(),
            last_packet_receive_time: Instant::now(),
            first_tick_received: false,
            base_client_tick: 0.0,
            frequency: 0.0,
        }
    }

    /// The networking system sends a request for the newest global tick rate
    /// every 10 seconds and returns an update event that contains the estimated
    /// server tick rate. We only need to gently adjust our frequency, so that
    /// no discontinuation of the local client tick occur.
    pub fn set_client_tick(&mut self, client_tick: ClientTick, packet_receive_time: Instant) {
        if !self.first_tick_received {
            // We currently trust the first client tick too much. If, because of an
            // asymmetry of the round trip time, the client tick deviates too much from the
            // real server tick, then the client needs to catch-up the real value. Since we
            // limit the client tick change to +/- 5 milliseconds each 10 seconds, such a
            // catch-up could take quite some time. Such cases are rare though, so I'm not
            // yet sure how to best counter such cases.
            self.first_tick_received = true;
            self.base_client_tick = client_tick.0 as f64;
            self.last_packet_receive_time = packet_receive_time;
            return;
        }

        let local_tick = self.get_client_tick_at(packet_receive_time);
        let tick_difference = client_tick.0 as f64 - local_tick;

        // Calculate frequency needed to make up the difference over the next 10
        // seconds. We also clamp the difference, so that we are more resistant to
        // cases, where the round trip time was highly asymmetric.
        self.frequency = tick_difference.clamp(-5.0, 5.0) / 10000.0;

        self.base_client_tick = local_tick;
        self.last_packet_receive_time = packet_receive_time;
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn get_client_tick_at(&self, time: Instant) -> f64 {
        let elapsed = time.duration_since(self.last_packet_receive_time).as_secs_f64();
        self.base_client_tick + (elapsed * 1000.0) + (elapsed * self.frequency * 1000.0)
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn get_client_tick(&self) -> ClientTick {
        let tick = self.get_client_tick_at(Instant::now());
        ClientTick(tick.round() as u32)
    }

    pub fn get_animation_timer_ms(&self) -> f32 {
        self.animation_timer_ms
    }

    pub fn update(&mut self) -> f64 {
        let new_elapsed = self.global_timer.elapsed().as_secs_f64();
        let delta_time = new_elapsed - self.previous_elapsed;

        self.frame_counter += 1;
        self.accumulate_second += delta_time;
        self.animation_timer_ms += 1000.0 * delta_time as f32;
        self.previous_elapsed = new_elapsed;

        if self.accumulate_second > 1.0 {
            self.frames_per_second = self.frame_counter;
            self.accumulate_second -= 1.0;
            self.frame_counter = 0;
        }

        delta_time
    }

    #[cfg(feature = "debug")]
    pub fn last_frames_per_second(&self) -> usize {
        self.frames_per_second
    }
}

#[cfg(test)]
mod increment {
    use crate::system::GameTimer;

    #[test]
    fn update_increments_frame_counter() {
        let mut game_timer = GameTimer::new();
        game_timer.update();
        assert_eq!(game_timer.frame_counter, 1);
    }

    #[test]
    fn update_increments_timers() {
        let mut game_timer = GameTimer::new();

        let animation_timer_ms = game_timer.get_animation_timer_ms();

        std::thread::sleep(std::time::Duration::from_millis(10));
        game_timer.update();

        let updated_animation_timer_ms = game_timer.get_animation_timer_ms();

        assert!(updated_animation_timer_ms > animation_timer_ms);
    }
}
