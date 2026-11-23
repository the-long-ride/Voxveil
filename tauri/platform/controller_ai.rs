use super::*;

impl ProcessingController {
    pub fn configure_relay_ai(
        &self,
        options: voxveil_windows_audio::RelayAiOptions,
    ) -> Result<BackendSnapshot, String> {
        #[cfg(target_os = "windows")]
        {
            self.backend
                .lock()
                .map_err(|_| "Windows audio backend lock is poisoned")?
                .configure_ai(options)
                .map(from_windows_probe)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = options;
            Ok(self.snapshot())
        }
    }

    pub fn system_ai_status(&self) -> voxveil_types::SystemAiStatus {
        #[cfg(target_os = "windows")]
        {
            self.backend
                .lock()
                .map(|backend| backend.system_ai_status())
                .unwrap_or(voxveil_types::SystemAiStatus::Failed)
        }
        #[cfg(not(target_os = "windows"))]
        {
            voxveil_types::SystemAiStatus::Off
        }
    }
}
