//! Custom application IPC is limited to the application's entrypoint documents.
//! This does not grant plugin permissions or authorize arbitrary file paths.

use tauri::Manager;

pub fn guard<R, F>(handler: F) -> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static
where
    R: tauri::Runtime,
    F: Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static,
{
    move |invoke| {
        // Read the current document from the native webview, never from IPC args.
        let webview = invoke.message.webview();
        let document_url = webview.url().ok();
        if !command_allowed_for_build(
            webview.label(),
            document_url.as_ref().map(|url| url.as_str()),
            invoke.message.command(),
            tauri::is_dev(),
            webview.config().build.dev_url.as_ref(),
        ) {
            invoke
                .resolver
                .reject("This document is not permitted to invoke that application command.");
            return true;
        }
        handler(invoke)
    }
}

const MAIN_DOCUMENTS: &[&str] = &[
    "tauri://localhost/",
    "tauri://localhost/index.html",
    "http://tauri.localhost/",
    "http://tauri.localhost/index.html",
    "https://tauri.localhost/",
    "https://tauri.localhost/index.html",
];

const UPDATER_DOCUMENTS: &[&str] = &[
    "tauri://localhost/updater.html",
    "http://tauri.localhost/updater.html",
    "https://tauri.localhost/updater.html",
];

const UPDATER_COMMANDS: &[&str] = &["check_app_update", "install_app_update", "open_application"];

fn command_allowed_for_build(
    window: &str,
    document_url: Option<&str>,
    command: &str,
    development: bool,
    configured_dev_url: Option<&tauri::Url>,
) -> bool {
    if command_allowed(window, document_url, command) {
        return true;
    }
    if !development || window != "main" || UPDATER_COMMANDS.contains(&command) {
        return false;
    }
    let (Some(document_url), Some(dev_url)) = (document_url, configured_dev_url) else {
        return false;
    };
    // `tauri dev` injects build.devUrl for its built-in asset server even though
    // tauri.conf.json only specifies frontendDist. Trust that native config's
    // exact entrypoint, never a frontend argument or an arbitrary localhost port.
    if !matches!(dev_url.scheme(), "http" | "https")
        || !matches!(dev_url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
        || !dev_url.username().is_empty()
        || dev_url.password().is_some()
        || dev_url.query().is_some()
        || dev_url.fragment().is_some()
        || !(dev_url.path().ends_with('/') || dev_url.path().ends_with("/index.html"))
    {
        return false;
    }
    document_url == dev_url.as_str()
        || dev_url
            .join("index.html")
            .is_ok_and(|index| document_url == index.as_str())
}

fn command_allowed(window: &str, document_url: Option<&str>, command: &str) -> bool {
    let Some(document_url) = document_url else {
        return false;
    };
    match window {
        "main" => MAIN_DOCUMENTS.contains(&document_url) && !UPDATER_COMMANDS.contains(&command),
        "updater" => {
            UPDATER_DOCUMENTS.contains(&document_url) && UPDATER_COMMANDS.contains(&command)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_modeling_and_file_commands_remain_available_only_to_main() {
        for url in MAIN_DOCUMENTS {
            for command in [
                "new_project",
                "open_project_file",
                "open_project_file_complete",
                "save_project_file",
                "save_project_file_complete",
                "save_current_project_complete",
                "workspace_snapshot_complete",
                "apply_model_script",
                "create_activity_diagram",
                "collaboration_connect",
                "history_undo",
            ] {
                assert!(command_allowed("main", Some(url), command));
                assert!(!command_allowed("updater", Some(url), command));
            }
            for command in UPDATER_COMMANDS {
                assert!(!command_allowed("main", Some(url), command));
            }
        }
    }

    #[test]
    fn updater_has_exactly_its_three_startup_commands() {
        for url in UPDATER_DOCUMENTS {
            for command in UPDATER_COMMANDS {
                assert!(command_allowed("updater", Some(url), command));
                assert!(!command_allowed("main", Some(url), command));
            }
            for command in [
                "open_project_file",
                "save_project_file",
                "apply_model_script",
                "stage_xmi_upload",
                "discard_staged_spreadsheet",
                "collaboration_connect",
                "new_project",
                "unknown_command",
            ] {
                assert!(!command_allowed("updater", Some(url), command));
            }
        }
    }

    #[test]
    fn unknown_windows_and_unavailable_urls_fail_closed() {
        for window in ["", "other", "MAIN", "main-child", "updater-child"] {
            assert!(!command_allowed(
                window,
                Some(MAIN_DOCUMENTS[0]),
                "new_project"
            ));
            assert!(!command_allowed(
                window,
                Some(UPDATER_DOCUMENTS[0]),
                "install_app_update"
            ));
        }
        assert!(!command_allowed("main", None, "open_project_file"));
        assert!(!command_allowed("updater", None, "install_app_update"));
    }

    #[test]
    fn remote_file_data_and_lookalike_documents_cannot_invoke_native_commands() {
        for url in [
            "https://example.invalid/index.html",
            "http://localhost/index.html",
            "http://127.0.0.1/index.html",
            "https://tauri.localhost.evil.invalid/index.html",
            "https://tauri.localhost@evil.invalid/index.html",
            "https://tauri.localhost:444/index.html",
            "https://tauri.localhost/other.html",
            "https://tauri.localhost/index.html?remote=true",
            "https://tauri.localhost/updater.html?remote=true",
            "file:///tmp/index.html",
            "data:text/html,untrusted",
            "blob:https://tauri.localhost/untrusted",
            "about:blank",
            "",
        ] {
            assert!(!command_allowed("main", Some(url), "open_project_file"));
            assert!(!command_allowed("updater", Some(url), "install_app_update"));
        }
    }

    #[test]
    fn configured_development_entrypoints_allow_project_workflows() {
        for base in [
            "http://localhost:1430/",
            "http://127.0.0.1:1431/",
            "http://[::1]:1432/",
            "https://localhost:5173/",
            "http://localhost:5173/frontend/",
            "http://localhost:5173/frontend/index.html",
        ] {
            let dev_url = tauri::Url::parse(base).unwrap();
            for document in [dev_url.clone(), dev_url.join("index.html").unwrap()] {
                for command in [
                    "new_project",
                    "open_project_file",
                    "open_project_file_complete",
                    "save_project_file_complete",
                    "save_current_project_complete",
                    "workspace_snapshot_complete",
                    "activity_snapshot",
                    "history_undo",
                    "history_redo",
                ] {
                    assert!(
                        command_allowed_for_build(
                            "main",
                            Some(document.as_str()),
                            command,
                            true,
                            Some(&dev_url),
                        ),
                        "{command} rejected at {document}"
                    );
                }
            }
        }
    }

    #[test]
    fn development_access_requires_native_configuration_and_development_build() {
        let dev_url = tauri::Url::parse("http://localhost:1430/").unwrap();
        for (development, configured) in [(false, None), (false, Some(&dev_url)), (true, None)] {
            assert!(!command_allowed_for_build(
                "main",
                Some(dev_url.as_str()),
                "new_project",
                development,
                configured,
            ));
        }
        assert!(!command_allowed_for_build(
            "main",
            None,
            "new_project",
            true,
            Some(&dev_url),
        ));
    }

    #[test]
    fn development_configuration_does_not_grant_other_documents_or_origins() {
        let dev_url = tauri::Url::parse("http://localhost:1430/").unwrap();
        for document in [
            "http://localhost:1431/",
            "http://127.0.0.1:1430/",
            "https://localhost:1430/",
            "http://localhost/",
            "http://localhost:1430/other.html",
            "http://localhost:1430/updater.html",
            "http://localhost:1430/index.html?remote=true",
            "http://localhost:1430/index.html#other",
            "http://localhost.evil.invalid:1430/",
            "http://localhost:1430@evil.invalid/",
            "http://user@localhost:1430/",
            "http://localhost:1430/index.html/other.html",
            "file:///tmp/index.html",
            "data:text/html,untrusted",
            "about:blank",
            "",
        ] {
            assert!(
                !command_allowed_for_build(
                    "main",
                    Some(document),
                    "new_project",
                    true,
                    Some(&dev_url),
                ),
                "unexpected access from {document}"
            );
        }
    }

    #[test]
    fn unsafe_development_configuration_cannot_authorize_itself() {
        for base in [
            "https://example.invalid/",
            "http://192.0.2.1:1430/",
            "http://localhost.evil.invalid:1430/",
            "http://user@localhost:1430/",
            "http://localhost:1430/?remote=true",
            "http://localhost:1430/#other",
            "http://localhost:1430/other.html",
            "file:///tmp/index.html",
            "data:text/html,untrusted",
        ] {
            let dev_url = tauri::Url::parse(base).unwrap();
            assert!(!command_allowed_for_build(
                "main",
                Some(base),
                "new_project",
                true,
                Some(&dev_url),
            ));
        }
    }

    #[test]
    fn development_access_preserves_window_and_updater_command_isolation() {
        let dev_url = tauri::Url::parse("http://localhost:1430/").unwrap();
        for window in ["main", "updater", "other", "main-child", ""] {
            for command in UPDATER_COMMANDS {
                assert!(!command_allowed_for_build(
                    window,
                    Some(dev_url.as_str()),
                    command,
                    true,
                    Some(&dev_url),
                ));
            }
            if window != "main" {
                assert!(!command_allowed_for_build(
                    window,
                    Some(dev_url.as_str()),
                    "new_project",
                    true,
                    Some(&dev_url),
                ));
            }
        }
        for development in [false, true] {
            for url in UPDATER_DOCUMENTS {
                for command in UPDATER_COMMANDS {
                    assert!(command_allowed_for_build(
                        "updater",
                        Some(url),
                        command,
                        development,
                        Some(&dev_url),
                    ));
                }
            }
        }
    }
}
