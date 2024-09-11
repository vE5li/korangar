use std::sync::atomic::{AtomicU64, Ordering};

use wgpu::Features;

static SUPPORTED_FEATURES: AtomicU64 = AtomicU64::new(0);

pub fn set_supported_features(supported_features: Features) {
    SUPPORTED_FEATURES.store(supported_features.bits(), Ordering::Relaxed)
}

pub fn features_supported(features: Features) -> bool {
    let supported = SUPPORTED_FEATURES.load(Ordering::Relaxed);
    // We do the comparison manually, since we don't need to check if the u64 value
    // only contains known bit.
    (supported & features.bits()) == features.bits()
}
