use std::ops::{Index, IndexMut};

use crate::profiling::Measurement;

#[derive(Debug, Default, Clone)]
pub struct FrameMeasurement {
    buffer: Vec<Measurement>,
    next_index: usize,
}

impl FrameMeasurement {
    /// Clears the measurement without de-allocating.
    pub(super) fn clear(&mut self) {
        self.next_index = 0;
    }

    /// Creates a new measurement and returns its index.
    pub(super) fn new_measurement(&mut self, name: &'static str) -> usize {
        let index = self.next_index;
        self.next_index += 1;

        if index == self.buffer.len() {
            self.buffer.push(Measurement::default());
        }

        let measurement = &mut self.buffer[index];
        measurement.start_measurement(name);

        index
    }

    /// Returns `true` if the frame has measurements.
    pub fn has_measurements(&self) -> bool {
        !self.buffer.is_empty()
    }

    /// Returns the root measurement of the frame's measurement.
    pub fn root_measurement(&self) -> &Measurement {
        &self.buffer[0]
    }

    /// Returns an iterator over all measurements.
    pub fn measurements(&self) -> impl Iterator<Item = &Measurement> {
        self.buffer.iter()
    }
}

impl Index<usize> for FrameMeasurement {
    type Output = Measurement;

    fn index(&self, index: usize) -> &Self::Output {
        &self.buffer[index]
    }
}

impl IndexMut<usize> for FrameMeasurement {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.buffer[index]
    }
}

#[cfg(test)]
mod test {
    use crate::profiling::FrameMeasurement;

    #[test]
    fn basic_api() {
        let mut measurement = FrameMeasurement::default();

        assert_eq!(measurement.new_measurement("0"), 0);
        assert_eq!(measurement.new_measurement("1"), 1);
        assert_eq!(measurement.new_measurement("2"), 2);
        assert_eq!(measurement.new_measurement("3"), 3);

        assert_eq!(measurement.root_measurement().name, "0");
        assert_eq!(measurement[0].name, "0");
        assert_eq!(measurement[1].name, "1");
        assert_eq!(measurement[2].name, "2");
        assert_eq!(measurement[3].name, "3");

        measurement.clear();

        assert_eq!(measurement.new_measurement("9"), 0);
        assert_eq!(measurement.new_measurement("8"), 1);
        assert_eq!(measurement.new_measurement("7"), 2);
        assert_eq!(measurement.new_measurement("6"), 3);

        assert_eq!(measurement.root_measurement().name, "9");
        assert_eq!(measurement[0].name, "9");
        assert_eq!(measurement[1].name, "8");
        assert_eq!(measurement[2].name, "7");
        assert_eq!(measurement[3].name, "6");
    }
}
