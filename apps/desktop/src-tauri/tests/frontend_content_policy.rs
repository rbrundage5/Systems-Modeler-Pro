use serde_json::Value;
use std::path::Path;

fn configuration() -> Value {
    serde_json::from_str(include_str!("../tauri.conf.json")).unwrap()
}

#[test]
fn frontend_cannot_load_remote_code_or_send_model_data_over_browser_network_apis() {
    let config = configuration();
    let security = &config["app"]["security"];
    let csp = security["csp"].as_object().expect("CSP must be enabled");
    for directive in [
        "default-src",
        "object-src",
        "base-uri",
        "frame-src",
        "frame-ancestors",
        "form-action",
        "worker-src",
    ] {
        assert_eq!(csp[directive], "'none'", "{directive}");
    }
    assert_eq!(csp["script-src"], "'self'");
    assert_eq!(
        csp["connect-src"],
        "ipc: http://ipc.localhost https://ipc.localhost"
    );
    assert_eq!(csp["img-src"], "'self' data: blob:");
    assert_eq!(csp["font-src"], "'self'");
    // Tauri must still add its required IPC script hashes/nonces at build time.
    assert!(security.get("dangerousDisableAssetCspModification").is_none());
    assert!(security.get("devCsp").is_none());
}

#[test]
fn diagram_styles_remain_supported_without_relaxing_script_permissions() {
    let config = configuration();
    assert_eq!(
        config["app"]["security"]["csp"]["style-src"],
        "'self' 'unsafe-inline'"
    );
    let script_policy = config["app"]["security"]["csp"]["script-src"]
        .as_str()
        .unwrap();
    for forbidden in ["unsafe-inline", "unsafe-eval", "data:", "blob:", "*"] {
        assert!(!script_policy.contains(forbidden));
    }
}

#[test]
fn both_entrypoints_use_existing_local_script_and_style_assets() {
    let frontend = Path::new(env!("CARGO_MANIFEST_DIR")).join("../frontend");
    for name in ["index.html", "updater.html"] {
        let html = std::fs::read_to_string(frontend.join(name)).unwrap();
        assert!(!html.contains("<style"), "{name}: use local CSS assets");
        for script in html.split("<script").skip(1) {
            let (attributes, rest) = script.split_once('>').unwrap();
            let source = attributes
                .split_once("src=\"")
                .expect("inline scripts are not permitted")
                .1
                .split('"')
                .next()
                .unwrap();
            assert!(source.ends_with(".js"));
            assert!(!source.contains([':', '/', '\\']));
            assert!(frontend.join(source).is_file(), "{name}: {source}");
            assert!(rest.split_once("</script>").unwrap().0.trim().is_empty());
        }
    }
    assert!(frontend.join("updater.css").is_file());
    assert!(
        std::fs::read_to_string(frontend.join("updater.html"))
            .unwrap()
            .contains("href=\"updater.css\"")
    );
}
