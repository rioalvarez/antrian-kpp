#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

// Embedded at compile time — no external file needed at runtime
const PRINTER_SELECTOR_JS: &str = include_str!("printer-selector.js");

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Deserialize, Serialize, Clone)]
struct AppConfig {
    #[serde(default = "default_server_url", rename = "serverUrl")]
    server_url: String,
    #[serde(default = "default_ticket_path", rename = "ticketPath")]
    ticket_path: String,
    #[serde(default = "default_agent_id", rename = "agentId")]
    agent_id: String,
    #[serde(default = "default_printer_name", rename = "printerName")]
    printer_name: String,
    #[serde(default = "default_retry_delay", rename = "retryDelay")]
    retry_delay: u32,
    #[serde(default = "default_paper_size", rename = "paperSize")]
    paper_size: String,
    #[serde(default = "default_feed_lines", rename = "feedLines")]
    feed_lines: u32,
}

fn default_server_url()  -> String { "http://localhost:8080".to_string() }
fn default_ticket_path() -> String { "/ticket".to_string() }
fn default_agent_id()    -> String { "printer-lobi".to_string() }
fn default_printer_name()-> String { String::new() }
fn default_retry_delay() -> u32    { 5 }
fn default_paper_size()  -> String { "80mm".to_string() }
fn default_feed_lines()  -> u32    { 1 }

// Holds config + the path we should write saves to
struct ConfigState {
    config:      Mutex<AppConfig>,
    config_path: Mutex<Option<PathBuf>>,
}

struct SidecarChild(Mutex<Option<Child>>);

/// Shared mutable server prefix for the navigation guard.
struct ServerPrefixState(Arc<Mutex<String>>);

// ── Tauri commands ────────────────────────────────────────────────────────────

/// List all printers installed on this machine via WMI.
#[tauri::command]
fn list_printers() -> Vec<String> {
    let mut cmd = Command::new("powershell");
    cmd.args([
        "-NoProfile", "-NonInteractive", "-Command",
        "@((Get-WmiObject Win32_Printer).Name) | ConvertTo-Json -Compress",
    ])
    .stdout(Stdio::piped())
    .stderr(Stdio::null());
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    let output = match cmd.output() {
        Ok(o) => o,
        Err(_) => return vec![],
    };

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() || stdout == "null" {
        return vec![];
    }

    // Multiple printers → JSON array
    if let Ok(printers) = serde_json::from_str::<Vec<String>>(&stdout) {
        return printers;
    }
    // Single printer → JSON string
    if let Ok(name) = serde_json::from_str::<String>(&stdout) {
        return vec![name];
    }

    vec![]
}

/// Return the currently configured printer name.
#[tauri::command]
fn get_printer_config(state: tauri::State<'_, ConfigState>) -> String {
    state.config.lock()
        .map(|c| c.printer_name.clone())
        .unwrap_or_default()
}

/// Return the currently configured server URL.
#[tauri::command]
fn get_server_url(state: tauri::State<'_, ConfigState>) -> String {
    state.config.lock()
        .map(|c| c.server_url.clone())
        .unwrap_or_else(|_| default_server_url())
}

/// Save a new printer selection: persists to config.json and restarts the print agent.
#[tauri::command]
fn save_printer(
    config_state:  tauri::State<'_, ConfigState>,
    sidecar_state: tauri::State<'_, SidecarChild>,
    printer_name:  String,
) -> Result<(), String> {
    // 1. Update in-memory config
    {
        let mut cfg = config_state.config.lock().map_err(|e| e.to_string())?;
        cfg.printer_name = printer_name.clone();
    }

    // 2. Persist: read existing config.json → update printerName → write back
    {
        let path_guard = config_state.config_path.lock().map_err(|e| e.to_string())?;
        let config_path = match path_guard.as_ref() {
            Some(p) => p.clone(),
            None => {
                let exe = std::env::current_exe().map_err(|e| e.to_string())?;
                exe.parent()
                    .ok_or("Cannot determine exe directory")?
                    .join("config.json")
            }
        };

        let mut json: serde_json::Value =
            if let Ok(content) = fs::read_to_string(&config_path) {
                serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
            } else {
                serde_json::json!({})
            };

        if let Some(obj) = json.as_object_mut() {
            obj.insert("printerName".to_string(), serde_json::Value::String(printer_name));
        }

        let content = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
        fs::write(&config_path, content).map_err(|e| e.to_string())?;
    }

    // 3. Kill existing print-agent
    if let Ok(mut guard) = sidecar_state.0.lock() {
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    // 4. Restart print-agent with updated config
    let cfg = config_state.config.lock().map_err(|e| e.to_string())?.clone();
    if let Some(sidecar) = find_sidecar() {
        if let Ok(yaml) = write_agent_yaml(&cfg) {
            let mut cmd = Command::new(&sidecar);
            cmd.args(["-config", &yaml])
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            #[cfg(windows)]
            cmd.creation_flags(0x08000000);
            if let Ok(child) = cmd.spawn() {
                if let Ok(mut guard) = sidecar_state.0.lock() {
                    *guard = Some(child);
                }
            }
        }
    }

    Ok(())
}

/// Return paper size and feed lines from config.
#[tauri::command]
fn get_paper_config(state: tauri::State<'_, ConfigState>) -> (String, u32) {
    state.config.lock()
        .map(|c| (c.paper_size.clone(), c.feed_lines))
        .unwrap_or_else(|_| (default_paper_size(), default_feed_lines()))
}

/// Save paper size + feed lines, persist to config.json, restart print agent.
#[tauri::command]
fn save_paper_config(
    config_state:  tauri::State<'_, ConfigState>,
    sidecar_state: tauri::State<'_, SidecarChild>,
    paper_size:    String,
    feed_lines:    u32,
) -> Result<(), String> {
    // 1. Update in-memory config
    {
        let mut cfg = config_state.config.lock().map_err(|e| e.to_string())?;
        cfg.paper_size = paper_size.clone();
        cfg.feed_lines = feed_lines;
    }

    // 2. Persist to config.json
    {
        let path_guard = config_state.config_path.lock().map_err(|e| e.to_string())?;
        let config_path = match path_guard.as_ref() {
            Some(p) => p.clone(),
            None => {
                let exe = std::env::current_exe().map_err(|e| e.to_string())?;
                exe.parent()
                    .ok_or("Cannot determine exe directory")?
                    .join("config.json")
            }
        };

        let mut json: serde_json::Value =
            if let Ok(content) = fs::read_to_string(&config_path) {
                serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
            } else {
                serde_json::json!({})
            };

        if let Some(obj) = json.as_object_mut() {
            obj.insert("paperSize".to_string(), serde_json::Value::String(paper_size));
            obj.insert("feedLines".to_string(), serde_json::Value::Number(serde_json::Number::from(feed_lines)));
        }

        let content = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
        fs::write(&config_path, content).map_err(|e| e.to_string())?;
    }

    // 3. Kill existing print-agent
    if let Ok(mut guard) = sidecar_state.0.lock() {
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    // 4. Restart print-agent with updated config
    let cfg = config_state.config.lock().map_err(|e| e.to_string())?.clone();
    if let Some(sidecar) = find_sidecar() {
        if let Ok(yaml) = write_agent_yaml(&cfg) {
            let mut cmd = Command::new(&sidecar);
            cmd.args(["-config", &yaml])
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            #[cfg(windows)]
            cmd.creation_flags(0x08000000);
            if let Ok(child) = cmd.spawn() {
                if let Ok(mut guard) = sidecar_state.0.lock() {
                    *guard = Some(child);
                }
            }
        }
    }

    Ok(())
}

/// Save a new server URL: persist to config.json, update nav guard, restart print-agent,
/// then navigate the webview to the new server.
#[tauri::command]
fn save_server_url(
    app:           tauri::AppHandle,
    config_state:  tauri::State<'_, ConfigState>,
    prefix_state:  tauri::State<'_, ServerPrefixState>,
    sidecar_state: tauri::State<'_, SidecarChild>,
    server_url:    String,
) -> Result<(), String> {
    let url = server_url.trim_end_matches('/').to_string();

    // 1. Update in-memory config, capture ticket_path
    let ticket_path = {
        let mut cfg = config_state.config.lock().map_err(|e| e.to_string())?;
        cfg.server_url = url.clone();
        cfg.ticket_path.clone()
    };

    // 2. Update navigation prefix so the guard allows the new origin
    {
        let mut prefix = prefix_state.0.lock().map_err(|e| e.to_string())?;
        *prefix = url.clone();
    }

    // 3. Persist to config.json
    {
        let path_guard = config_state.config_path.lock().map_err(|e| e.to_string())?;
        let config_path = match path_guard.as_ref() {
            Some(p) => p.clone(),
            None => {
                let exe = std::env::current_exe().map_err(|e| e.to_string())?;
                exe.parent()
                    .ok_or("Cannot determine exe directory")?
                    .join("config.json")
            }
        };

        let mut json: serde_json::Value =
            if let Ok(content) = fs::read_to_string(&config_path) {
                serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
            } else {
                serde_json::json!({})
            };

        if let Some(obj) = json.as_object_mut() {
            obj.insert("serverUrl".to_string(), serde_json::Value::String(url.clone()));
        }

        let content = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
        fs::write(&config_path, content).map_err(|e| e.to_string())?;
    }

    // 4. Kill existing print-agent
    if let Ok(mut guard) = sidecar_state.0.lock() {
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    // 5. Restart print-agent with updated config
    let cfg = config_state.config.lock().map_err(|e| e.to_string())?.clone();
    if let Some(sidecar) = find_sidecar() {
        if let Ok(yaml) = write_agent_yaml(&cfg) {
            let mut cmd = Command::new(&sidecar);
            cmd.args(["-config", &yaml])
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            #[cfg(windows)]
            cmd.creation_flags(0x08000000);
            if let Ok(child) = cmd.spawn() {
                if let Ok(mut guard) = sidecar_state.0.lock() {
                    *guard = Some(child);
                }
            }
        }
    }

    // 6. Navigate webview to new server
    let new_url_str = format!("{}{}", url, ticket_path);
    if let Some(window) = app.get_webview_window("main") {
        if let Ok(parsed) = new_url_str.parse::<url::Url>() {
            let _ = window.navigate(parsed);
        }
    }

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn find_sidecar() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let named = dir.join("print-agent-x86_64-pc-windows-msvc.exe");
    if named.exists() { return Some(named); }
    let plain = dir.join("print-agent.exe");
    if plain.exists() { return Some(plain); }
    None
}

fn write_agent_yaml(config: &AppConfig) -> Result<String, Box<dyn std::error::Error>> {
    let yaml_path = std::env::temp_dir().join("antrian-ticket-agent.yaml");
    let content = format!(
        "agent_id: \"{}\"\nserver_url: \"{}\"\nprinter_name: \"{}\"\nretry_delay: {}\npaper_size: \"{}\"\nfeed_lines: {}\n",
        config.agent_id, config.server_url, config.printer_name, config.retry_delay,
        config.paper_size, config.feed_lines
    );
    let mut file = fs::File::create(&yaml_path)?;
    file.write_all(content.as_bytes())?;
    Ok(yaml_path.to_string_lossy().to_string())
}

/// Load config.json and return (config, save_path).
fn load_config() -> (AppConfig, Option<PathBuf>) {
    // Always save next to the executable
    let save_path = std::env::current_exe().ok()
        .and_then(|exe| exe.parent().map(|d| d.join("config.json")));

    let mut candidates: Vec<PathBuf> = vec![];
    if let Some(ref sp) = save_path { candidates.push(sp.clone()); }
    if let Ok(cwd) = std::env::current_dir() { candidates.push(cwd.join("config.json")); }

    for path in &candidates {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(cfg) = serde_json::from_str::<AppConfig>(&content) {
                return (cfg, save_path.or_else(|| Some(path.clone())));
            }
        }
    }

    (
        AppConfig {
            server_url:   default_server_url(),
            ticket_path:  default_ticket_path(),
            agent_id:     default_agent_id(),
            printer_name: default_printer_name(),
            retry_delay:  default_retry_delay(),
            paper_size:   default_paper_size(),
            feed_lines:   default_feed_lines(),
        },
        save_path,
    )
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let (config, config_path) = load_config();

    // Capture values we need before moving `config` into state
    let ticket_url    = format!("{}{}", config.server_url, config.ticket_path);
    let server_prefix_arc = Arc::new(Mutex::new(config.server_url.clone()));
    let server_prefix_nav = Arc::clone(&server_prefix_arc);
    let sidecar_path = find_sidecar();
    let yaml_path    = write_agent_yaml(&config).ok();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            list_printers,
            get_printer_config,
            save_printer,
            get_server_url,
            get_paper_config,
            save_paper_config,
            save_server_url,
        ])
        // Inject printer selector UI on every server page load
        .on_page_load(|webview, payload| {
            if let tauri::webview::PageLoadEvent::Finished = payload.event() {
                let url = payload.url().to_string();
                // Skip local Tauri pages (the splash/error page)
                if url.starts_with("tauri://")
                    || url.starts_with("https://tauri.localhost")
                    || url.starts_with("http://tauri.localhost")
                {
                    return;
                }
                let _ = webview.eval(PRINTER_SELECTOR_JS);
            }
        })
        .setup(move |app| {
            // Register managed state
            app.manage(ConfigState {
                config:      Mutex::new(config),
                config_path: Mutex::new(config_path),
            });
            app.manage(ServerPrefixState(server_prefix_arc));

            // Open the ticket page
            let parsed_url = ticket_url.parse().unwrap_or_else(|_| {
                "http://localhost:8080/ticket".parse().unwrap()
            });

            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(parsed_url))
                .title("Antrian Tiket")
                .inner_size(480.0, 700.0)
                .min_inner_size(400.0, 600.0)
                .on_navigation(move |nav_url| {
                    if let Ok(prefix) = server_prefix_nav.lock() {
                        nav_url.as_str().starts_with(prefix.as_str())
                    } else {
                        true
                    }
                })
                .build()?;

            // Spawn print-agent sidecar (hidden, no console window)
            if let (Some(ref sidecar), Some(ref yaml)) = (&sidecar_path, &yaml_path) {
                let mut cmd = Command::new(sidecar);
                cmd.args(["-config", yaml])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                #[cfg(windows)]
                cmd.creation_flags(0x08000000);
                app.manage(SidecarChild(Mutex::new(cmd.spawn().ok())));
            } else {
                app.manage(SidecarChild(Mutex::new(None)));
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                if let Some(state) = window.try_state::<SidecarChild>() {
                    if let Ok(mut guard) = state.0.lock() {
                        if let Some(mut child) = guard.take() {
                            let _ = child.kill();
                            let _ = child.wait();
                        }
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
