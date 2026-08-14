use super::PlatformCapabilities;
use voxveil_types::ProcessingBackendStatus;

pub const fn capabilities() -> PlatformCapabilities {
    PlatformCapabilities::DESKTOP_STANDARD
}

pub const fn processing_backend_status() -> ProcessingBackendStatus {
    ProcessingBackendStatus::Unsupported
}
