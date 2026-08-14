pub const TELEMETRY_ENABLED: bool = false;
pub const CLOUD_AUDIO_ENABLED: bool = false;
pub const GENERAL_HTTP_ENABLED: bool = false;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privacy_sensitive_network_features_are_off() {
        assert!(!TELEMETRY_ENABLED);
        assert!(!CLOUD_AUDIO_ENABLED);
        assert!(!GENERAL_HTTP_ENABLED);
    }
}
