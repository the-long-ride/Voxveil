use super::PlatformCapabilities;
use voxveil_types::ProcessingBackendStatus;

pub const fn standard_capabilities() -> PlatformCapabilities {
    PlatformCapabilities::MOBILE_STANDARD_IOS
}

pub const fn pro_system_capabilities() -> PlatformCapabilities {
    PlatformCapabilities::PRO_SYSTEM
}

pub const fn processing_backend_status() -> ProcessingBackendStatus {
    ProcessingBackendStatus::Unsupported
}
