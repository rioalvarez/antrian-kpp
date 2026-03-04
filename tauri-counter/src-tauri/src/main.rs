#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

// --- Configuration Structs ---
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AppConfig {
    #[serde(default = "default_server_url")]
    server_url: String,
    // ID internal loket (None = belum pernah dipilih)
    counter_id: Option<u32>,
    // Nama loket yang ditampilkan ke user (disimpan agar startup tidak perlu fetch)
    counter_name: Option<String>,
}

fn default_server_url() -> String {
    "http://localhost:8080".to_string()
}

struct ConfigState {
    config: Mutex<AppConfig>,
    config_path: Mutex<Option<PathBuf>>,
}

// --- Helpers ---

fn resolve_config_path(state: &ConfigState) -> Result<PathBuf, String> {
    let guard = state.config_path.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(p) => Ok(p.clone()),
        None => {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            Ok(exe
                .parent()
                .ok_or("Tidak dapat menentukan direktori executable")?
                .join("config.json"))
        }
    }
}

fn write_config_file(
    path: &PathBuf,
    server_url: &str,
    counter_id: Option<u32>,
    counter_name: Option<&str>,
) -> Result<(), String> {
    let mut json: serde_json::Value = if let Ok(content) = fs::read_to_string(path) {
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if let Some(obj) = json.as_object_mut() {
        obj.insert("serverUrl".to_string(), serde_json::Value::String(server_url.to_string()));
        match counter_id {
            Some(id) => { obj.insert("counterId".to_string(), serde_json::json!(id)); }
            None      => { obj.remove("counterId"); }
        }
        match counter_name {
            Some(n) if !n.is_empty() => { obj.insert("counterName".to_string(), serde_json::Value::String(n.to_string())); }
            _                        => { obj.remove("counterName"); }
        }
        obj.remove("counterPath"); // bersihkan key lama
    }

    let content = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}

/// Ekstrak nomor loket dari URL seperti http://host:port/counter/3 → Some(3)
fn parse_counter_id_from_url(url: &str) -> Option<u32> {
    let segments: Vec<&str> = url.split('/').collect();
    for i in 0..segments.len().saturating_sub(1) {
        if segments[i] == "counter" {
            let prev = if i > 0 { segments[i - 1] } else { "" };
            if prev != "api" {
                return segments[i + 1].parse::<u32>().ok();
            }
        }
    }
    None
}

/// Bangun script JS untuk floating banner "Simpan sebagai default?"
/// Banner fetch counter_name dari API (same-origin), lalu ikutkan saat invoke save_config.
fn build_save_banner_js(counter_id: u32, server_url: &str) -> String {
    let safe_url = server_url.replace('\\', r"\\").replace('\'', r"\'");

    format!(r#"(function(){{
  if(document.getElementById('_tauri_save_banner'))return;
  var cid={cid};
  var surl='{url}';

  function showBanner(counterName){{
    if(document.getElementById('_tauri_save_banner'))return;
    var el=document.createElement('div');
    el.id='_tauri_save_banner';
    el.style.cssText='position:fixed;bottom:24px;right:24px;background:#ffffff;border:1px solid #e4e4e7;border-radius:14px;padding:18px 20px;box-shadow:0 8px 32px rgba(0,0,0,0.14);z-index:999999;font-family:Segoe UI,system-ui,sans-serif;max-width:300px;animation:_tsb_in 0.25s ease;line-height:1.4';
    el.innerHTML='<style>@keyframes _tsb_in{{from{{opacity:0;transform:translateY(10px)}}to{{opacity:1;transform:translateY(0)}}}}</style>'
      +'<div style="font-weight:700;font-size:0.95em;color:#18181b;margin-bottom:6px">Simpan sebagai Default?</div>'
      +'<div style="font-size:0.95em;font-weight:600;color:#0ea5e9;margin-bottom:10px">'+counterName+'</div>'
      +'<div style="font-size:0.8em;color:#6b7280;margin-bottom:14px">akan dipilih otomatis saat aplikasi dibuka kembali.</div>'
      +'<div style="display:flex;gap:8px">'
      +'<button id="_tsb_save" style="flex:1;background:#0ea5e9;color:#fff;border:none;padding:9px 8px;border-radius:8px;cursor:pointer;font-size:0.85em;font-weight:600">Simpan</button>'
      +'<button id="_tsb_dismiss" style="flex:1;background:#f4f4f5;color:#52525b;border:none;padding:9px 8px;border-radius:8px;cursor:pointer;font-size:0.85em">Abaikan</button>'
      +'</div>';
    document.body.appendChild(el);
    document.getElementById('_tsb_save').onclick=function(){{
      this.textContent='Menyimpan...';
      this.disabled=true;
      window.__TAURI__.core.invoke('save_config',{{serverUrl:surl,counterId:cid,counterName:counterName}})
        .then(function(){{el.remove();}})
        .catch(function(){{el.remove();}});
    }};
    document.getElementById('_tsb_dismiss').onclick=function(){{el.remove();}};
    setTimeout(function(){{if(document.getElementById('_tauri_save_banner'))el.remove();}},15000);
  }}

  // Fetch counter_name dari API (same-origin, berjalan di halaman server)
  fetch('/api/counter/'+cid)
    .then(function(r){{return r.json();}})
    .then(function(d){{
      showBanner(d.counter_name||d.counter_number||String(cid));
    }})
    .catch(function(){{
      showBanner(String(cid));
    }});
}})();"#,
        cid = counter_id,
        url = safe_url,
    )
}

// --- Tauri Commands ---

#[tauri::command]
fn get_server_config(state: tauri::State<'_, ConfigState>) -> Result<serde_json::Value, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "serverUrl":   config.server_url,
        "counterId":   config.counter_id,
        "counterName": config.counter_name,
    }))
}

/// Simpan hanya server URL (dipakai saat user koreksi alamat server)
#[tauri::command]
fn save_server_url(state: tauri::State<'_, ConfigState>, server_url: String) -> Result<(), String> {
    let config_path = resolve_config_path(&state)?;
    let (counter_id, counter_name) = {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        config.server_url = server_url.clone();
        (config.counter_id, config.counter_name.clone())
    };
    write_config_file(&config_path, &server_url, counter_id, counter_name.as_deref())?;
    println!("Server URL disimpan: {}", server_url);
    Ok(())
}

/// Simpan server URL + ID loket + nama loket (dipanggil dari banner konfirmasi)
#[tauri::command]
fn save_config(
    state: tauri::State<'_, ConfigState>,
    server_url: String,
    counter_id: u32,
    counter_name: String,
) -> Result<(), String> {
    let config_path = resolve_config_path(&state)?;
    let name_opt = if counter_name.is_empty() { None } else { Some(counter_name.as_str()) };
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        config.server_url  = server_url.clone();
        config.counter_id  = Some(counter_id);
        config.counter_name = name_opt.map(str::to_string);
    }
    write_config_file(&config_path, &server_url, Some(counter_id), name_opt)?;
    println!("Config disimpan: serverUrl={}, counterId={}, counterName={:?}", server_url, counter_id, name_opt);
    Ok(())
}

/// Navigasi langsung ke loket tertentu (dipakai saat counter_id sudah tersimpan)
#[tauri::command]
fn navigate_to_counter(
    app: tauri::AppHandle,
    state: tauri::State<'_, ConfigState>,
    counter_id: u32,
) -> Result<(), String> {
    let server_url = state.config.lock().map_err(|e| e.to_string())?.server_url.clone();
    let url_str = format!("{}/counter/{}", server_url, counter_id);
    println!("Navigasi ke: {}", url_str);
    if let Some(win) = app.get_webview_window("main") {
        if let Ok(url) = tauri::Url::parse(&url_str) {
            let _ = win.navigate(url);
        }
    }
    Ok(())
}

/// Navigasi ke halaman pilih loket (/counters)
#[tauri::command]
fn navigate_to_counters(
    app: tauri::AppHandle,
    state: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    let server_url = state.config.lock().map_err(|e| e.to_string())?.server_url.clone();
    let url_str = format!("{}/counters", server_url);
    println!("Navigasi ke: {}", url_str);
    if let Some(win) = app.get_webview_window("main") {
        if let Ok(url) = tauri::Url::parse(&url_str) {
            let _ = win.navigate(url);
        }
    }
    Ok(())
}

// --- Configuration Loading ---
fn load_config() -> (AppConfig, Option<PathBuf>) {
    let mut paths: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join("config.json"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join("config.json"));
    }

    for path in &paths {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                println!("Config dimuat dari: {:?}", path);
                return (config, Some(path.clone()));
            }
        }
    }

    println!("Menggunakan config default");
    (
        AppConfig {
            server_url:   default_server_url(),
            counter_id:   None,
            counter_name: None,
        },
        None,
    )
}

fn main() {
    let (config, config_path) = load_config();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_server_config,
            save_server_url,
            save_config,
            navigate_to_counter,
            navigate_to_counters,
        ])
        .setup(move |app| {
            app.manage(ConfigState {
                config: Mutex::new(config.clone()),
                config_path: Mutex::new(config_path),
            });

            let app_handle = app.handle().clone();

            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Antrian Loket")
                .inner_size(900.0, 700.0)
                .min_inner_size(480.0, 600.0)
                .on_navigation(move |nav_url| {
                    let url_str = nav_url.as_str().to_string();

                    if let Some(counter_id) = parse_counter_id_from_url(&url_str) {
                        let app_clone = app_handle.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_millis(1000));

                            let state = app_clone.state::<ConfigState>();
                            let (saved_id, server_url) = {
                                let cfg = state.config.lock().unwrap();
                                (cfg.counter_id, cfg.server_url.clone())
                            };

                            if saved_id != Some(counter_id) {
                                if let Some(win) = app_clone.get_webview_window("main") {
                                    let js = build_save_banner_js(counter_id, &server_url);
                                    let _ = win.eval(&js);
                                }
                            }
                        });
                    }

                    url_str.starts_with("tauri://")
                        || url_str.starts_with("https://tauri.localhost")
                        || url_str.starts_with("http://tauri.localhost")
                        || url_str.starts_with("http://")
                        || url_str.starts_with("https://")
                })
                .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error menjalankan aplikasi tauri");
}
