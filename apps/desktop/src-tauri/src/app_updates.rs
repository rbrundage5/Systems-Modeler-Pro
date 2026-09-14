//! Updates are offered in a separate startup window before modeling is accessible.
mod gate;

use gate::{Gate, Phase};
use serde::Serialize;
use std::{sync::Mutex, time::Duration};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_updater::{Update, UpdaterExt};

const ENDPOINT: &str =
    "https://github.com/rbrundage5/Systems-Modeler-Pro/releases/latest/download/latest.json";
const RELEASE_PREFIX: &str = "https://github.com/rbrundage5/Systems-Modeler-Pro/releases/download/";
const PUBLIC_KEY: &str = match option_env!("SMP_UPDATER_PUBLIC_KEY") {
    Some(key) => key,
    None => "",
};

#[derive(Default)]
pub struct UpdateState(Mutex<Gate<Update>>);

#[derive(Serialize)]
pub struct UpdateInfo {
    current_version: String,
    version: Option<String>,
}

fn startup_window(label: &str) -> Result<(), String> {
    if label != "updater" {
        return Err("Update commands are restricted to the startup window.".into());
    }
    Ok(())
}

fn lock(state: &UpdateState) -> Result<std::sync::MutexGuard<'_, Gate<Update>>, String> {
    state
        .0
        .lock()
        .map_err(|_| "Update state is unavailable. Restart the application.".into())
}

pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(windows) && !cfg!(debug_assertions) && !PUBLIC_KEY.trim().is_empty() {
        WebviewWindowBuilder::new(app, "updater", WebviewUrl::App("updater.html".into()))
            .title("Systems Modeler Pro — Updates")
            .inner_size(580.0, 340.0)
            .resizable(false)
            .build()?;
    } else {
        show_application(app.handle()).map_err(std::io::Error::other)?;
    }
    Ok(())
}

fn show_application(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<UpdateState>();
    let mut gate = lock(&state)?;
    if gate.phase == Phase::Installing {
        return Err("The update is being installed. Please wait.".into());
    }
    let main = app
        .get_webview_window("main")
        .ok_or("Application window is unavailable.")?;
    main.show().map_err(|error| error.to_string())?;
    gate.open().map_err(str::to_string)?;
    drop(gate);
    let _ = main.set_focus();
    if let Some(window) = app.get_webview_window("updater") {
        window.close().map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn on_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    if window.label() != "updater" {
        return;
    }
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        let state = window.state::<UpdateState>();
        let is_open = lock(&state)
            .map(|gate| gate.phase == Phase::Open)
            .unwrap_or(false);
        if !is_open {
            api.prevent_close();
            // Closing the startup screen is equivalent to choosing Open application.
            // During installation the gate refuses this transition.
            let _ = show_application(window.app_handle());
        }
    }
}

#[tauri::command]
pub fn open_application(window: WebviewWindow, app: AppHandle) -> Result<(), String> {
    startup_window(window.label())?;
    show_application(&app)
}

fn approved_download(url: &str) -> bool {
    url.strip_prefix(RELEASE_PREFIX)
        .is_some_and(|path| !path.is_empty() && path.ends_with("-setup.exe"))
}

#[tauri::command]
pub async fn check_app_update(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, UpdateState>,
) -> Result<UpdateInfo, String> {
    startup_window(window.label())?;
    if !cfg!(windows) || cfg!(debug_assertions) || PUBLIC_KEY.trim().is_empty() {
        return Err("This development build does not use automatic updates.".into());
    }
    lock(&state)?.begin_check().map_err(str::to_string)?;
    let result = async {
        let endpoint = ENDPOINT
            .parse()
            .map_err(|error| format!("Invalid update endpoint: {error}"))?;
        let updater = app
            .updater_builder()
            .pubkey(PUBLIC_KEY)
            .endpoints(vec![endpoint])
            .map_err(|error| error.to_string())?
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|error| error.to_string())?;
        let mut update = updater.check().await.map_err(|error| error.to_string())?;
        if let Some(candidate) = update.as_mut() {
            if !approved_download(candidate.download_url.as_str()) {
                return Err("The update is not a Windows installer from this repository.".into());
            }
            candidate.timeout = Some(Duration::from_secs(300));
        }
        Ok::<_, String>(update)
    }
    .await;
    match result {
        Ok(update) => {
            let info = UpdateInfo {
                current_version: app.package_info().version.to_string(),
                version: update.as_ref().map(|candidate| candidate.version.clone()),
            };
            if !lock(&state)?.finish_check(update) {
                return Err("The application has already been opened.".into());
            }
            Ok(info)
        }
        Err(error) => {
            lock(&state)?.finish_check(None);
            Err(format!(
                "Could not check for updates. You can still open the application. {error}"
            ))
        }
    }
}

#[tauri::command]
pub async fn install_app_update(
    window: WebviewWindow,
    state: State<'_, UpdateState>,
) -> Result<(), String> {
    startup_window(window.label())?;
    let update = lock(&state)?.begin_install().map_err(str::to_string)?;
    // No bytes, URL, key or installer path are accepted from the frontend.
    // download verifies the mandatory Tauri signature before install can run.
    let result = async {
        let bytes = update
            .download(|_, _| {}, || {})
            .await
            .map_err(|error| error.to_string())?;
        update.install(bytes).map_err(|error| error.to_string())
    }
    .await;
    if let Err(error) = result {
        lock(&state)?.install_failed();
        return Err(format!(
            "The update was not installed. You can retry or open the existing version. {error}"
        ));
    }
    // On Windows the plugin launches the passive installer with restart enabled,
    // cleans up Tauri and exits. No workspace was opened in this process.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_window_cannot_invoke_startup_update_commands() {
        assert!(startup_window("main").is_err());
        assert!(startup_window("other").is_err());
        assert!(startup_window("updater").is_ok());
    }

    #[test]
    fn only_this_repositorys_windows_release_assets_are_accepted() {
        assert!(approved_download(&format!(
            "{RELEASE_PREFIX}desktop-v0.1.1/app-setup.exe"
        )));
        for url in [
            "http://github.com/rbrundage5/Systems-Modeler-Pro/releases/download/v1/app-setup.exe",
            "https://github.com/other/repo/releases/download/v1/app-setup.exe",
            "https://github.com.evil.invalid/rbrundage5/Systems-Modeler-Pro/releases/download/v1/app-setup.exe",
            "file:///tmp/app-setup.exe",
            "https://github.com/rbrundage5/Systems-Modeler-Pro/releases/download/v1/script.ps1",
        ] {
            assert!(!approved_download(url));
        }
    }
}
