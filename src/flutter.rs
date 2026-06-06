//! Flutter companion extension for Zed: maps Flutter tasks to the Dart extension's
//! DAP (`flutter debug_adapter` / `dart debug_adapter`), which provides breakpoints,
//! isolate/thread views, and the rest of DAP.
//!
//! When a `flutter run` / `flutter test` / `flutter attach` task omits `-d` / `--device-id`,
//! this locator resolves a device by:
//! 1. Reading `.zed/flutter_devices.json` (new format) and `.zed/flutter_device_id` (legacy).
//! 2. Running `flutter devices --machine` in the task directory (so FVM picks the right SDK).
//! 3. Using persisted/fallback ids when still available; otherwise first supported device.
//! 4. Updating `.zed/flutter_devices.json` with `default_device_id` and `last_seen`.

use std::path::Path;

use zed_extension_api::serde_json::{self, json};
use zed_extension_api::{
    self as zed, Command, LanguageServerId, Os, TaskTemplate, Worktree, current_platform,
};

const DART_ADAPTER: &str = "Dart";
const DEVICE_STORE_REL: &str = ".zed/flutter_device_id";
const DEVICE_STORE_JSON_REL: &str = ".zed/flutter_devices.json";

struct FlutterExtension;

#[derive(Clone, Debug, Default)]
struct FlutterLaunchOptions {
    profile: Option<String>,
    tool_args: Vec<String>,
    program_args: Vec<String>,
    vm_service_uri: Option<String>,
}

impl FlutterExtension {
    fn dart_flutter_config(
        resolved_label: &str,
        use_fvm: bool,
        program: &str,
        cwd: Option<String>,
        request: &str,
        device_id: Option<&str>,
        platform: Option<&str>,
        launch: &FlutterLaunchOptions,
    ) -> Option<String> {
        let mut value = json!({
            "adapter": DART_ADAPTER,
            "type": "flutter",
            "request": request,
            "label": resolved_label,
            "program": program,
            "useFvm": use_fvm,
            "args": launch.program_args,
        });
        let obj = value.as_object_mut()?;
        if let Some(c) = &cwd {
            obj.insert("cwd".into(), serde_json::Value::String(c.clone()));
        }
        if let Some(d) = device_id {
            obj.insert("device_id".into(), serde_json::Value::String(d.into()));
        }
        if let Some(p) = platform {
            obj.insert("platform".into(), serde_json::Value::String(p.into()));
        }
        if let Some(profile) = &launch.profile {
            obj.insert("profile".into(), serde_json::Value::String(profile.clone()));
            obj.insert(
                "flutterMode".into(),
                serde_json::Value::String(profile.clone()),
            );
        }
        if let Some(uri) = &launch.vm_service_uri {
            obj.insert("vmServiceUri".into(), serde_json::Value::String(uri.clone()));
        }
        if !launch.tool_args.is_empty() {
            obj.insert(
                "toolArgs".into(),
                serde_json::Value::Array(
                    launch
                        .tool_args
                        .iter()
                        .map(|a| serde_json::Value::String(a.clone()))
                        .collect(),
                ),
            );
        }
        // Lets the Dart/Flutter DAP write the VM Service URI for DevTools / hot reload scripts.
        if request == "launch" {
            obj.insert(
                "vmServiceInfoFile".into(),
                serde_json::Value::String(".zed/flutter/vmservice.json".into()),
            );
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

fn split_at_double_dash<'a>(rest: &'a [&'a str]) -> (&'a [&'a str], &'a [&'a str]) {
    if let Some(pos) = rest.iter().position(|a| *a == "--") {
        let (before, after) = rest.split_at(pos);
        (before, &after[1..])
    } else {
        (rest, &[])
    }
}

fn parse_flag_value(rest: &[&str], short: &str, long: &str) -> Option<String> {
    let mut i = 0;
    while i < rest.len() {
        match rest[i] {
            v if v == short || v == long => return rest.get(i + 1).map(|s| (*s).to_string()),
            v if let Some(val) = v.strip_prefix(&format!("{long}=")) => {
                return Some(val.to_string());
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn parse_device_id(rest: &[&str]) -> Option<String> {
    parse_flag_value(rest, "-d", "--device-id")
}

fn parse_target_path(rest: &[&str]) -> Option<String> {
    parse_flag_value(rest, "-t", "--target")
}

fn parse_vm_service_uri(rest: &[&str]) -> Option<String> {
    parse_flag_value(rest, "", "--debug-uri").or_else(|| parse_flag_value(rest, "", "--host-vmservice-port").map(|p| format!("http://127.0.0.1:{p}/")))
}

/// Maps `flutter run` profile flags to DAP `profile` / `flutterMode`.
fn parse_profile_mode(rest: &[&str]) -> Option<&'static str> {
    for arg in rest {
        match *arg {
            "--release" => return Some("release"),
            "--profile" => return Some("profile"),
            "--debug" => return Some("debug"),
            _ => {}
        }
    }
    None
}

fn is_handled_run_flag(arg: &str) -> bool {
    matches!(
        arg,
        "-d" | "--device-id" | "-t" | "--target" | "--debug" | "--profile" | "--release"
    ) || arg.starts_with("--device-id=")
        || arg.starts_with("--target=")
        || arg.starts_with("--debug-uri=")
        || arg.starts_with("--host-vmservice-port=")
}

/// Collects flutter-tool flags (flavor, dart-define, web options, etc.) for `toolArgs`.
fn collect_tool_args(rest: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < rest.len() {
        let arg = rest[i];
        if arg == "--" {
            break;
        }
        if is_handled_run_flag(arg) {
            if matches!(arg, "-d" | "--device-id" | "-t" | "--target" | "--debug-uri" | "--host-vmservice-port") {
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if arg.starts_with('-') {
            let takes_value = matches!(
                arg,
                "--flavor"
                    | "--dart-define"
                    | "--dart-define-from-file"
                    | "--web-port"
                    | "--web-hostname"
                    | "--web-tls-cert-path"
                    | "--web-tls-cert-key-path"
                    | "--device-user"
                    | "--build-number"
                    | "--build-name"
                    | "--split-debug-info"
                    | "--enable-experiment"
                    | "--dart-entrypoint-args"
            ) || arg.starts_with("--dart-define=")
                || arg.starts_with("--dart-define-from-file=")
                || arg.starts_with("--flavor=");
            out.push(arg.to_string());
            if takes_value && !arg.contains('=') {
                if let Some(v) = rest.get(i + 1) {
                    out.push((*v).to_string());
                    i += 1;
                }
            }
            i += 1;
            continue;
        }
        // Positional entrypoint is handled separately.
        i += 1;
    }
    out
}

fn launch_options_from_rest(rest: &[&str]) -> FlutterLaunchOptions {
    let (tool_part, program_part) = split_at_double_dash(rest);
    FlutterLaunchOptions {
        profile: parse_profile_mode(tool_part).map(str::to_string),
        tool_args: collect_tool_args(tool_part),
        program_args: program_part.iter().map(|s| (*s).to_string()).collect(),
        vm_service_uri: parse_vm_service_uri(tool_part),
    }
}

fn infer_platform(device: Option<&str>) -> Option<&'static str> {
    let d = device?;
    if matches!(
        d,
        "chrome" | "edge" | "web-server" | "wasm_release" | "wasm_debug"
    ) {
        Some("web")
    } else if matches!(d, "linux" | "windows" | "macos") {
        Some("desktop")
    } else {
        None
    }
}

fn infer_platform_from_target_platform(target_platform: &str) -> Option<&'static str> {
    let tp = target_platform.to_ascii_lowercase();
    if tp.contains("web") {
        Some("web")
    } else if tp.contains("linux")
        || tp.contains("windows")
        || tp.contains("darwin")
        || tp.contains("macos")
    {
        Some("desktop")
    } else {
        None
    }
}

fn platform_for_device(device_id: Option<&str>, devices: &[DeviceInfo]) -> Option<&'static str> {
    if let Some(id) = device_id {
        if let Some(device) = devices.iter().find(|d| d.id == id) {
            if let Some(tp) = device.target_platform.as_deref() {
                if let Some(p) = infer_platform_from_target_platform(tp) {
                    return Some(p);
                }
            }
        }
        return infer_platform(Some(id));
    }
    None
}

fn program_for_run(rest: &[&str]) -> String {
    if let Some(target) = parse_target_path(rest) {
        return target;
    }
    let (tool_part, _) = split_at_double_dash(rest);
    for arg in tool_part {
        if !arg.starts_with('-') {
            return (*arg).to_string();
        }
    }
    "lib/main.dart".into()
}

fn program_for_test(rest: &[&str]) -> Option<String> {
    rest.iter()
        .find(|a| !a.starts_with('-'))
        .map(|p| p.split('?').next().unwrap_or(p).to_string())
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

fn read_text_file_with_shell(cwd: &str, rel_path: &str) -> Option<String> {
    let output = match current_platform().0 {
        Os::Windows => {
            let root = ps_single_quote_escape(cwd);
            let rel = ps_single_quote_escape(rel_path);
            Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    &format!(
                        "Set-Location -LiteralPath '{root}'; if (Test-Path -LiteralPath '{rel}') {{ Get-Content -LiteralPath '{rel}' -Raw }}",
                    ),
                ])
                .output()
                .ok()?
        }
        Os::Mac | Os::Linux => {
            let cwd_q = shell_single_quote(cwd);
            let rel_q = shell_single_quote(rel_path);
            Command::new("sh")
                .args([
                    "-c",
                    &format!("cd {cwd_q} 2>/dev/null && cat {rel_q} 2>/dev/null || true"),
                ])
                .output()
                .ok()?
        }
    };
    let txt = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if txt.is_empty() {
        None
    } else {
        Some(txt)
    }
}

fn json_platform_key() -> &'static str {
    match current_platform().0 {
        Os::Mac => "mac",
        Os::Linux => "linux",
        Os::Windows => "windows",
    }
}

fn read_persisted_device_candidates(cwd: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(txt) = read_text_file_with_shell(cwd, DEVICE_STORE_JSON_REL) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
            if let Some(default_id) = v.get("default_device_id").and_then(|x| x.as_str()) {
                if !default_id.is_empty() {
                    out.push(default_id.to_string());
                }
            }
            if let Some(fallbacks) = v.get("fallback_device_ids").and_then(|x| x.as_object()) {
                let key = json_platform_key();
                if let Some(list) = fallbacks.get(key).and_then(|x| x.as_array()) {
                    for id in list.iter().filter_map(|x| x.as_str()) {
                        if !id.is_empty() {
                            out.push(id.to_string());
                        }
                    }
                }
                if let Some(list) = fallbacks.get("all").and_then(|x| x.as_array()) {
                    for id in list.iter().filter_map(|x| x.as_str()) {
                        if !id.is_empty() {
                            out.push(id.to_string());
                        }
                    }
                }
            }
        }
    }
    if let Some(legacy) = read_persisted_device_line(cwd) {
        out.push(legacy);
    }
    out
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

#[derive(Clone, Debug)]
struct DeviceInfo {
    id: String,
    name: Option<String>,
    is_supported: bool,
    target_platform: Option<String>,
    emulator: Option<bool>,
    sdk: Option<String>,
}

fn parse_flutter_devices_machine(stdout: &[u8]) -> Vec<DeviceInfo> {
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
        out.push(DeviceInfo {
            id: id.to_string(),
            name: row
                .get("name")
                .and_then(|x| x.as_str())
                .map(str::to_string),
            is_supported: supported,
            target_platform: row
                .get("targetPlatform")
                .and_then(|x| x.as_str())
                .map(str::to_string),
            emulator: row.get("emulator").and_then(|x| x.as_bool()),
            sdk: row.get("sdk").and_then(|x| x.as_str()).map(str::to_string),
        });
    }
    out
}

fn write_device_store_json(cwd: &str, selected_id: &str, devices: &[DeviceInfo]) {
    let mut last_seen = serde_json::Map::new();
    for d in devices {
        last_seen.insert(
            d.id.clone(),
            json!({
                "name": d.name,
                "is_supported": d.is_supported,
                "target_platform": d.target_platform,
                "emulator": d.emulator,
                "sdk": d.sdk,
            }),
        );
    }

    let mut obj = serde_json::Map::new();
    obj.insert(
        "default_device_id".to_string(),
        serde_json::Value::String(selected_id.to_string()),
    );
    obj.insert("last_seen".to_string(), serde_json::Value::Object(last_seen));
    let txt = match serde_json::to_string_pretty(&serde_json::Value::Object(obj)) {
        Ok(v) => v,
        Err(_) => return,
    };

    match current_platform().0 {
        Os::Windows => {
            let root = ps_single_quote_escape(cwd);
            let rel = ps_single_quote_escape(DEVICE_STORE_JSON_REL);
            let script = format!(
                "Set-Location -LiteralPath '{root}'; New-Item -ItemType Directory -Force -Path '.zed' | Out-Null; [IO.File]::WriteAllText('{rel}', @'\n{txt}\n'@)"
            );
            let _ = Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                .output();
        }
        Os::Mac | Os::Linux => {
            let cwd_q = shell_single_quote(cwd);
            let cmd = format!(
                "cd {cwd_q} && mkdir -p .zed && cat > {DEVICE_STORE_JSON_REL} <<'EOF'\n{txt}\nEOF"
            );
            let _ = Command::new("sh").args(["-c", &cmd]).output();
        }
    }
}

struct ResolvedDevice {
    id: Option<String>,
    devices: Vec<DeviceInfo>,
}

/// Resolves `device_id` when the task did not pass `-d` / `--device-id`.
fn resolve_device_for_debug(
    from_task_args: Option<String>,
    cwd: Option<&str>,
    use_fvm: bool,
) -> ResolvedDevice {
    if let Some(id) = from_task_args {
        return ResolvedDevice {
            id: Some(id),
            devices: Vec::new(),
        };
    }
    let Some(cwd) = cwd else {
        return ResolvedDevice {
            id: None,
            devices: Vec::new(),
        };
    };
    let persisted_candidates = read_persisted_device_candidates(cwd);
    let machine_stdout = match run_flutter_devices_machine_stdout(cwd, use_fvm) {
        Some(s) => s,
        None => {
            return ResolvedDevice {
                id: persisted_candidates.into_iter().next(),
                devices: Vec::new(),
            };
        }
    };
    let devices = parse_flutter_devices_machine(&machine_stdout);

    if devices.is_empty() {
        return ResolvedDevice {
            id: persisted_candidates.into_iter().next(),
            devices,
        };
    }

    let ids: Vec<&str> = devices.iter().map(|d| d.id.as_str()).collect();
    for candidate in persisted_candidates {
        if ids.iter().any(|d| *d == candidate) {
            write_device_store_json(cwd, &candidate, &devices);
            return ResolvedDevice {
                id: Some(candidate),
                devices,
            };
        }
    }

    let selected = devices
        .iter()
        .find(|d| d.is_supported)
        .map(|d| d.id.clone())
        .or_else(|| devices.first().map(|d| d.id.clone()));
    if let Some(ref id) = selected {
        write_device_store_json(cwd, id, &devices);
    }
    ResolvedDevice {
        id: selected,
        devices,
    }
}

fn fvm_flutter_sdk_path(worktree_root: &str) -> Option<String> {
    let sdk_link = read_text_file_with_shell(worktree_root, ".fvm/fvm_config.json")?;
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&sdk_link) {
        if let Some(version) = v.get("flutterSdkVersion").and_then(|x| x.as_str()) {
            let cache = read_text_file_with_shell(worktree_root, ".fvm/flutter_sdk");
            if cache.is_some() {
                let root_q = shell_single_quote(worktree_root);
                let output = match current_platform().0 {
                    Os::Windows => {
                        let root = ps_single_quote_escape(worktree_root);
                        Command::new("powershell")
                            .args([
                                "-NoProfile",
                                "-NonInteractive",
                                "-Command",
                                &format!(
                                    "Set-Location -LiteralPath '{root}'; if (Test-Path '.fvm/flutter_sdk') {{ Resolve-Path '.fvm/flutter_sdk' | Select-Object -ExpandProperty Path }}"
                                ),
                            ])
                            .output()
                            .ok()?
                    }
                    Os::Mac | Os::Linux => Command::new("sh")
                        .args([
                            "-c",
                            &format!("cd {root_q} && readlink -f .fvm/flutter_sdk 2>/dev/null || realpath .fvm/flutter_sdk 2>/dev/null || echo .fvm/flutter_sdk"),
                        ])
                        .output()
                        .ok()?,
                };
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
            let _ = version; // version present confirms FVM project
        }
    }
    None
}

impl zed::Extension for FlutterExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_additional_workspace_configuration(
        &mut self,
        _language_server_id: &LanguageServerId,
        target_language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> zed::Result<Option<zed_extension_api::serde_json::Value>> {
        if target_language_server_id.as_ref() != "dart" {
            return Ok(None);
        }
        let root = worktree.root_path();
        let Some(flutter_sdk) = fvm_flutter_sdk_path(&root) else {
            return Ok(None);
        };
        Ok(Some(json!({
            "dart": {
                "flutterSdkPath": flutter_sdk
            }
        })))
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
        let resolved = resolve_device_for_debug(from_args, cwd.as_deref(), use_fvm);
        let device = resolved.id;
        let platform = platform_for_device(device.as_deref(), &resolved.devices);
        let launch = launch_options_from_rest(&rest);

        let (request, program, launch) = match cmd {
            "run" => {
                let program = program_for_run(&rest);
                ("launch", program, launch)
            }
            "test" => {
                let program = program_for_test(&rest)?;
                ("launch", program, launch)
            }
            "attach" => {
                let program = parse_target_path(&rest).unwrap_or_else(|| "lib/main.dart".into());
                ("attach", program, launch)
            }
            _ => return None,
        };

        let config = Self::dart_flutter_config(
            &resolved_label,
            use_fvm,
            &program,
            cwd,
            request,
            device.as_deref(),
            platform,
            &launch,
        )?;

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
    fn parse_profile_flags() {
        let rest = vec!["--profile", "-t", "lib/main.dart"];
        let r: Vec<&str> = rest.iter().copied().collect();
        assert_eq!(parse_profile_mode(&r), Some("profile"));

        let release = vec!["--release"];
        assert_eq!(parse_profile_mode(&release), Some("release"));
    }

    #[test]
    fn collect_flavor_and_dart_define() {
        let rest = vec![
            "--flavor",
            "dev",
            "--dart-define",
            "API_URL=https://dev.example",
            "-t",
            "lib/main_dev.dart",
        ];
        let r: Vec<&str> = rest.iter().copied().collect();
        let tool_args = collect_tool_args(&r);
        assert!(tool_args.contains(&"--flavor".to_string()));
        assert!(tool_args.contains(&"dev".to_string()));
        assert!(tool_args.contains(&"--dart-define".to_string()));
        assert!(tool_args.contains(&"API_URL=https://dev.example".to_string()));
    }

    #[test]
    fn program_args_after_double_dash() {
        let rest = vec!["--debug", "lib/main.dart", "--", "--verbose"];
        let r: Vec<&str> = rest.iter().copied().collect();
        let opts = launch_options_from_rest(&r);
        assert_eq!(opts.profile.as_deref(), Some("debug"));
        assert_eq!(opts.program_args, vec!["--verbose".to_string()]);
    }

    #[test]
    fn parse_devices_machine_json() {
        let json = br#"[
          {"name":"Linux","id":"linux","isSupported":true,"targetPlatform":"linux-x64"}
        ]"#;
        let d = parse_flutter_devices_machine(json);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].id, "linux");
        assert!(d[0].is_supported);
        assert_eq!(d[0].target_platform.as_deref(), Some("linux-x64"));
    }

    #[test]
    fn infer_platform_from_target_platform_values() {
        assert_eq!(
            infer_platform_from_target_platform("linux-x64"),
            Some("desktop")
        );
        assert_eq!(
            infer_platform_from_target_platform("web-javascript"),
            Some("web")
        );
        assert_eq!(infer_platform_from_target_platform("android-arm64"), None);
    }

    #[test]
    fn platform_for_device_uses_machine_metadata() {
        let devices = vec![DeviceInfo {
            id: "linux".into(),
            name: Some("Linux".into()),
            is_supported: true,
            target_platform: Some("linux-x64".into()),
            emulator: Some(false),
            sdk: None,
        }];
        assert_eq!(platform_for_device(Some("linux"), &devices), Some("desktop"));
    }

    #[test]
    fn infer_platform_web() {
        assert_eq!(infer_platform(Some("chrome")), Some("web"));
        assert_eq!(infer_platform(Some("linux")), Some("desktop"));
    }

    #[test]
    fn parse_attach_debug_uri() {
        let rest = vec!["--debug-uri", "ws://127.0.0.1:12345/ws"];
        let r: Vec<&str> = rest.iter().copied().collect();
        let opts = launch_options_from_rest(&r);
        assert_eq!(
            opts.vm_service_uri.as_deref(),
            Some("ws://127.0.0.1:12345/ws")
        );
    }

    #[test]
    fn launch_config_includes_vmservice_info_file() {
        let launch = FlutterLaunchOptions::default();
        let cfg = FlutterExtension::dart_flutter_config(
            "test",
            false,
            "lib/main.dart",
            Some("$ZED_WORKTREE_ROOT".into()),
            "launch",
            None,
            None,
            &launch,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&cfg).unwrap();
        assert_eq!(
            v.get("vmServiceInfoFile").and_then(|x| x.as_str()),
            Some(".zed/flutter/vmservice.json")
        );
    }

    #[test]
    fn parse_persisted_json_candidates_order() {
        let v = serde_json::json!({
            "default_device_id": "android",
            "fallback_device_ids": {
                "linux": ["linux", "chrome"],
                "all": ["edge"]
            }
        });
        let txt = serde_json::to_string(&v).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&txt).unwrap();
        let mut out = Vec::new();
        if let Some(default_id) = parsed.get("default_device_id").and_then(|x| x.as_str()) {
            out.push(default_id.to_string());
        }
        if let Some(fallbacks) = parsed.get("fallback_device_ids").and_then(|x| x.as_object()) {
            if let Some(list) = fallbacks.get("linux").and_then(|x| x.as_array()) {
                out.extend(list.iter().filter_map(|x| x.as_str()).map(str::to_string));
            }
        }
        assert_eq!(out, vec!["android", "linux", "chrome"]);
    }
}
