// A copy of "pacy" by https://github.com/BVE-Reborn/pacy/
//
// zlib License
//
// (C) 2020 Connor Fitzgerald
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not claim
//    that you wrote the original software. If you use this software in a
//    product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use spin_sleep::SpinSleeper;

pub trait ComparativeTimestamp: Copy {
    fn difference(base: Self, new: Self) -> Duration;
}

impl ComparativeTimestamp for Instant {
    fn difference(base: Self, new: Self) -> Duration {
        new - base
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FrameStage<T: ComparativeTimestamp> {
    index: usize,
    base: T,
}

pub struct FramePacer {
    internals: Internals,
    sleeper: SpinSleeper,
}

impl FramePacer {
    pub fn new(reported_frequency: f64) -> Self {
        Self {
            internals: Internals::new(Monitor::new(reported_frequency)),
            sleeper: SpinSleeper::default(),
        }
    }

    pub fn create_frame_stage<T>(&mut self, base: T) -> FrameStage<T>
    where
        T: ComparativeTimestamp,
    {
        let index = self.internals.frame_stages.len();
        self.internals
            .frame_stages
            .push(FrameStageStats::new(self.internals.reference_instant.elapsed()));
        FrameStage { index, base }
    }

    pub fn set_monitor_frequency(&mut self, frequency: f64) {
        self.internals.monitor.reported_frequency = frequency;
    }

    pub fn begin_frame_stage<T>(&mut self, stage_id: FrameStage<T>, now: T)
    where
        T: ComparativeTimestamp,
    {
        self.internals.frame_stages[stage_id.index].begin(T::difference(stage_id.base, now));
    }

    pub fn end_frame_stage<T>(&mut self, stage_id: FrameStage<T>, now: T)
    where
        T: ComparativeTimestamp,
    {
        self.internals.frame_stages[stage_id.index].end(T::difference(stage_id.base, now));
    }

    pub fn wait_for_frame(&mut self) {
        let next_frame_pipeline_duration: Duration = self
            .internals
            .frame_stages
            .iter()
            .map(FrameStageStats::estimate_time_for_completion)
            .sum();

        let sleep_duration = self
            .internals
            .monitor
            .duration_until_next_hittable_timestamp(next_frame_pipeline_duration);

        self.internals.sleep_history.push_back(sleep_duration);
        self.sleeper.sleep(sleep_duration);
    }
}

struct Internals {
    reference_instant: Instant,
    frame_stages: Vec<FrameStageStats>,
    sleep_history: VecDeque<Duration>,
    monitor: Monitor,
}

impl Internals {
    fn new(monitor: Monitor) -> Self {
        Self {
            reference_instant: Instant::now(),
            frame_stages: Vec::new(),
            sleep_history: VecDeque::new(),
            monitor,
        }
    }
}

#[derive(Default)]
struct FrameStageStats {
    offset: Duration,
    start_time: Option<Duration>,
    end_time: Option<Duration>,
    duration_history: VecDeque<Duration>,
}

impl FrameStageStats {
    fn new(offset: Duration) -> Self {
        Self {
            offset,
            ..Default::default()
        }
    }

    fn begin(&mut self, value: Duration) {
        self.start_time = Some(self.offset + value);
    }

    fn end(&mut self, value: Duration) {
        self.duration_history.reserve(1);
        self.end_time = Some(self.offset + value);
        if let Some(start_time) = self.start_time {
            self.duration_history.push_back(self.end_time.unwrap() - start_time);
        }
    }

    fn estimate_time_for_completion(&self) -> Duration {
        *self.duration_history.iter().rev().take(10).max().unwrap_or(&Duration::from_secs(0))
    }
}

struct Monitor {
    reported_frequency: f64,
    last_reported_timestamp: Instant,
}

impl Monitor {
    fn new(reported_frequency: f64) -> Self {
        Self {
            reported_frequency,
            last_reported_timestamp: Instant::now(),
        }
    }

    fn duration_until_next_hittable_timestamp(&self, compute_time: Duration) -> Duration {
        let actual_vblank_secs = ((self.reported_frequency + 0.5).floor() - 0.5).recip();
        let actual_vblank_nanos = (1_000_000_000.0 * actual_vblank_secs) as u128;

        let now = Instant::now();

        let compute_finished = now + compute_time;
        let dur_since_timestamp = compute_finished - self.last_reported_timestamp;

        let nanos_into_frame = dur_since_timestamp.as_nanos() % actual_vblank_nanos;
        let nanos_remaining = actual_vblank_nanos - nanos_into_frame;

        Duration::from_nanos(nanos_remaining as u64)
    }
}
