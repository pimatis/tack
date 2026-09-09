use tauri::Manager;

pub(crate) fn app_db_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| {
            std::fs::create_dir_all(&dir).ok();
            dir.join("tack.db")
        })
        .map_err(|e| e.to_string())
}
