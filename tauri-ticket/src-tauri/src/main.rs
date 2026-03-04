#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Deserialize;
use std::fs;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

#[derive(Deserialize)]
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
}

fn default_server_url() -> String { "http://localhost:8080".to_string() }
fn default_ticket_path() -> String { "/ticket".to_string() }
fn default_agent_id() -> String { "printer-lobi".to_string() }
fn default_printer_name() -> String { "ECO80".to_string() }
fn default_retry_delay() -> u32 { 5 }

fn load_config() -> AppConfig {
    // Try config.json next to executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let path = dir.join("config.json");
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                    println!("Config loaded from: {:?}", path);
                    return config;
                }
            }
        }
    }

    // Try config.json in current directory
    if let Ok(content) = fs::read_to_string("config.json") {
        if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
            println!("Config loaded from: current directory");
            return config;
        }
    }

    println!("Using default config");
    AppConfig {
        server_url: default_server_url(),
        ticket_path: default_ticket_path(),
        agent_id: default_agent_id(),
        printer_name: default_printer_name(),
        retry_delay: default_retry_delay(),
    }
}

/// Find the print-agent binary next to the executable
fn find_sidecar() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;

    // Check for the Tauri sidecar name convention first
    let sidecar = dir.join("print-agent-x86_64-pc-windows-msvc.exe");
    if sidecar.exists() {
        return Some(sidecar);
    }

    // Then check plain name
    let plain = dir.join("print-agent.exe");
    if plain.exists() {
        return Some(plain);
    }

    None
}

/// Generate a temporary YAML config file for print-agent
fn write_agent_yaml(config: &AppConfig) -> Result<String, Box<dyn std::error::Error>> {
    let temp_dir = std::env::temp_dir();
    let yaml_path = temp_dir.join("antrian-ticket-agent.yaml");

    let yaml_content = format!(
        "agent_id: \"{}\"\nserver_url: \"{}\"\nprinter_name: \"{}\"\nretry_delay: {}\n",
        config.agent_id, config.server_url, config.printer_name, config.retry_delay
    );

    let mut file = fs::File::create(&yaml_path)?;
    file.write_all(yaml_content.as_bytes())?;
    println!("Agent YAML written to: {:?}", yaml_path);

    Ok(yaml_path.to_string_lossy().to_string())
}

struct SidecarChild(Mutex<Option<Child>>);

fn main() {
    let config = load_config();
    let url = format!("{}{}", config.server_url, config.ticket_path);
    let server_url = config.server_url.clone();

    // Prepare sidecar info before moving config into closure
    let sidecar_path = find_sidecar();
    let yaml_path = write_agent_yaml(&config).ok();

    tauri::Builder::default()
        .setup(move |app| {
            // Create the webview window
            let parsed_url = url.parse().unwrap_or_else(|_| {
                "http://localhost:8080/ticket".parse().unwrap()
            });

            let server = server_url.clone();
            WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External(parsed_url),
            )
            .title("Antrian Tiket")
            .inner_size(480.0, 700.0)
            .min_inner_size(400.0, 600.0)
            .on_navigation(move |nav_url| {
                nav_url.as_str().starts_with(&server)
            })
            .build()?;

            // Spawn print-agent sidecar
            if let (Some(ref sidecar), Some(ref yaml)) = (&sidecar_path, &yaml_path) {
                println!("Starting print-agent: {:?}", sidecar);
                let mut cmd = Command::new(sidecar);
                cmd.args(["-config", yaml])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                #[cfg(windows)]
                cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
                match cmd.spawn() {
                    Ok(child) => {
                        println!("print-agent started (PID: {})", child.id());
                        app.manage(SidecarChild(Mutex::new(Some(child))));
                    }
                    Err(e) => {
                        eprintln!("Failed to start print-agent: {}", e);
                        app.manage(SidecarChild(Mutex::new(None)));
                    }
                }
            } else {
                if sidecar_path.is_none() {
                    eprintln!("print-agent binary not found, skipping sidecar");
                }
                if yaml_path.is_none() {
                    eprintln!("Failed to write agent YAML config");
                }
                app.manage(SidecarChild(Mutex::new(None)));
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                // Kill sidecar when window is destroyed
                if let Some(state) = window.try_state::<SidecarChild>() {
                    if let Ok(mut guard) = state.0.lock() {
                        if let Some(mut child) = guard.take() {
                            println!("Killing print-agent (PID: {})...", child.id());
                            let _ = child.kill();
                            let _ = child.wait();
                            println!("print-agent stopped.");
                        }
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
