use tauri::State;
use voxveil_types::{AudioBypassReason, OutputMode, ProcessingEngineKind, ProcessingMode};

use super::{dto::{AppSourceDto, AppViewState}, state::AppState};

fn validate_percent(value: u8) -> Result<u8, String> {
    if value <= 100 { Ok(value) } else { Err("value must be between 0 and 100".into()) }
}

fn validate_output_mode(mode: OutputMode, virtual_available: bool) -> Result<OutputMode, String> {
    if !virtual_available && matches!(mode, OutputMode::Virtual | OutputMode::Both) {
        Err("virtual output is unavailable".into())
    } else {
        Ok(mode)
    }
}

#[tauri::command]
pub fn get_app_state(state: State<'_, AppState>) -> Result<AppViewState, String> {
    Ok(state.lock()?.clone())
}

#[tauri::command]
pub fn set_master_enabled(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state.lock()?.master_enabled = enabled;
    Ok(())
}

#[tauri::command]
pub fn set_processing_mode(state: State<'_, AppState>, mode: ProcessingMode) -> Result<(), String> {
    state.lock()?.processing_mode = mode;
    Ok(())
}

#[tauri::command]
pub fn set_engine(state: State<'_, AppState>, engine: ProcessingEngineKind) -> Result<(), String> {
    state.lock()?.engine = engine;
    Ok(())
}

#[tauri::command]
pub fn set_vocal_level(state: State<'_, AppState>, value: u8) -> Result<(), String> {
    state.lock()?.vocal_level = validate_percent(value)?;
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
pub fn list_audio_outputs(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    Ok(vec![state.lock()?.physical_output.clone()])
}

#[tauri::command]
pub fn set_app_override(state: State<'_, AppState>, id: String, enabled: bool) -> Result<(), String> {
    let mut current = state.lock()?;
    let source = current.apps.iter_mut().find(|source| source.id == id).ok_or_else(|| "unknown audio source".to_string())?;
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
    fn virtual_routes_require_a_virtual_output() {
        assert!(validate_output_mode(OutputMode::Virtual, false).is_err());
        assert!(validate_output_mode(OutputMode::Both, false).is_err());
        assert_eq!(validate_output_mode(OutputMode::Physical, false), Ok(OutputMode::Physical));
        assert_eq!(validate_output_mode(OutputMode::Virtual, true), Ok(OutputMode::Virtual));
    }
}
