mod controller;

pub use controller::{BackendSnapshot, ProcessingController};

pub mod android;
pub mod ios;
pub mod linux;
pub mod macos;
pub mod windows;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlatformCapabilities {
    pub all_output: bool,
    pub per_app: bool,
    pub virtual_output: bool,
    pub privileged_required: bool,
}

impl PlatformCapabilities {
    pub const DESKTOP_STANDARD: Self = Self {
        all_output: true,
        per_app: true,
        virtual_output: true,
        privileged_required: false,
    };

    pub const MOBILE_STANDARD_ANDROID: Self = Self {
        all_output: false,
        per_app: true,
        virtual_output: false,
        privileged_required: false,
    };

    pub const MOBILE_STANDARD_IOS: Self = Self {
        all_output: false,
        per_app: false,
        virtual_output: false,
        privileged_required: false,
    };

    pub const PRO_SYSTEM: Self = Self {
        all_output: true,
        per_app: true,
        virtual_output: true,
        privileged_required: true,
    };
}

use voxveil_types::ProcessingBackendStatus;

#[cfg(target_os = "windows")]
pub const fn processing_backend_status() -> ProcessingBackendStatus {
    windows::processing_backend_status()
}

#[cfg(target_os = "linux")]
pub const fn processing_backend_status() -> ProcessingBackendStatus {
    linux::processing_backend_status()
}

#[cfg(target_os = "macos")]
pub const fn processing_backend_status() -> ProcessingBackendStatus {
    macos::processing_backend_status()
}

#[cfg(target_os = "android")]
pub const fn processing_backend_status() -> ProcessingBackendStatus {
    android::processing_backend_status()
}

#[cfg(target_os = "ios")]
pub const fn processing_backend_status() -> ProcessingBackendStatus {
    ios::processing_backend_status()
}

#[cfg(not(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "macos",
    target_os = "android",
    target_os = "ios",
)))]
pub const fn processing_backend_status() -> ProcessingBackendStatus {
    ProcessingBackendStatus::Unsupported
}
