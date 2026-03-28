pub mod app_core;
pub mod features;

pub mod cli {
    pub use crate::features::cli::handler::{is_cli_mode, run_cli};
}

use tauri::Manager;
use tokio::sync::RwLock;
use app_core::config::{self, AppState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .register_uri_scheme_protocol("pzdocs", |_ctx, request| {
            use tauri::http::Response;
            let uri = request.uri().to_string();
            eprintln!("[pzdocs] request: {}", uri);

            // Strip scheme+host prefix (varies by platform)
            let raw = uri
                .strip_prefix("pzdocs://localhost")
                .or_else(|| uri.strip_prefix("pzdocs:///"))
                .or_else(|| uri.strip_prefix("pzdocs://"))
                .or_else(|| uri.strip_prefix("http://pzdocs.localhost/"))
                .or_else(|| uri.strip_prefix("https://pzdocs.localhost/"))
                .unwrap_or(&uri);

            // URL-decode (handles %20, %3A, etc.)
            let decoded = urlencoding::decode(raw).unwrap_or_else(|_| raw.into());
            let mut path_str = decoded.to_string();

            // Windows: strip leading / before drive letter (e.g., /C:/Users → C:/Users)
            if path_str.len() >= 3
                && path_str.starts_with('/')
                && path_str.as_bytes().get(2) == Some(&b':')
            {
                path_str = path_str[1..].to_string();
            }

            // Strip query string and fragment
            if let Some(pos) = path_str.find('?') {
                path_str.truncate(pos);
            }
            if let Some(pos) = path_str.find('#') {
                path_str.truncate(pos);
            }

            let file_path = std::path::Path::new(&path_str);
            eprintln!("[pzdocs] resolved: {:?} (exists={})", file_path, file_path.exists());

            if !file_path.exists() || !file_path.is_file() {
                eprintln!("[pzdocs] 404: file not found");
                return Response::builder()
                    .status(404)
                    .header("Content-Type", "text/plain")
                    .body(format!("Not found: {}", path_str).into_bytes())
                    .unwrap();
            }

            // Security: only serve from extensions directory
            let normalized = file_path.to_string_lossy().replace('\\', "/");
            if !normalized.contains("/extensions/") {
                eprintln!("[pzdocs] 403: not in extensions dir");
                return Response::builder()
                    .status(403)
                    .body(b"Forbidden".to_vec())
                    .unwrap();
            }

            let content = match std::fs::read(file_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[pzdocs] read error: {}", e);
                    return Response::builder()
                        .status(500)
                        .body(format!("Read error: {}", e).into_bytes())
                        .unwrap();
                }
            };

            let mime = match file_path.extension().and_then(|e| e.to_str()) {
                Some("html") | Some("htm") => "text/html; charset=utf-8",
                Some("css") => "text/css; charset=utf-8",
                Some("js") | Some("mjs") => "application/javascript; charset=utf-8",
                Some("json") => "application/json; charset=utf-8",
                Some("png") => "image/png",
                Some("jpg") | Some("jpeg") => "image/jpeg",
                Some("gif") => "image/gif",
                Some("svg") => "image/svg+xml",
                Some("woff") => "font/woff",
                Some("woff2") => "font/woff2",
                Some("ttf") => "font/ttf",
                Some("eot") => "application/vnd.ms-fontobject",
                Some("ico") => "image/x-icon",
                Some("txt") | Some("rst") => "text/plain; charset=utf-8",
                Some("xml") => "application/xml",
                _ => "application/octet-stream",
            };

            Response::builder()
                .status(200)
                .header("Content-Type", mime)
                .header("Access-Control-Allow-Origin", "*")
                .body(content)
                .unwrap()
        })
        .setup(|app| {
            // Portable mode: check for 'portable' marker file next to the executable
            let app_data = {
                let exe_dir = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|p| p.to_path_buf()));

                if let Some(ref dir) = exe_dir {
                    if dir.join("portable").exists() || dir.join("portable.txt").exists() {
                        let portable_data = dir.join("ProjectModzboid_Data");
                        std::fs::create_dir_all(&portable_data).ok();
                        portable_data
                    } else {
                        let default_dir = app.path().app_data_dir()?;
                        std::fs::create_dir_all(&default_dir)?;
                        default_dir
                    }
                } else {
                    let default_dir = app.path().app_data_dir()?;
                    std::fs::create_dir_all(&default_dir)?;
                    default_dir
                }
            };

            let loaded_config = config::load_config(&app_data)
                .unwrap_or_else(|_| config::default_config());

            let db = tauri::async_runtime::block_on(
                app_core::db::init_db(&app_data)
            )
            .map_err(|e| e.to_string())?;

            // Collect watch paths from config before moving it into AppState
            let mut watch_paths = vec![];
            if let Some(ref p) = loaded_config.workshop_path {
                watch_paths.push(p.clone());
            }
            if let Some(ref p) = loaded_config.local_mods_path {
                watch_paths.push(p.clone());
            }

            let incompat_db = features::conflicts::detector::load_incompat_db(&app_data);

            // Allow the asset protocol to serve files from the app data directory
            // (needed for the docs extension iframe)
            // Debug: write to file since eprintln may not show
            let extensions_dir = app_data.join("extensions");
            if let Err(e) = app.asset_protocol_scope().allow_directory(&extensions_dir, true) {
                eprintln!("Warning: could not add extensions dir to asset scope: {}", e);
            }

            // Enable devtools in debug builds
            #[cfg(debug_assertions)]
            if let Some(window) = app.get_webview_window("main") {
                window.open_devtools();
            }

            app.manage(AppState {
                db,
                config: RwLock::new(loaded_config),
                app_data_dir: app_data,
                watcher_handle: std::sync::Mutex::new(None),
                incompat_db: RwLock::new(incompat_db),
            });

            // Start file watcher for mod directories if paths are configured
            if !watch_paths.is_empty() {
                if let Ok(handle) = features::discovery::watcher::start_watcher(
                    app.handle().clone(),
                    watch_paths,
                ) {
                    let state: tauri::State<AppState> = app.state();
                    let mut wh = state.watcher_handle.lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    *wh = Some(handle);
                    drop(wh);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            config::get_config_cmd,
            config::save_config_cmd,
            features::discovery::commands::discover_mods,
            features::discovery::commands::get_mod_details,
            features::discovery::commands::refresh_mods,
            features::discovery::commands::rescan_mod_version,
            features::profiles::commands::list_profiles_cmd,
            features::profiles::commands::get_profile_cmd,
            features::profiles::commands::create_profile_cmd,
            features::profiles::commands::update_profile_cmd,
            features::profiles::commands::delete_profile_cmd,
            features::profiles::commands::duplicate_profile_cmd,
            features::profiles::commands::export_profile_cmd,
            features::profiles::commands::import_profile_cmd,
            features::load_order::commands::sort_load_order_cmd,
            features::load_order::commands::validate_load_order_cmd,
            features::load_order::commands::auto_resolve_deps_cmd,
            features::load_order::commands::reverse_deps_cmd,
            features::load_order::commands::get_community_rules_cmd,
            features::load_order::commands::save_community_rules_cmd,
            features::launcher::commands::detect_game_path_cmd,
            features::launcher::commands::detect_steam_path_cmd,
            features::launcher::commands::verify_game_path_cmd,
            features::launcher::commands::verify_steam_path_cmd,
            features::launcher::commands::detect_game_version_cmd,
            features::launcher::commands::import_from_game_cmd,
            features::launcher::commands::launch_game_cmd,
            features::launcher::commands::open_folder_cmd,
            features::conflicts::commands::detect_conflicts_cmd,
            features::server_config::commands::list_server_configs_cmd,
            features::server_config::commands::validate_server_config_cmd,
            features::server_config::commands::load_server_config_cmd,
            features::server_config::commands::save_server_config_cmd,
            features::sandbox::commands::load_sandbox_vars_cmd,
            features::sandbox::commands::save_sandbox_vars_cmd,
            features::workshop::commands::get_workshop_items_cmd,
            features::workshop::commands::open_workshop_page_cmd,
            features::workshop::commands::fetch_workshop_meta_cmd,
            features::workshop::commands::fetch_single_workshop_meta_cmd,
            features::workshop::commands::search_workshop_cmd,
            features::backup::commands::create_backup_cmd,
            features::backup::commands::list_backups_cmd,
            features::backup::commands::restore_backup_cmd,
            features::backup::commands::delete_backup_cmd,
            features::inspector::commands::inspect_mod_cmd,
            features::inspector::commands::check_mod_lua_cmd,
            features::inspector::commands::scan_mod_migration_cmd,
            features::inspector::commands::scan_all_mods_migration_cmd,
            features::inspector::commands::list_migration_versions_cmd,
            features::inspector::commands::scan_all_mods_compat_cmd,
            features::inspector::commands::scan_all_scripts_compat_cmd,
            features::inspector::commands::auto_fix_mod_cmd,
            features::inspector::commands::create_modpack_fixes_cmd,
            features::rcon::commands::rcon_command_cmd,
            features::rcon::commands::rcon_test_cmd,
            features::sharing::commands::export_mod_list_cmd,
            features::sharing::commands::parse_mod_list_import_cmd,
            features::sharing::commands::apply_mod_list_import_cmd,
            features::sharing::commands::load_mods_from_server_ini_cmd,
            features::sharing::commands::save_mods_to_server_ini_cmd,
            features::extensions::commands::list_extensions_cmd,
            features::extensions::commands::install_extension_cmd,
            features::extensions::commands::toggle_extension_cmd,
            features::extensions::commands::uninstall_extension_cmd,
            features::extensions::commands::get_replacements_cmd,
            features::extensions::commands::export_extension_cmd,
            features::diagnostics::commands::analyze_crash_log_cmd,
            features::diagnostics::commands::preflight_check_cmd,
            features::diagnostics::commands::bisect_start_cmd,
            features::diagnostics::commands::bisect_report_cmd,
            features::api_docs::commands::search_api_cmd,
            features::api_docs::commands::get_api_class_cmd,
            features::api_docs::commands::get_api_events_cmd,
            features::api_docs::commands::get_api_stats_cmd,
            features::api_docs::commands::list_api_classes_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
