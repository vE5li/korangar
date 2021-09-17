#[derive(Copy, Clone, Debug)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
}

impl Version {

    pub fn new(major: u8, minor: u8) -> Self {
        return Self { major, minor };
    }

    pub fn equals_or_above(&self, major: u8, minor: u8) -> bool {
        return self.major >= major && self.minor >= minor;
    }
}
