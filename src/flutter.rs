//! Flutter companion extension for Zed: maps Flutter tasks to the Dart extension's
//! DAP (`flutter debug_adapter` / `dart debug_adapter`), which provides breakpoints,
//! isolate/thread views, and the rest of DAP.
//!
//! When a `flutter run` / `flutter test` task omits `-d` / `--device-id`, this locator
//! resolves a device by:
//! 1. Reading `.zed/flutter_device_id` in the task working directory (first line = device id).
//! 2. Running `flutter devices --machine` in that directory (so FVM picks the right SDK).
//! 3. Using the persisted id if it still appears in the machine list; otherwise the first
//!    supported device from that list.

use std::path::Path;

use zed_extension_api::serde_json::{self, json};
use zed_extension_api::{self as zed, Command, Os, TaskTemplate, current_platform};

const DART_ADAPTER: &str = "Dart";
const DEVICE_STORE_REL: &str = ".zed/flutter_device_id";

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

/// POSIX single-quoted string literal content for `sh -c`.
fn shell_single_quote(path: &str) -> String {
    let mut out = String::from("'");
    for ch in path.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

/// Escape for PowerShell single-quoted string.
fn ps_single_quote_escape(path: &str) -> String {
    path.replace('\'', "''")
}

fn read_persisted_device_line(cwd: &str) -> Option<String> {
    let store_path = Path::new(cwd).join(DEVICE_STORE_REL);
    let p = store_path.to_string_lossy();
    let p_esc = ps_single_quote_escape(&p);

    let output = match current_platform().0 {
        Os::Windows => Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!(
                    "if (Test-Path -LiteralPath '{p_esc}') {{ (Get-Content -LiteralPath '{p_esc}' -TotalCount 1).Trim() }}",
                ),
            ])
            .output(),
        Os::Mac | Os::Linux => {
            let cwd_q = shell_single_quote(cwd);
            Command::new("sh").arg("-c").arg(format!(
                "cd {cwd_q} 2>/dev/null && head -n 1 {DEVICE_STORE_REL} 2>/dev/null || true"
            )).output()
        }
    }
    .ok()?;

    let line = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if line.is_empty() {
        None
    } else {
        Some(line)
    }
}

fn run_flutter_devices_machine_stdout(cwd: &str, use_fvm: bool) -> Option<Vec<u8>> {
    let stdout = match current_platform().0 {
        Os::Windows => {
            let loc = ps_single_quote_escape(cwd);
            let cmdline = if use_fvm {
                format!("Set-Location -LiteralPath '{loc}'; fvm flutter devices --machine")
            } else {
                format!("Set-Location -LiteralPath '{loc}'; flutter devices --machine")
            };
            Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &cmdline])
                .output()
                .ok()?
                .stdout
        }
        Os::Mac | Os::Linux => {
            let cwd_q = shell_single_quote(cwd);
            let inner = if use_fvm {
                format!("cd {cwd_q} && fvm flutter devices --machine")
            } else {
                format!("cd {cwd_q} && flutter devices --machine")
            };
            Command::new("sh")
                .args(["-c", &inner])
                .output()
                .ok()?
                .stdout
        }
    };
    Some(stdout)
}

fn parse_flutter_devices_machine(stdout: &[u8]) -> Vec<(String, bool)> {
    let s = String::from_utf8_lossy(stdout);
    let Ok(v) = serde_json::from_str::<serde_json::Value>(s.trim()) else {
        return Vec::new();
    };
    let Some(arr) = v.as_array() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for row in arr {
        let Some(id) = row.get("id").and_then(|x| x.as_str()) else {
            continue;
        };
        let supported = row
            .get("isSupported")
            .and_then(|x| x.as_bool())
            .unwrap_or(true);
        out.push((id.to_string(), supported));
    }
    out
}

/// Resolves `device_id` when the task did not pass `-d` / `--device-id`.
fn resolve_device_for_debug(
    from_task_args: Option<String>,
    cwd: Option<&str>,
    use_fvm: bool,
) -> Option<String> {
    if from_task_args.is_some() {
        return from_task_args;
    }
    let cwd = cwd?;
    let persisted = read_persisted_device_line(cwd);
    let machine_stdout = run_flutter_devices_machine_stdout(cwd, use_fvm)?;
    let devices = parse_flutter_devices_machine(&machine_stdout);

    if devices.is_empty() {
        return persisted;
    }

    let ids: Vec<&str> = devices.iter().map(|(id, _)| id.as_str()).collect();
    if let Some(ref id) = persisted {
        if ids.iter().any(|d| *d == id) {
            return persisted;
        }
    }

    devices
        .iter()
        .find(|(_, sup)| *sup)
        .map(|(id, _)| id.clone())
        .or_else(|| devices.first().map(|(id, _)| id.clone()))
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
        let from_args = parse_device_id(&rest);
        let device = resolve_device_for_debug(from_args, cwd.as_deref(), use_fvm);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_device_id_flags() {
        let v = vec!["run", "-d", "emu", "lib/main.dart"];
        let r: Vec<&str> = v.iter().copied().collect();
        assert_eq!(parse_device_id(&r[1..]), Some("emu".into()));

        let v2 = vec!["run", "--device-id=chrome"];
        let r2: Vec<&str> = v2.iter().copied().collect();
        assert_eq!(parse_device_id(&r2[1..]), Some("chrome".into()));
    }

    #[test]
    fn parse_devices_machine_json() {
        let json = br#"[
          {"name":"Linux","id":"linux","isSupported":true}
        ]"#;
        let d = parse_flutter_devices_machine(json);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].0, "linux");
        assert!(d[0].1);
    }

    #[test]
    fn infer_platform_web() {
        assert_eq!(infer_platform(Some("chrome")), Some("web"));
        assert_eq!(infer_platform(Some("linux")), Some("desktop"));
    }
}
