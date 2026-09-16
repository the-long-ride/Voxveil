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
        .setup(|app| {
            #[cfg(target_os = "windows")]
            {
                use tauri::Manager;

                // Audio endpoint IDs are intentionally persistent, but devices can be
                // unplugged or removed between runs. A stale nonessential preference
                // must never prevent the application from starting.
                let mut prefs = config::windows_audio::load(app.handle()).unwrap_or_default();
                if let Some(endpoint_id) = prefs.physical_output_endpoint_id.clone() {
                    let controller = app.state::<platform::ProcessingController>();
                    if controller
                        .set_physical_output(Some(endpoint_id))
                        .is_err()
                    {
                        prefs.physical_output_endpoint_id = None;
                        let _ = config::windows_audio::save(app.handle(), &prefs);
                    }
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app::commands::get_app_state,
            app::system_audio::list_system_audio_endpoints,
            app::system_audio::install_system_audio_component,
            app::system_audio_actions::set_physical_audio_output,
            app::system_audio_actions::open_windows_sound_settings,
            app::system_audio_actions::open_vb_cable_download,
            app::commands::set_master_enabled,
            app::commands::set_processing_mode,
            app::commands::set_engine,
            app::commands::set_classic_suppression_profile,
            app::commands::set_vocal_level,
            app::commands::set_quality_preference,
            app::commands::list_audio_sources,
            app::commands::list_audio_outputs,
            app::commands::set_app_override,
            app::commands::set_output_route,
            models::commands::get_ai_model_status,
            models::commands::install_ai_model,
            models::commands::remove_ai_model,
        ])
        .run(tauri::generate_context!())
        .expect("Voxveil failed to start");
}
