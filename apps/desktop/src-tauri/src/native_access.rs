//! Custom application IPC is limited to the two bundled entrypoint documents.
//! This does not grant plugin permissions or authorize arbitrary file paths.

pub fn guard<R, F>(handler: F) -> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static
where
    R: tauri::Runtime,
    F: Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static,
{
    move |invoke| {
        // Read the current document from the native webview, never from IPC args.
        let webview = invoke.message.webview();
        let document_url = webview.url().ok();
        if !command_allowed(
            webview.label(),
            document_url.as_ref().map(|url| url.as_str()),
            invoke.message.command(),
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

const UPDATER_COMMANDS: &[&str] = &[
    "check_app_update",
    "install_app_update",
    "open_application",
];

pub fn command_allowed(window: &str, document_url: Option<&str>, command: &str) -> bool {
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
                "open_project_file",
                "save_project_file",
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
}
