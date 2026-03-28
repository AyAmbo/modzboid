// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Check for CLI mode
    if project_modzboid_lib::cli::is_cli_mode() {
        if project_modzboid_lib::cli::run_cli() {
            return;
        }
    }

    project_modzboid_lib::run()
}
