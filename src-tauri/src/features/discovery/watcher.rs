use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::Emitter;

pub struct WatcherHandle {
    _watcher: RecommendedWatcher,
}

pub fn start_watcher(
    app: tauri::AppHandle,
    paths: Vec<std::path::PathBuf>,
) -> Result<WatcherHandle, crate::app_core::error::AppError> {
    let app_clone = app.clone();
    let mut watcher = notify::recommended_watcher(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
                    let _ = app_clone.emit(
                        "mods-changed",
                        serde_json::json!({
                            "changeType": format!("{:?}", event.kind),
                        }),
                    );
                }
            }
        },
    )
    .map_err(|e| crate::app_core::error::AppError::Io(e.to_string()))?;

    for path in &paths {
        if path.exists() {
            watcher
                .watch(path, RecursiveMode::Recursive)
                .map_err(|e| crate::app_core::error::AppError::Io(e.to_string()))?;
        }
    }

    Ok(WatcherHandle { _watcher: watcher })
}
