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
