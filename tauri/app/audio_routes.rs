use tauri::{AppHandle, State};
use voxveil_types::{AudioRouteChoice, ProcessingBackendStatus, ProcessingEngineKind};

use super::dto::AppViewState;
use super::state::AppState;
use crate::platform::{BackendSnapshot, ProcessingController};

pub(super) fn validate_master_enable(
    route: AudioRouteChoice,
    status: ProcessingBackendStatus,
    enabled: bool,
) -> Result<(), String> {
    if enabled && route == AudioRouteChoice::OwnedFilePlayback {
        return Err(
            "system processing cannot be enabled while local file playback is selected".into(),
        );
    }
    if enabled && status != ProcessingBackendStatus::Ready {
        Err("processing backend is unavailable".into())
    } else {
        Ok(())
    }
}

pub(super) fn validate_master_enable_before_start(
    route: AudioRouteChoice,
    snapshot: &BackendSnapshot,
    enabled: bool,
) -> Result<(), String> {
    let configured_relay = snapshot.status == ProcessingBackendStatus::RoutingRequired
        && snapshot.physical_output_endpoint_id.is_some()
        && matches!(
            snapshot.backend_kind.as_deref(),
            Some("vb-cable-relay") | Some("voxveil-cable-relay")
        );
    if enabled && route != AudioRouteChoice::OwnedFilePlayback && configured_relay {
        Ok(())
    } else {
        validate_master_enable(route, snapshot.status, enabled)
    }
}

pub(super) fn validate_engine_for_route(
    route: AudioRouteChoice,
    engine: ProcessingEngineKind,
) -> Result<(), String> {
    if route == AudioRouteChoice::OwnedFilePlayback && engine != ProcessingEngineKind::Dsp {
        Err("local file playback requires the DSP engine".into())
    } else {
        Ok(())
    }
}

#[tauri::command]
pub async fn set_audio_route(
    app: AppHandle,
    state: State<'_, AppState>,
    controller: State<'_, ProcessingController>,
    route: AudioRouteChoice,
) -> Result<AppViewState, String> {
    #[cfg(target_os = "windows")]
    {
        let (previous_route, was_enabled, vocal_level) = {
            let current = state.lock()?;
            (
                current.audio_route_choice,
                current.master_enabled,
                current.vocal_level,
            )
        };
        let mut previous_preferences = crate::config::windows_audio::load(&app)?;
        previous_preferences.route_choice = previous_route;
        let mut next_preferences = previous_preferences.clone();
        next_preferences.route_choice = route;

        crate::config::windows_audio::save(&app, &next_preferences)?;

        if was_enabled {
            if let Err(error) = controller.set_enabled(false, vocal_level) {
                return Err(rollback_audio_route(
                    &app,
                    &controller,
                    previous_route,
                    &previous_preferences,
                    was_enabled,
                    vocal_level,
                    error,
                ));
            }
        }

        let snapshot = match controller.set_interception_policy(route.interception_policy()) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return Err(rollback_audio_route(
                    &app,
                    &controller,
                    previous_route,
                    &previous_preferences,
                    was_enabled,
                    vocal_level,
                    error,
                ));
            }
        };

        let snapshot = if was_enabled && route != AudioRouteChoice::OwnedFilePlayback {
            match controller.set_enabled(true, vocal_level) {
                Ok(snapshot) if validate_master_enable(route, snapshot.status, true).is_ok() => {
                    snapshot
                }
                Ok(snapshot) => {
                    let error = snapshot
                        .detail
                        .unwrap_or_else(|| "processing backend is unavailable".into());
                    return Err(rollback_audio_route(
                        &app,
                        &controller,
                        previous_route,
                        &previous_preferences,
                        was_enabled,
                        vocal_level,
                        error,
                    ));
                }
                Err(error) => {
                    return Err(rollback_audio_route(
                        &app,
                        &controller,
                        previous_route,
                        &previous_preferences,
                        was_enabled,
                        vocal_level,
                        error,
                    ));
                }
            }
        } else {
            snapshot
        };

        let mut current = state.lock()?;
        current.audio_route_choice = route;
        current.audio_route_error = None;
        current.apply_backend(&snapshot);
        current.audio_route_choice = route;
        if route == AudioRouteChoice::OwnedFilePlayback {
            current.engine = ProcessingEngineKind::Dsp;
        }
        return Ok(current.clone());
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, state, controller, route);
        Err("Windows audio routes are unavailable on this platform build".into())
    }
}

#[cfg(target_os = "windows")]
fn rollback_audio_route(
    app: &AppHandle,
    controller: &ProcessingController,
    previous_route: AudioRouteChoice,
    previous_preferences: &crate::config::windows_audio::WindowsAudioPreferences,
    was_enabled: bool,
    vocal_level: u8,
    cause: String,
) -> String {
    let controller_rollback = controller
        .set_interception_policy(previous_route.interception_policy())
        .and_then(|_| {
            if was_enabled {
                controller.set_enabled(true, vocal_level).map(|_| ())
            } else {
                Ok(())
            }
        });
    let preference_rollback = crate::config::windows_audio::save(app, previous_preferences);
    match (controller_rollback, preference_rollback) {
        (Ok(()), Ok(())) => cause,
        (controller_result, preference_result) => format!(
            "{cause}; route rollback errors: controller={:?}; preferences={:?}",
            controller_result.err(),
            preference_result.err()
        ),
    }
}

#[cfg(test)]
mod route_validation_tests {
    use super::{
        validate_engine_for_route, validate_master_enable, validate_master_enable_before_start,
    };
    use crate::platform::BackendSnapshot;
    use voxveil_types::{AudioRouteChoice, ProcessingBackendStatus, ProcessingEngineKind};

    #[test]
    fn owned_file_route_never_enables_system_processing() {
        assert!(
            validate_master_enable(
                AudioRouteChoice::OwnedFilePlayback,
                ProcessingBackendStatus::Ready,
                true,
            )
            .is_err()
        );
        assert!(
            validate_master_enable(
                AudioRouteChoice::OwnedFilePlayback,
                ProcessingBackendStatus::Unsupported,
                false,
            )
            .is_ok()
        );
    }

    #[test]
    fn configured_relay_can_start_from_routing_required_state() {
        for backend_kind in ["vb-cable-relay", "voxveil-cable-relay"] {
            let snapshot = BackendSnapshot {
                status: ProcessingBackendStatus::RoutingRequired,
                detail: Some("relay is configured and ready to start".into()),
                backend_kind: Some(backend_kind.into()),
                physical_output: Some("Speakers".into()),
                physical_output_endpoint_id: Some("speakers-id".into()),
                per_app_available: false,
            };
            assert!(validate_master_enable_before_start(
                AudioRouteChoice::PhysicalApo,
                &snapshot,
                true,
            ).is_ok());
        }
    }

    #[test]
    fn relay_start_preflight_still_rejects_missing_prerequisites() {
        let missing_output = BackendSnapshot {
            status: ProcessingBackendStatus::RoutingRequired,
            detail: Some("select a physical output".into()),
            backend_kind: Some("vb-cable-relay".into()),
            physical_output: None,
            physical_output_endpoint_id: None,
            per_app_available: false,
        };
        assert!(
            validate_master_enable_before_start(
                AudioRouteChoice::PhysicalApo,
                &missing_output,
                true,
            )
            .is_err()
        );

        let apo_route = BackendSnapshot {
            physical_output_endpoint_id: Some("speakers-id".into()),
            backend_kind: Some("apo".into()),
            ..missing_output
        };
        assert!(
            validate_master_enable_before_start(AudioRouteChoice::PhysicalApo, &apo_route, true,)
                .is_err()
        );
    }

    #[test]
    fn owned_file_route_requires_the_dsp_engine() {
        assert!(
            validate_engine_for_route(
                AudioRouteChoice::OwnedFilePlayback,
                ProcessingEngineKind::Dsp,
            )
            .is_ok()
        );
        assert!(
            validate_engine_for_route(
                AudioRouteChoice::OwnedFilePlayback,
                ProcessingEngineKind::Auto,
            )
            .is_err()
        );
        assert!(
            validate_engine_for_route(AudioRouteChoice::PhysicalApo, ProcessingEngineKind::Auto,)
                .is_ok()
        );
    }
}
