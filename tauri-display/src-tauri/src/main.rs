#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

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
    #[serde(default = "default_display_path")]
    display_path: String,
    #[serde(default = "default_true")]
    fullscreen: bool,
    #[serde(default = "default_true")]
    kiosk: bool,
    #[serde(default = "default_false")]
    dev_tools: bool,
    #[serde(default = "default_true")]
    use_local_tts: bool,
}

fn default_server_url() -> String {
    "http://localhost:8080".to_string()
}
fn default_display_path() -> String {
    "/display".to_string()
}
fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}

// Wrapper for state management — tracks config + path to config file
struct ConfigState {
    config: Mutex<AppConfig>,
    config_path: Mutex<Option<PathBuf>>,
}

// --- Tauri Commands ---
#[tauri::command]
fn get_server_config(state: tauri::State<'_, ConfigState>) -> Result<serde_json::Value, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "serverUrl": config.server_url,
        "displayPath": config.display_path,
    }))
}

#[tauri::command]
fn save_server_url(state: tauri::State<'_, ConfigState>, server_url: String) -> Result<(), String> {
    // 1. Update in-memory config
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        config.server_url = server_url.clone();
    }

    // 2. Read existing config file (or create new), update only serverUrl, write back
    let path_guard = state.config_path.lock().map_err(|e| e.to_string())?;
    let config_path = match path_guard.as_ref() {
        Some(p) => p.clone(),
        None => {
            // Fallback: save next to executable
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            exe.parent()
                .ok_or("Cannot determine exe directory")?
                .join("config.json")
        }
    };

    // Read existing file or start with empty object
    let mut json_value: serde_json::Value = if let Ok(content) = fs::read_to_string(&config_path) {
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Update only the serverUrl key
    if let Some(obj) = json_value.as_object_mut() {
        obj.insert(
            "serverUrl".to_string(),
            serde_json::Value::String(server_url),
        );
    }

    // Write back
    let content = serde_json::to_string_pretty(&json_value).map_err(|e| e.to_string())?;
    fs::write(&config_path, content).map_err(|e| e.to_string())?;
    println!("Config saved to: {:?}", config_path);

    Ok(())
}

#[tauri::command]
fn navigate_to_server(app: tauri::AppHandle, state: tauri::State<'_, ConfigState>) -> Result<(), String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let url_str = format!("{}{}", config.server_url, config.display_path);
    if let Some(window) = app.get_webview_window("main") {
        if let Ok(url) = url::Url::parse(&url_str) {
            let _ = window.navigate(url);
        }
    }
    Ok(())
}

// --- Main Application ---
fn main() {
    tauri::Builder::default()
        // --- Global Shortcut Plugin ---
        .plugin(build_shortcut_plugin())
        // --- Tauri Commands ---
        .invoke_handler(tauri::generate_handler![get_server_config, save_server_url, navigate_to_server])
        // --- Custom URI Protocol for local audio files ---
        .register_uri_scheme_protocol("local-audio", |ctx, request| {
            let app = ctx.app_handle();
            let uri = request.uri().to_string();
            let path = uri
                .replace("http://local-audio.localhost/", "")
                .replace("https://local-audio.localhost/", "")
                .replace("local-audio://localhost/", "")
                .replace("local-audio://", "");
            let path = path.split('?').next().unwrap_or(&path).to_string();
            let path = path.split('#').next().unwrap_or(&path).to_string();
            let decoded_path =
                percent_encoding::percent_decode_str(&path).decode_utf8_lossy().to_string();

            let resource_path = resolve_audio_path(app, &decoded_path);

            match fs::read(&resource_path) {
                Ok(data) => tauri::http::Response::builder()
                    .status(200)
                    .header("content-type", "audio/mpeg")
                    .header("access-control-allow-origin", "*")
                    .body(data)
                    .unwrap(),
                Err(_) => tauri::http::Response::builder()
                    .status(404)
                    .body(Vec::new())
                    .unwrap(),
            }
        })
        // --- Page Load Handler: Inject JS only for server pages (not local index.html) ---
        .on_page_load(|webview, payload| {
            if let tauri::webview::PageLoadEvent::Finished = payload.event() {
                let current_url = payload.url().to_string();

                // Skip injection for local pages (tauri://localhost or https://tauri.localhost)
                if current_url.starts_with("tauri://")
                    || current_url.starts_with("https://tauri.localhost")
                    || current_url.starts_with("http://tauri.localhost")
                {
                    return;
                }

                let app = webview.app_handle();
                if let Some(config_state) = app.try_state::<ConfigState>() {
                    let config = config_state.config.lock().unwrap().clone();
                    let init_js = get_init_js(config.use_local_tts, app);
                    let _ = webview.eval(&init_js);

                    // Double-check: re-inject audio flags after 1 second
                    let webview_clone = webview.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                        let _ = webview_clone.eval(
                            r#"
                            if (typeof audioEnabled !== 'undefined') audioEnabled = true;
                            if (typeof audioInitialized !== 'undefined') audioInitialized = true;
                            window.audioEnabled = true;
                            window.audioInitialized = true;
                            const _btn = document.getElementById('enable-audio-btn');
                            if (_btn) _btn.remove();
                            if (window.audioContext && window.audioContext.state === 'suspended') {
                                window.audioContext.resume();
                            }
                            "#,
                        );
                    });
                }
            }
        })
        // --- App Setup ---
        .setup(|app| {
            let (config, config_path) = load_config(app.handle());
            println!(
                "Config: server={}, path={}, fullscreen={}, kiosk={}, localTTS={}",
                config.server_url,
                config.display_path,
                config.fullscreen,
                config.kiosk,
                config.use_local_tts
            );

            // Store config + path in app state
            app.manage(ConfigState {
                config: Mutex::new(config.clone()),
                config_path: Mutex::new(config_path),
            });

            // Create main window programmatically with on_navigation
            let window = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::App("index.html".into()),
            )
            .title("Display Antrian")
            .inner_size(1920.0, 1080.0)
            .resizable(true)
            .fullscreen(config.fullscreen)
            .decorations(!config.kiosk)
            .on_navigation(|nav_url| {
                let url_str = nav_url.as_str();
                url_str.starts_with("tauri://")
                    || url_str.starts_with("https://tauri.localhost")
                    || url_str.starts_with("http://tauri.localhost")
                    || url_str.starts_with("http://")
                    || url_str.starts_with("https://")
            })
            .build()?;

            // Open devtools if configured
            if config.dev_tools {
                window.open_devtools();
            }

            // Register global shortcuts
            #[cfg(desktop)]
            register_shortcuts(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// --- Build Global Shortcut Plugin ---
fn build_shortcut_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};

    let f5 = Shortcut::new(None, Code::F5);
    let f11 = Shortcut::new(None, Code::F11);
    let escape = Shortcut::new(None, Code::Escape);
    let ctrl_shift_d =
        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyD);
    let ctrl_shift_v =
        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyV);
    let ctrl_shift_t =
        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyT);
    let ctrl_shift_l =
        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyL);
    let ctrl_shift_a =
        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyA);
    let ctrl_q = Shortcut::new(Some(Modifiers::CONTROL), Code::KeyQ);

    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(move |app, shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }

            let window = match app.get_webview_window("main") {
                Some(w) => w,
                None => return,
            };

            // F5 - Reload page from server
            if shortcut == &f5 {
                if let Some(config_state) = app.try_state::<ConfigState>() {
                    let config = config_state.config.lock().unwrap().clone();
                    let url_str = format!("{}{}", config.server_url, config.display_path);
                    if let Ok(url) = url::Url::parse(&url_str) {
                        let _ = window.navigate(url);
                    }
                }
            }
            // F11 - Toggle fullscreen
            else if shortcut == &f11 {
                if let Ok(is_fs) = window.is_fullscreen() {
                    let _ = window.set_fullscreen(!is_fs);
                }
            }
            // Escape - Exit fullscreen (disabled in kiosk mode)
            else if shortcut == &escape {
                if let Some(config_state) = app.try_state::<ConfigState>() {
                    let config = config_state.config.lock().unwrap().clone();
                    if !config.kiosk {
                        let _ = window.set_fullscreen(false);
                    }
                }
            }
            // Ctrl+Shift+D - Toggle DevTools
            else if shortcut == &ctrl_shift_d {
                if window.is_devtools_open() {
                    window.close_devtools();
                } else {
                    window.open_devtools();
                }
            }
            // Ctrl+Shift+V - List available voices
            else if shortcut == &ctrl_shift_v {
                let _ = window.eval(
                    r#"
                    const voices = speechSynthesis.getVoices();
                    console.log('=== AVAILABLE VOICES ===');
                    voices.forEach((v, i) => {
                        const isIndo = v.lang.startsWith('id') ? ' [INDONESIAN]' : '';
                        console.log(i + ': ' + v.name + ' (' + v.lang + ')' + isIndo);
                    });
                    console.log('========================');
                    "#,
                );
            }
            // Ctrl+Shift+T - Test Web Speech API TTS
            else if shortcut == &ctrl_shift_t {
                let _ = window.eval(
                    r#"
                    console.log('Testing Web Speech API TTS...');
                    const text = 'Nomor antrian A satu, silakan menuju Loket 1';
                    const utterance = new SpeechSynthesisUtterance(text);
                    utterance.lang = 'id-ID';
                    utterance.rate = 0.9;
                    const voices = speechSynthesis.getVoices();
                    const idVoice = voices.find(v => v.lang.startsWith('id') || v.name.includes('Indonesia') || v.name.includes('Gadis'));
                    if (idVoice) { utterance.voice = idVoice; console.log('Using voice:', idVoice.name); }
                    speechSynthesis.speak(utterance);
                    "#,
                );
            }
            // Ctrl+Shift+L - Test Local TTS
            else if shortcut == &ctrl_shift_l {
                let _ = window.eval(
                    r#"
                    console.log('Testing Local TTS...');
                    if (typeof testLocalTTS === 'function') {
                        testLocalTTS('A001', 'Loket A1');
                    } else {
                        console.error('Local TTS not loaded. Make sure useLocalTTS is enabled in config.');
                    }
                    "#,
                );
            }
            // Ctrl+Shift+A - Test all counter formats
            else if shortcut == &ctrl_shift_a {
                let _ = window.eval(
                    r#"
                    console.log('Testing all counter formats...');
                    if (typeof testCounterFormats === 'function') {
                        testCounterFormats();
                    } else {
                        console.error('Local TTS not loaded.');
                    }
                    "#,
                );
            }
            // Ctrl+Q - Exit application
            else if shortcut == &ctrl_q {
                app.exit(0);
            }
        })
        .build()
}

// --- Register Shortcuts ---
#[cfg(desktop)]
fn register_shortcuts(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

    let shortcuts = vec![
        Shortcut::new(None, Code::F5),
        Shortcut::new(None, Code::F11),
        Shortcut::new(None, Code::Escape),
        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyD),
        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyV),
        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyT),
        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyL),
        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyA),
        Shortcut::new(Some(Modifiers::CONTROL), Code::KeyQ),
    ];

    for shortcut in shortcuts {
        app.global_shortcut().register(shortcut)?;
    }

    Ok(())
}

// --- Configuration Loading (returns config + path to the config file) ---
fn load_config(app_handle: &tauri::AppHandle) -> (AppConfig, Option<PathBuf>) {
    let mut paths_to_try: Vec<PathBuf> = Vec::new();

    // Save path: always next to executable (writable)
    let save_path = std::env::current_exe().ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("config.json")));

    // Priority 1: Next to the executable
    if let Some(ref sp) = save_path {
        paths_to_try.push(sp.clone());
    }
    // Priority 2: Resources path (for installed/bundled apps)
    if let Ok(resource_path) = app_handle.path().resource_dir() {
        paths_to_try.push(resource_path.join("config.json"));
        paths_to_try.push(resource_path.join("_up_").join("config.json"));
    }
    // Priority 3: Current working directory (for dev)
    if let Ok(cwd) = std::env::current_dir() {
        paths_to_try.push(cwd.join("config.json"));
    }

    for path in &paths_to_try {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                println!("Config loaded from: {:?}", path);
                return (config, save_path.or_else(|| Some(path.clone())));
            }
        }
    }

    println!("Using default config");
    (
        AppConfig {
            server_url: default_server_url(),
            display_path: default_display_path(),
            fullscreen: true,
            kiosk: true,
            dev_tools: false,
            use_local_tts: true,
        },
        save_path,
    )
}

// --- JavaScript Injection ---
fn get_init_js(use_local_tts: bool, app_handle: &tauri::AppHandle) -> String {
    // Read main-injection.js
    let main_injection_js = read_resource_file(app_handle, "js/main-injection.js");

    // Read local-tts.js if enabled
    let local_tts_js = if use_local_tts {
        let tts_script = read_resource_file(app_handle, "local-tts.js");
        let audio_url = if cfg!(target_os = "windows") {
            "http://local-audio.localhost"
        } else {
            "local-audio://localhost"
        };
        format!(
            r#"
            window.LOCAL_AUDIO_URL = '{}';
            window.USE_LOCAL_TTS = true;
            console.log('Tauri: Local TTS enabled, audio URL:', window.LOCAL_AUDIO_URL);
            {}
            console.log('Tauri: Local TTS module injected, USE_LOCAL_TTS =', window.USE_LOCAL_TTS);
            "#,
            audio_url, tts_script
        )
    } else {
        String::new()
    };

    format!("{}\n\n{}", main_injection_js, local_tts_js)
}

// --- Read a file from the resource directory ---
fn read_resource_file(app_handle: &tauri::AppHandle, relative_path: &str) -> String {
    let mut paths_to_try: Vec<PathBuf> = Vec::new();

    if let Ok(resource_path) = app_handle.path().resource_dir() {
        paths_to_try.push(resource_path.join(relative_path));
        paths_to_try.push(resource_path.join("_up_").join(relative_path));
    }
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            paths_to_try.push(dir.join(relative_path));
            paths_to_try.push(dir.join("_up_").join(relative_path));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        paths_to_try.push(cwd.join(relative_path));
    }

    for path in &paths_to_try {
        if let Ok(content) = fs::read_to_string(path) {
            println!("Loaded resource: {:?}", path);
            return content;
        }
    }

    eprintln!("Warning: Could not find resource file: {}", relative_path);
    String::new()
}

// --- Resolve audio file path ---
fn resolve_audio_path(app_handle: &tauri::AppHandle, filename: &str) -> PathBuf {
    let mut paths_to_try: Vec<PathBuf> = Vec::new();

    if let Ok(resource_path) = app_handle.path().resource_dir() {
        paths_to_try.push(resource_path.join("audio").join(filename));
        paths_to_try.push(resource_path.join("_up_").join("audio").join(filename));
    }
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            paths_to_try.push(dir.join("audio").join(filename));
            paths_to_try.push(dir.join("_up_").join("audio").join(filename));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        paths_to_try.push(cwd.join("audio").join(filename));
    }

    for path in &paths_to_try {
        if path.exists() {
            return path.clone();
        }
    }

    paths_to_try
        .into_iter()
        .next()
        .unwrap_or_else(|| PathBuf::from(filename))
}
