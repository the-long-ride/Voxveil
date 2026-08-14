use super::PlatformCapabilities;

pub const fn standard_capabilities() -> PlatformCapabilities {
    PlatformCapabilities::MOBILE_STANDARD_ANDROID
}

pub const fn pro_system_capabilities() -> PlatformCapabilities {
    PlatformCapabilities::PRO_SYSTEM
}
