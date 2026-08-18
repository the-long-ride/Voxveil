#![forbid(unsafe_code)]

mod app;
pub mod audio;
mod config;
pub mod models;
pub mod platform;
pub mod realtime;
pub mod routing;
pub mod security;
pub mod separation;

pub fn verify_embedded_system_audio_payload() -> Result<(), String> {
    app::system_audio::verify_embedded_payload()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let controller = platform::ProcessingController::default();
    let snapshot = controller.snapshot();
    let mut view_state = app::dto::AppViewState::default();
    view_state.apply_backend(&snapshot);
    let state = app::state::AppState::new(view_state);
    let model_manager = models::manager::ModelManager::default();
    tauri::Builder::default()
        .manage(state)
        .manage(controller)
        .manage(model_manager)
        .invoke_handler(tauri::generate_handler![
            app::commands::get_app_state,
            app::commands::set_master_enabled,
            app::commands::set_processing_mode,
            app::commands::set_engine,
            app::commands::set_vocal_level,
            app::commands::set_quality_preference,
            app::commands::list_audio_sources,
            app::commands::list_audio_outputs,
            app::commands::set_app_override,
            app::commands::set_output_route,
            app::system_audio::install_windows_audio_component,
            models::commands::get_ai_model_status,
            models::commands::install_ai_model,
            models::commands::remove_ai_model,
        ])
        .run(tauri::generate_context!())
        .expect("Voxveil failed to start");
}
