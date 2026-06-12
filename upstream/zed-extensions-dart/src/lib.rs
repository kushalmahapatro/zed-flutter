//! Proposed logic for [zed-extensions/dart](https://github.com/zed-extensions/dart)
//! `get_dap_binary` — pass Flutter DAP fields through instead of hardcoding chrome/web/debug.
//!
//! Copy into `src/dap_config.rs` (or inline) when opening the upstream PR.

use serde_json::{json, Map, Value};

/// Build the JSON configuration string passed to `flutter debug_adapter` / `dart debug_adapter`.
pub fn build_flutter_dap_configuration(
    user_config: &Value,
    tool: &str,
    request: &str,
    program: &str,
    cwd: &str,
    program_args: &[String],
) -> Value {
    let mut dap = Map::new();
    dap.insert("type".into(), Value::String(tool.into()));
    dap.insert("request".into(), Value::String(request.into()));
    dap.insert("program".into(), Value::String(program.into()));
    dap.insert("cwd".into(), Value::String(cwd.into()));
    dap.insert(
        "args".into(),
        Value::Array(
            program_args
                .iter()
                .map(|s| Value::String(s.clone()))
                .collect(),
        ),
    );
    dap.insert("stopOnEntry".into(), Value::Bool(false));

    if let Some(mode) = flutter_mode(user_config) {
        dap.insert("flutterMode".into(), Value::String(mode));
    }

    if let Some(tool_args) = string_array(user_config, "toolArgs") {
        dap.insert("toolArgs".into(), json!(tool_args));
    }

    if let Some(id) = optional_str(user_config, "device_id").or(optional_str(user_config, "deviceId"))
    {
        dap.insert("deviceId".into(), Value::String(id.to_string()));
    }

    if let Some(platform) = optional_str(user_config, "platform") {
        dap.insert("platform".into(), Value::String(platform.to_string()));
    }

    if let Some(uri) = optional_str(user_config, "vmServiceUri") {
        dap.insert("vmServiceUri".into(), Value::String(uri.to_string()));
    }

    if let Some(path) = optional_str(user_config, "vmServiceInfoFile") {
        dap.insert(
            "vmServiceInfoFile".into(),
            Value::String(resolve_vm_service_info_file(cwd, path)),
        );
    }

    for key in [
        "sendLogsToClient",
        "debugSdkLibraries",
        "debugExternalPackageLibraries",
        "noDebug",
    ] {
        if let Some(v) = user_config.get(key) {
            if v.is_boolean() || v.is_null() {
                dap.insert(key.into(), v.clone());
            }
        }
    }

    if let Some(custom) = optional_str(user_config, "customTool") {
        dap.insert("customTool".into(), Value::String(custom.to_string()));
    }
    if let Some(n) = user_config
        .get("customToolReplacesArgs")
        .and_then(|v| v.as_u64())
    {
        dap.insert(
            "customToolReplacesArgs".into(),
            Value::Number(n.into()),
        );
    }

    Value::Object(dap)
}

fn optional_str<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
}

fn string_array(value: &Value, key: &str) -> Option<Vec<String>> {
    value.get(key).and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    })
}

fn flutter_mode(value: &Value) -> Option<String> {
    optional_str(value, "flutterMode")
        .or(optional_str(value, "profile"))
        .map(str::to_string)
}

/// Resolve relative vmServiceInfoFile against launch cwd (worktree-aware paths from Zed tasks).
pub fn resolve_vm_service_info_file(cwd: &str, path: &str) -> String {
    let path = path.trim();
    if path.starts_with('/') || path.starts_with('$') {
        return path.to_string();
    }
    let base = cwd.trim_end_matches('/');
    format!("{base}/{path}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_tool_args_and_profile_mode() {
        let user = json!({
            "type": "flutter",
            "profile": "release",
            "toolArgs": ["--flavor", "prod", "--dart-define", "X=1"],
            "device_id": "linux",
            "platform": "desktop"
        });
        let dap = build_flutter_dap_configuration(
            &user,
            "flutter",
            "launch",
            "lib/main.dart",
            "/project",
            &[],
        );
        assert_eq!(dap["flutterMode"], "release");
        assert_eq!(dap["toolArgs"][0], "--flavor");
        assert_eq!(dap["deviceId"], "linux");
        assert_eq!(dap["platform"], "desktop");
    }

    #[test]
    fn omits_device_when_not_in_user_config() {
        let user = json!({ "type": "flutter" });
        let dap = build_flutter_dap_configuration(
            &user,
            "flutter",
            "launch",
            "lib/main.dart",
            "/project",
            &[],
        );
        assert!(dap.get("deviceId").is_none());
        assert!(dap.get("platform").is_none());
        assert!(dap.get("flutterMode").is_none());
    }

    #[test]
    fn attach_passes_vm_service_fields() {
        let user = json!({
            "request": "attach",
            "vmServiceUri": "ws://127.0.0.1:1/ws",
            "vmServiceInfoFile": ".zed/flutter/vmservice.json"
        });
        let dap = build_flutter_dap_configuration(
            &user,
            "flutter",
            "attach",
            "lib/main.dart",
            "/project/apps/mobile",
            &[],
        );
        assert_eq!(dap["vmServiceUri"], "ws://127.0.0.1:1/ws");
        assert_eq!(
            dap["vmServiceInfoFile"],
            "/project/apps/mobile/.zed/flutter/vmservice.json"
        );
    }

    #[test]
    fn resolve_relative_vmservice_path() {
        assert_eq!(
            resolve_vm_service_info_file("/root", ".zed/flutter/vmservice.json"),
            "/root/.zed/flutter/vmservice.json"
        );
    }
}
