// OpenRevo - Auto Updater Module
// Pure ASCII compliant

pub mod types;
pub mod checker;
pub mod installer;

pub use types::*;
pub use checker::*;
pub use installer::*;

#[tauri::command]
pub async fn check_for_updates_cmd(preferred_source: Option<String>) -> Result<UpdateCheckResult, String> {
    checker::check_for_updates(preferred_source).await
}

#[tauri::command]
pub async fn download_and_install_update_cmd(
    app: tauri::AppHandle,
    download_url: String,
    expected_sha256: String,
    filename: String,
    is_installer: bool,
) -> Result<(), String> {
    let downloaded_path = installer::download_update_asset(&app, &download_url, &expected_sha256, &filename).await?;
    installer::apply_update(&downloaded_path, is_installer)?;
    app.exit(0);
    Ok(())
}
