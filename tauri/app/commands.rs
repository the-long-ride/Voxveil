use tauri::{AppHandle, State};
use voxveil_types::{
    AudioBypassReason, OutputMode, ProcessingBackendStatus, ProcessingEngineKind, ProcessingMode,
};

use super::{
    dto::{AppSourceDto, AppViewState},
    state::AppState,
};
use crate::platform::ProcessingController;

fn validate_percent(value: u8) -> Result<u8, String> {
    if value <= 100 {
        Ok(value)
    } else {
        Err("value must be between 0 and 100".into())
    }
}

fn validate_output_mode(mode: OutputMode, virtual_available: bool) -> Result<OutputMode, String> {
    if !virtual_available && matches!(mode, OutputMode::Virtual | OutputMode::Both) {
        Err("virtual output is unavailable".into())
    } else {
        Ok(mode)
    }
}

#[tauri::command]
pub fn get_app_state(
    state: State<'_, AppState>,
    controller: State<'_, ProcessingController>,
) -> Result<AppViewState, String> {
    let snapshot = controller.snapshot();
    let mut current = state.lock()?;
    current.apply_backend(&snapshot);
    Ok(current.clone())
}

fn validate_master_enable(status: ProcessingBackendStatus, enabled: bool) -> Result<(), String> {
    if enabled && status != ProcessingBackendStatus::Ready {
        Err("processing backend is unavailable".into())
    } else {
        Ok(())
    }
}

#[tauri::command]
pub fn set_master_enabled(
    state: State<'_, AppState>,
    controller: State<'_, ProcessingController>,
    enabled: bool,
) -> Result<(), String> {
    let vocal_level = state.lock()?.vocal_level;
    let snapshot = controller.set_enabled(enabled, vocal_level)?;
    validate_master_enable(snapshot.status, enabled)?;
    let mut current = state.lock()?;
    current.apply_backend(&snapshot);
    current.master_enabled = enabled;
    Ok(())
}

#[tauri::command]
pub fn set_processing_mode(
    state: State<'_, AppState>,
    controller: State<'_, ProcessingController>,
    mode: ProcessingMode,
) -> Result<(), String> {
    let snapshot = controller.snapshot();
    if mode == ProcessingMode::PerApp && !snapshot.per_app_available {
        return Err(
            "per-app processing is not available in the current Windows audio relay".into(),
        );
    }
    state.lock()?.processing_mode = mode;
    Ok(())
}

#[tauri::command]
pub fn set_engine(
    app: AppHandle,
    state: State<'_, AppState>,
    engine: ProcessingEngineKind,
) -> Result<(), String> {
    if engine == ProcessingEngineKind::Ai && !crate::models::ai_runtime_ready(&app)? {
        return Err(
            "AI engine is unavailable until a verified model and inference runtime are both ready"
                .to_string(),
        );
    }
    state.lock()?.engine = engine;
    Ok(())
}

#[tauri::command]
pub fn set_vocal_level(
    state: State<'_, AppState>,
    controller: State<'_, ProcessingController>,
    value: u8,
) -> Result<(), String> {
    let value = validate_percent(value)?;
    controller.set_vocal_level(value)?;
    state.lock()?.vocal_level = value;
    Ok(())
}

#[tauri::command]
pub fn set_quality_preference(state: State<'_, AppState>, value: u8) -> Result<(), String> {
    state.lock()?.quality = validate_percent(value)?;
    Ok(())
}

#[tauri::command]
pub fn list_audio_sources(state: State<'_, AppState>) -> Result<Vec<AppSourceDto>, String> {
    Ok(state.lock()?.apps.clone())
}

#[tauri::command]
pub fn list_audio_outputs(
    controller: State<'_, ProcessingController>,
) -> Result<Vec<String>, String> {
    Ok(controller.physical_outputs())
}

#[tauri::command]
pub fn set_app_override(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    let mut current = state.lock()?;
    let source = current
        .apps
        .iter_mut()
        .find(|source| source.id == id)
        .ok_or_else(|| "unknown audio source".to_string())?;
    if source.bypass_reason == Some(AudioBypassReason::Communication) && enabled {
        return Err("communication audio is bypassed by default".into());
    }
    source.enabled = enabled;
    Ok(())
}

#[tauri::command]
pub fn set_output_route(state: State<'_, AppState>, mode: OutputMode) -> Result<(), String> {
    let mut current = state.lock()?;
    let virtual_available = current.virtual_output_available;
    current.output_mode = validate_output_mode(mode, virtual_available)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_percent_command_values() {
        assert_eq!(validate_percent(0), Ok(0));
        assert_eq!(validate_percent(100), Ok(100));
        assert!(validate_percent(101).is_err());
    }

    #[test]
    fn enabling_processing_requires_a_ready_backend() {
        assert!(validate_master_enable(ProcessingBackendStatus::ComponentRequired, true).is_err());
        assert!(validate_master_enable(ProcessingBackendStatus::Unsupported, true).is_err());
        assert_eq!(
            validate_master_enable(ProcessingBackendStatus::Ready, true),
            Ok(())
        );
        assert_eq!(
            validate_master_enable(ProcessingBackendStatus::Faulted, false),
            Ok(())
        );
    }

    #[test]
    fn virtual_routes_require_a_virtual_output() {
        assert!(validate_output_mode(OutputMode::Virtual, false).is_err());
        assert!(validate_output_mode(OutputMode::Both, false).is_err());
        assert_eq!(
            validate_output_mode(OutputMode::Physical, false),
            Ok(OutputMode::Physical)
        );
        assert_eq!(
            validate_output_mode(OutputMode::Virtual, true),
            Ok(OutputMode::Virtual)
        );
    }
}
