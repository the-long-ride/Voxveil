#![forbid(unsafe_code)]

mod app;
pub mod audio;
mod config;
pub mod platform;
pub mod realtime;
pub mod routing;
pub mod security;
pub mod separation;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = app::state::AppState::default();
    tauri::Builder::default()
        .manage(state)
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
        ])
        .run(tauri::generate_context!())
        .expect("Voxveil failed to start");
}
