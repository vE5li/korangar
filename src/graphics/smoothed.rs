pub struct SmoothedValue {
    current: f32,
    desired: f32,
    threshhold: f32,
    speed: f32,
}

impl SmoothedValue {

    pub fn new(value: f32, threshhold: f32, speed: f32) -> Self {

        let current = value;
        let desired = value;
        Self {
            current,
            desired,
            threshhold,
            speed,
        }
    }

    pub fn update(&mut self, delta_time: f64) {
        if self.desired >= self.current + self.threshhold {

            let new_current = self.current + ((self.desired - self.current) * self.speed * delta_time as f32);
            self.current = self.desired.min(new_current);
        } else if self.desired <= self.current - self.threshhold {

            let new_current = self.current - ((self.current - self.desired) * self.speed * delta_time as f32);
            self.current = self.desired.max(new_current);
        }
    }

    pub fn move_desired(&mut self, offset: f32) {
        self.desired += offset;
    }

    pub fn move_desired_clamp(&mut self, offset: f32, minimum: f32, maximum: f32) {
        self.desired = (self.desired + offset).clamp(minimum, maximum);
    }

    pub fn get_current(&self) -> f32 {
        self.current
    }
}
