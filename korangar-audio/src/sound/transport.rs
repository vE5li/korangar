pub(crate) struct Transport {
    pub(crate) position: usize,
    /// The start and end frames of the sound that should be looped. The upper
    /// bound is *exclusive*.
    pub(crate) loop_region: Option<(usize, usize)>,
    pub(crate) playing: bool,
}

impl Transport {
    #[must_use]
    pub(crate) fn new(looping: bool, num_frames: usize) -> Self {
        Self {
            position: 0,
            loop_region: looping.then(|| (0, num_frames)),
            playing: true,
        }
    }

    pub(crate) fn increment_position(&mut self, num_frames: usize) {
        if !self.playing {
            return;
        }
        self.position += 1;
        if let Some((loop_start, loop_end)) = self.loop_region {
            while self.position >= loop_end {
                self.position -= loop_end - loop_start;
            }
        }
        if self.position >= num_frames {
            self.playing = false;
        }
    }

    pub(crate) fn seek_to(&mut self, mut position: usize, num_frames: usize) {
        if let Some((loop_start, loop_end)) = self.loop_region {
            if position > self.position {
                while position >= loop_end {
                    position -= loop_end - loop_start;
                }
            } else {
                while position < loop_start {
                    position += loop_end - loop_start;
                }
            }
        }
        self.position = position;
        if self.position >= num_frames {
            self.playing = false;
        }
    }
}
