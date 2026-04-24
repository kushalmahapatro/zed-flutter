//! Flutter companion extension for Zed: maps Flutter tasks to the Dart extension's
//! DAP (`flutter debug_adapter` / `dart debug_adapter`), which provides breakpoints,
//! isolate/thread views, and the rest of DAP.

use zed_extension_api::serde_json::{self, json};
use zed_extension_api::{self as zed, TaskTemplate};

const DART_ADAPTER: &str = "Dart";

struct FlutterExtension;

impl FlutterExtension {
    fn dart_flutter_config(
        resolved_label: &str,
        use_fvm: bool,
        program: &str,
        cwd: Option<String>,
        request: &str,
        device_id: Option<&str>,
        platform: Option<&str>,
        vm_service_uri: Option<&str>,
    ) -> Option<String> {
        let mut value = json!({
            "adapter": DART_ADAPTER,
            "type": "flutter",
            "request": request,
            "label": resolved_label,
            "program": program,
            "useFvm": use_fvm,
            "args": [],
        });
        if let Some(c) = &cwd {
            value
                .as_object_mut()?
                .insert("cwd".into(), serde_json::Value::String(c.clone()));
        }
        if let Some(d) = device_id {
            value
                .as_object_mut()?
                .insert("device_id".into(), serde_json::Value::String(d.into()));
        }
        if let Some(p) = platform {
            value
                .as_object_mut()?
                .insert("platform".into(), serde_json::Value::String(p.into()));
        }
        if let Some(uri) = vm_service_uri {
            value
                .as_object_mut()?
                .insert("vmServiceUri".into(), serde_json::Value::String(uri.into()));
        }
        serde_json::to_string(&value).ok()
    }
}

fn args_after_fvm_flutter(args: &[String]) -> Option<&[String]> {
    match args.split_first() {
        Some((head, tail)) if head == "flutter" => Some(tail),
        _ => None,
    }
}

/// Returns `(use_fvm, args after the leading "flutter" token)`.
fn flutter_invocation(task: &TaskTemplate) -> Option<(bool, &[String])> {
    if task.command == "flutter" {
        return Some((false, &task.args));
    }
    if task.command == "fvm" {
        return Some((true, args_after_fvm_flutter(&task.args)?));
    }
    None
}

fn parse_device_id(rest: &[&str]) -> Option<String> {
    let mut i = 0;
    while i < rest.len() {
        match rest[i] {
            "-d" | "--device-id" => {
                return rest.get(i + 1).map(|s| (*s).to_string());
            }
            v if v.starts_with("--device-id=") => {
                return v.strip_prefix("--device-id=").map(str::to_string);
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn parse_target_path(rest: &[&str]) -> Option<String> {
    let mut i = 0;
    while i < rest.len() {
        match rest[i] {
            "-t" | "--target" => {
                return rest.get(i + 1).map(|s| (*s).to_string());
            }
            v if v.starts_with("--target=") => {
                return v.strip_prefix("--target=").map(str::to_string);
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn infer_platform(device: Option<&str>) -> Option<&'static str> {
    let d = device?;
    if matches!(
        d,
        "chrome" | "edge" | "web-server" | "wasm_release" | "wasm_debug"
    ) {
        Some("web")
    } else {
        Some("desktop")
    }
}

fn program_for_run(rest: &[&str]) -> String {
    parse_target_path(rest).unwrap_or_else(|| "lib/main.dart".into())
}

fn program_for_test(rest: &[&str]) -> Option<String> {
    rest.first().map(|p| p.split('?').next().unwrap_or(p).to_string())
}

impl zed::Extension for FlutterExtension {
    fn new() -> Self {
        Self
    }

    fn dap_locator_create_scenario(
        &mut self,
        _locator_name: String,
        build_task: TaskTemplate,
        resolved_label: String,
        debug_adapter_name: String,
    ) -> Option<zed::DebugScenario> {
        if debug_adapter_name != DART_ADAPTER {
            return None;
        }

        let (use_fvm, subargs) = flutter_invocation(&build_task)?;
        let cmd = subargs.first()?.as_str();
        let rest: Vec<&str> = subargs.iter().skip(1).map(String::as_str).collect();

        let cwd = build_task.cwd.clone();
        let device = parse_device_id(&rest);
        let platform = infer_platform(device.as_deref());

        let config = match cmd {
            "run" => {
                let program = program_for_run(&rest);
                Self::dart_flutter_config(
                    &resolved_label,
                    use_fvm,
                    &program,
                    cwd,
                    "launch",
                    device.as_deref(),
                    platform,
                    None,
                )?
            }
            "test" => {
                let program = program_for_test(&rest)?;
                Self::dart_flutter_config(
                    &resolved_label,
                    use_fvm,
                    &program,
                    cwd,
                    "launch",
                    device.as_deref(),
                    platform,
                    None,
                )?
            }
            _ => return None,
        };

        Some(zed::DebugScenario {
            adapter: debug_adapter_name,
            label: resolved_label,
            config,
            tcp_connection: None,
            build: None,
        })
    }
}

zed::register_extension!(FlutterExtension);
