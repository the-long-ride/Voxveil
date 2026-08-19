#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_windows_device_identity_case_insensitively() {
        assert_eq!(normalize_device_id("HDAUDIO\\FUNC_01"), "hdaudio\\func_01");
    }

    #[test]
    fn empty_endpoint_id_has_no_adapter_binding() {
        assert_eq!(resolve_adapter_device_id(""), Ok(None));
    }
}
