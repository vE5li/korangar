pub mod version_20220406;
pub mod version_20250416;

/// All supported packet versions.
#[derive(Debug, Clone, Copy)]
pub enum SupportedPacketVersion {
    _20220406,
    _20250416,
}
