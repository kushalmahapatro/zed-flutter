// Proposed replacement for `get_dap_binary` in zed-extensions/dart/src/dart.rs
//
// 1. Add to Cargo.toml: serde_json is already a dependency via zed_extension_api
// 2. Add mod dap_config; and copy src/lib.rs from this folder as src/dap_config.rs
//    OR inline build_flutter_dap_configuration from upstream/zed-extensions-dart/src/lib.rs
//
// See UPSTREAM_PR.md for the full PR checklist.

use zed_extension_api::serde_json::json;
use zed_extension_api::{
    self as zed, current_platform, serde_json, DebugAdapterBinary, DebugTaskDefinition, Os, Result,
    StartDebuggingRequestArguments, StartDebuggingRequestArgumentsRequest, Worktree,
};

// mod dap_config;
// use dap_config::build_flutter_dap_configuration;

fn get_dap_binary(
    &mut self,
    _adapter_name: String,
    config: DebugTaskDefinition,
    _user_provided_debug_adapter_path: Option<String>,
    worktree: &Worktree,
) -> Result<DebugAdapterBinary> {
    let user_config: serde_json::Value = serde_json::from_str(&config.config)
        .map_err(|e| format!("Failed to parse debug config: {e}"))?;

    let program = user_config
        .get("program")
        .and_then(|v| v.as_str())
        .unwrap_or("lib/main.dart");

    let args: Vec<String> = user_config
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    let use_fvm = user_config
        .get("useFvm")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let debug_mode = user_config
        .get("type")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "type is required and cannot be empty or null".to_string())?;

    let (os, _) = current_platform();
    let tool = if debug_mode == "flutter" {
        match os {
            Os::Windows => "flutter.bat",
            _ => "flutter",
        }
    } else {
        match os {
            Os::Windows => "dart.bat",
            _ => "dart",
        }
    };

    let (command, arguments) = if use_fvm {
        (
            "fvm".to_string(),
            vec![tool.to_string(), "debug_adapter".to_string()],
        )
    } else {
        (tool.to_string(), vec!["debug_adapter".to_string()])
    };

    let cwd = user_config
        .get("cwd")
        .and_then(|v| v.as_str())
        .map(|s| s.replace("$ZED_WORKTREE_ROOT", &worktree.root_path()))
        .unwrap_or_else(|| worktree.root_path());

    let request = user_config
        .get("request")
        .and_then(|v| v.as_str())
        .unwrap_or("launch");

    // BEFORE (drops toolArgs, profile, vmServiceInfoFile; forces chrome/web/debug):
    //   let device_id = user_config.get("device_id")...unwrap_or("chrome");
    //   let platform = ...unwrap_or("web");
    //   let config_json = json!({ ..., "flutterMode": "debug", "deviceId": device_id, ... });
    //
    // AFTER: pass through Flutter DAP fields — see zed_dart_dap_proposal::build_flutter_dap_configuration
    let dap_value = zed_dart_dap_proposal::build_flutter_dap_configuration(
        &user_config,
        tool,
        request,
        program,
        &cwd,
        &args,
    );
    let config_json = dap_value.to_string();

    Ok(DebugAdapterBinary {
        command: Some(command),
        arguments,
        envs: vec![],
        cwd: Some(cwd),
        connection: None,
        request_args: StartDebuggingRequestArguments {
            configuration: config_json,
            request: match request {
                "attach" => StartDebuggingRequestArgumentsRequest::Attach,
                _ => StartDebuggingRequestArgumentsRequest::Launch,
            },
        },
    })
}
