mod audio;
mod network;
mod ui;
mod tray;

use anyhow::Result;
use gtk4::{prelude::*, Application};
use log::{error, info};
use std::sync::{Arc, RwLock};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::process::Command;
use std::env;
use std::rc::Rc;
use std::cell::RefCell;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use serde::Deserialize;

const DAEMON_ADDR: &str = "127.0.0.1:9999";
const APP_ID: &str = "com.github.faraclas.ai-voice-client";

#[derive(Debug, Clone)]
enum HotkeyEvent {
    Press,
    Release,
}

#[derive(Deserialize, Debug)]
struct HealthResponse {
    pub status: String,
    pub gpu_active: Option<bool>,
    pub active_device: Option<String>,
    pub loaded_model: Option<String>,
}

fn ensure_config_exists_and_updated() {
    if let Some(config_dir) = dirs::config_dir() {
        let app_dir = config_dir.join("ai-voice-server");
        if !app_dir.exists() {
            let _ = fs::create_dir_all(&app_dir);
        }
        
        let env_path = app_dir.join("client.env");
        
        let mut existing_content = String::new();
        if env_path.exists() {
            if let Ok(mut file) = fs::File::open(&env_path) {
                let _ = file.read_to_string(&mut existing_content);
            }
        } else {
            existing_content.push_str("# AI Voice Client Configuration\n");
            existing_content.push_str("# Automatically generated configuration file.\n\n");
        }

        let mut appended_anything = false;

        let settings = vec![
            (
                "SERVER_HOST",
                "# Remote AI Voice Server IP/Hostname.\n# SERVER_HOST=\"192.168.0.205\"\n"
            ),
            (
                "SERVER_PORT",
                "# Remote AI Voice Server Port.\n# SERVER_PORT=\"3000\"\n"
            ),
            (
                "AI_VOICE_SERVER_WS_URL",
                "# Full WebSocket URL (optional, overrides SERVER_HOST/SERVER_PORT).\n# AI_VOICE_SERVER_WS_URL=ws://192.168.0.205:3000/stream\n"
            ),
            (
                "AI_VOICE_OUTPUT_MODE",
                "# Options: \"type\" (keyboard injection) or \"clipboard\" (wl-copy).\n# AI_VOICE_OUTPUT_MODE=type\n"
            ),
            (
                "AI_VOICE_ADMIN_API_KEY",
                "# Admin secret key for remote server management (model swapping, restarts).\n# AI_VOICE_ADMIN_API_KEY=your-secret-key\n"
            ),
            (
                "AI_VOICE_TYPING_DELAY",
                "# Delay in ms between keystrokes.\n# AI_VOICE_TYPING_DELAY=2\n"
            ),
            (
                "AI_VOICE_TYPING_HOLD",
                "# Hold time in ms for virtual keystrokes.\n# AI_VOICE_TYPING_HOLD=2\n"
            ),
        ];

        let mut to_append = String::new();
        if !env_path.exists() {
            to_append.push_str(&existing_content);
        }

        for (key, block) in settings {
            if !existing_content.contains(key) {
                to_append.push_str(block);
                to_append.push_str("\n");
                appended_anything = true;
            }
        }

        if appended_anything || !env_path.exists() {
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&env_path) {
                let _ = file.write_all(to_append.as_bytes());
            }
        }
    }
}

fn main() -> Result<()> {
    ensure_config_exists_and_updated();

    let _ = dotenvy::dotenv();
    if let Some(home) = dirs::config_dir() {
        let _ = dotenvy::from_path(home.join("ai-voice-server/client.env"));
        let _ = dotenvy::from_path(home.join("ai-voice/client.conf"));
        let _ = dotenvy::from_path(home.join("ai-voice/client.env"));
    }
    let _ = dotenvy::from_path("/etc/ai-voice-server/client.env");
    env_logger::init();
    
    info!("Starting AI Voice Server Client Daemon...");

    let ws_url = if let (Ok(host), Ok(port)) = (env::var("SERVER_HOST"), env::var("SERVER_PORT")) {
        format!("ws://{}:{}/stream", host, port)
    } else {
        env::var("AI_VOICE_SERVER_WS_URL")
            .unwrap_or_else(|_| "ws://127.0.0.1:3000/stream".to_string())
    };
    
    let admin_key_val = env::var("AI_VOICE_ADMIN_API_KEY").ok().filter(|s| !s.is_empty());
    let initial_output_mode = env::var("AI_VOICE_OUTPUT_MODE").unwrap_or_else(|_| "type".to_string());

    info!("Configured to connect to AI Voice Server at: {}", ws_url);

    // Shared thread-safe state for AppIndicator tray
    let status_state = Arc::new(RwLock::new("Connecting".to_string()));
    let device_state = Arc::new(RwLock::new("cpu".to_string()));
    let model_state = Arc::new(RwLock::new("small.en".to_string()));
    let tray_mode_state = Arc::new(RwLock::new(initial_output_mode.clone()));
    let admin_key_state = Arc::new(RwLock::new(admin_key_val));

    // Channels for inter-thread communication
    let (hotkey_tx, hotkey_rx) = mpsc::channel::<HotkeyEvent>(32);
    let (audio_ctl_tx, audio_ctl_rx) = mpsc::channel::<bool>(2);
    let (audio_data_tx, audio_data_rx) = mpsc::channel::<Vec<u8>>(100);
    let (text_tx, mut text_rx) = mpsc::channel::<String>(10);
    let (status_tx, status_rx) = mpsc::channel::<(String, Option<f64>)>(10);
    let (mod_up_tx, mod_up_rx) = tokio::sync::watch::channel(true);

    // AppIndicator Channels
    let (restart_tx, mut restart_rx) = mpsc::channel::<()>(5);
    let (model_swap_tx, mut model_swap_rx) = mpsc::channel::<String>(5);
    let (test_conn_tx, mut test_conn_rx) = mpsc::channel::<()>(5);
    let (toggle_mode_tx, mut toggle_mode_rx) = mpsc::channel::<()>(5);

    // Spawn ksni AppIndicator System Tray Service
    let tray = tray::AppTray {
        status: status_state.clone(),
        active_device: device_state.clone(),
        loaded_model: model_state.clone(),
        output_mode: tray_mode_state.clone(),
        server_url: ws_url.clone(),
        admin_key: admin_key_state.clone(),
        restart_tx: restart_tx.clone(),
        model_swap_tx: model_swap_tx.clone(),
        test_conn_tx: test_conn_tx.clone(),
        toggle_mode_tx: toggle_mode_tx.clone(),
    };
    
    let tray_service = ksni::TrayService::new(tray);
    let tray_handle = tray_service.handle();
    std::thread::spawn(move || {
        let _ = tray_service.run();
    });

    // Dynamic state for output mode, used by ydotool text injection
    let current_output_mode = Arc::new(tokio::sync::RwLock::new(initial_output_mode));
    let mode_clone_for_ydotool = current_output_mode.clone();

    let http_base_url = ws_url
        .replace("ws://", "http://")
        .replace("wss://", "https://")
        .replace("/stream", "");

    // 2. Start Tokio Runtime for Async Tasks
    let http_url_for_task = http_base_url.clone();
    let status_state_clone = status_state.clone();
    let device_state_clone = device_state.clone();
    let model_state_clone = model_state.clone();
    let tray_handle_clone = tray_handle.clone();
    let status_tx_clone = status_tx.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // 1. Start Audio Capture Subsystem
            if let Err(e) = audio::start_audio_capture(audio_ctl_rx, audio_data_tx) {
                error!("Failed to start audio capture: {}", e);
            }

            // 2. Spawn WebSocket Client Task
            let net_client = network::NetworkClient::new(&ws_url);
            let status_tx_net = status_tx.clone();
            tokio::spawn(async move {
                if let Err(e) = net_client.start(audio_data_rx, text_tx, status_tx_net).await {
                    error!("Network client error: {:#}", e);
                }
            });

            // 3. Background Heartbeat Polling Loop (Every 30 seconds)
            let http_client = reqwest::Client::new();
            let http_url = http_url_for_task.clone();
            let status_st = status_state_clone.clone();
            let dev_st = device_state_clone.clone();
            let mod_st = model_state_clone.clone();
            let th_handle = tray_handle_clone.clone();

            tokio::spawn(async move {
                let health_endpoint = format!("{}/health", http_url);
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
                loop {
                    interval.tick().await;
                    match http_client.get(&health_endpoint).send().await {
                        Ok(resp) if resp.status().is_success() => {
                            if let Ok(data) = resp.json::<HealthResponse>().await {
                                {
                                    let mut st = status_st.write().unwrap_or_else(|e| e.into_inner());
                                    if *st == "Disconnected" || *st == "Offline" || *st == "Connecting" {
                                        *st = "Connected".to_string();
                                    }
                                }
                                if let Some(dev) = data.active_device {
                                    let mut d = dev_st.write().unwrap_or_else(|e| e.into_inner());
                                    *d = dev;
                                }
                                if let Some(m) = data.loaded_model {
                                    let mut md = mod_st.write().unwrap_or_else(|e| e.into_inner());
                                    *md = m;
                                }
                                th_handle.update(|_| {});
                            }
                        }
                        _ => {
                            {
                                let mut st = status_st.write().unwrap_or_else(|e| e.into_inner());
                                *st = "Disconnected".to_string();
                            }
                            th_handle.update(|_| {});
                        }
                    }
                }
            });

            // 4. Remote Service Restart Listener
            let http_url_restart = http_base_url.clone();
            let admin_key_restart = admin_key_state.clone();
            let status_st_restart = status_state_clone.clone();
            let th_handle_restart = tray_handle_clone.clone();
            let status_tx_restart = status_tx_clone.clone();

            tokio::spawn(async move {
                let http_c = reqwest::Client::new();
                while let Some(_) = restart_rx.recv().await {
                    let key_opt = admin_key_restart.read().unwrap_or_else(|e| e.into_inner()).clone();
                    let key = match key_opt {
                        Some(k) => k,
                        None => {
                            let _ = status_tx_restart.send(("toast_no_key".to_string(), None)).await;
                            continue;
                        }
                    };

                    {
                        let mut st = status_st_restart.write().unwrap_or_else(|e| e.into_inner());
                        *st = "Restarting...".to_string();
                    }
                    th_handle_restart.update(|_| {});

                    let restart_endpoint = format!("{}/restart", http_url_restart);
                    info!("Sending remote restart POST to {}", restart_endpoint);
                    match http_c.post(&restart_endpoint)
                        .header("Authorization", format!("Bearer {}", key))
                        .send()
                        .await 
                    {
                        Ok(resp) => {
                            info!("Remote restart POST returned HTTP status: {}", resp.status());
                            if let Ok(text) = resp.text().await {
                                info!("Remote restart response body: {}", text);
                            }
                        }
                        Err(e) => {
                            error!("Failed to send remote restart POST: {}", e);
                        }
                    }

                    // Poll /health every 1s until server comes back online
                    let health_ep = format!("{}/health", http_url_restart);
                    for _ in 0..30 {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        if let Ok(resp) = http_c.get(&health_ep).send().await {
                            if resp.status().is_success() {
                                break;
                            }
                        }
                    }
                }
            });

            // 5. Remote Model Hot-Swap Listener
            let http_url_model = http_base_url.clone();
            let admin_key_model = admin_key_state.clone();
            let status_tx_model = status_tx_clone.clone();

            tokio::spawn(async move {
                let http_c = reqwest::Client::new();
                while let Some(new_model) = model_swap_rx.recv().await {
                    let key_opt = admin_key_model.read().unwrap_or_else(|e| e.into_inner()).clone();
                    let key = match key_opt {
                        Some(k) => k,
                        None => {
                            let _ = status_tx_model.send(("toast_no_key".to_string(), None)).await;
                            continue;
                        }
                    };

                    let _ = status_tx_model.send(("toast_swapping".to_string(), None)).await;
                    let model_endpoint = format!("{}/set_model", http_url_model);
                    let _ = http_c.post(&model_endpoint)
                        .header("Authorization", format!("Bearer {}", key))
                        .json(&serde_json::json!({ "model": new_model }))
                        .send()
                        .await;
                }
            });

            // 6. Test Connection Listener
            let http_url_test = http_base_url.clone();
            let status_tx_test = status_tx_clone.clone();
            tokio::spawn(async move {
                let http_c = reqwest::Client::new();
                while let Some(_) = test_conn_rx.recv().await {
                    let health_ep = format!("{}/health", http_url_test);
                    let start = std::time::Instant::now();
                    match http_c.get(&health_ep).send().await {
                        Ok(resp) if resp.status().is_success() => {
                            let ms = start.elapsed().as_millis();
                            let _ = status_tx_test.send((format!("toast_test_ok_{}", ms), None)).await;
                        }
                        _ => {
                            let _ = status_tx_test.send(("toast_test_fail".to_string(), None)).await;
                        }
                    }
                }
            });

            // 7. Toggle Mode Listener (Tray app synchronization)
            let mode_rw = current_output_mode.clone();
            let mode_tray_rw = tray_mode_state.clone();
            let status_tx_toggle = status_tx_clone.clone();
            let th_handle_toggle = tray_handle_clone.clone();
            tokio::spawn(async move {
                while let Some(_) = toggle_mode_rx.recv().await {
                    let new_mode = {
                        let mut m = mode_rw.write().await;
                        if *m == "type" {
                            *m = "clipboard".to_string();
                        } else {
                            *m = "type".to_string();
                        }
                        m.clone()
                    };

                    {
                        let mut tm = mode_tray_rw.write().unwrap_or_else(|e| e.into_inner());
                        *tm = new_mode.clone();
                    }
                    th_handle_toggle.update(|_| {});

                    if new_mode == "clipboard" {
                        let _ = status_tx_toggle.send(("toast_clipboard".to_string(), None)).await;
                    } else {
                        let _ = status_tx_toggle.send(("toast_type".to_string(), None)).await;
                    }

                    let stx = status_tx_toggle.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        let _ = stx.send(("toast_hide".to_string(), None)).await;
                    });
                }
            });
            
            // 8. Text Injection Task (ydotool)
            tokio::spawn(async move {
                info!("Listening for transcription results...");
                while let Some(text) = text_rx.recv().await {
                    let mut rx = mod_up_rx.clone();
                    if !*rx.borrow() {
                        let _ = rx.changed().await;
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    }

                    let output_mode = mode_clone_for_ydotool.read().await.clone();

                    if output_mode.to_lowercase() == "clipboard" {
                        let _ = Command::new("wl-copy").arg(&text).status().await;
                    } else {
                        let typing_delay = env::var("AI_VOICE_TYPING_DELAY").unwrap_or_else(|_| "2".to_string());
                        let typing_hold = env::var("AI_VOICE_TYPING_HOLD").unwrap_or_else(|_| "2".to_string());
                        
                        let _ = Command::new("ydotool")
                            .arg("type")
                            .arg("-d").arg(&typing_delay)
                            .arg("-H").arg(&typing_hold)
                            .arg(&text)
                            .output()
                            .await;
                    }
                }
            });

            // 9. Spawn UDP Listener for Hotkeys
            let socket = UdpSocket::bind(DAEMON_ADDR).await.expect("Failed to bind UDP socket");
            let mut buf = [0; 32];
            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((len, _)) => {
                        let msg = &buf[..len];
                        if msg == b"PRESS" {
                            let _ = hotkey_tx.send(HotkeyEvent::Press).await;
                        } else if msg == b"RELEASE" {
                            let _ = mod_up_tx.send(false);
                            let _ = hotkey_tx.send(HotkeyEvent::Release).await;
                        } else if msg == b"MODIFIER_UP" {
                            let _ = mod_up_tx.send(true);
                        } else if msg == b"TOGGLE_MODE" {
                            let _ = toggle_mode_tx.send(()).await;
                        }
                    }
                    Err(e) => error!("UDP receive error: {}", e),
                }
            }
        });
    });

    // 3. Main GUI Thread (GTK)
    let app = Application::builder().application_id(APP_ID).build();
    
    let hotkey_rx_opt = Rc::new(RefCell::new(Some(hotkey_rx)));
    let status_rx_opt = Rc::new(RefCell::new(Some(status_rx)));
    let status_state_gtk = status_state.clone();
    let tray_handle_gtk = tray_handle.clone();

    app.connect_activate(move |app| {
        let (window, label) = ui::build_ui(app);
        let window_clone = window.clone();
        let label_clone = label.clone();
        
        let main_context = gtk4::glib::MainContext::default();
        
        if let Some(mut rx) = hotkey_rx_opt.borrow_mut().take() {
            let audio_tx = audio_ctl_tx.clone();
            let st_gtk = status_state_gtk.clone();
            let th_gtk = tray_handle_gtk.clone();
            let win_gtk = window.clone();
            let lbl_gtk = label.clone();

            main_context.spawn_local(async move {
                info!("GTK event loop connected. Listening for hotkeys...");
                while let Some(event) = rx.recv().await {
                    match event {
                        HotkeyEvent::Press => {
                            let current_status = {
                                let st = st_gtk.read().unwrap_or_else(|e| e.into_inner());
                                st.clone()
                            };

                            if current_status == "Disconnected" || current_status == "Offline" {
                                info!("Hotkey Pressed - Server Offline Warning");
                                lbl_gtk.set_text("⚠️ Server Unavailable");
                                win_gtk.set_visible(true);
                                
                                let win_hide = win_gtk.clone();
                                gtk4::glib::timeout_add_local_once(
                                    std::time::Duration::from_millis(1500),
                                    move || {
                                        win_hide.set_visible(false);
                                    },
                                );
                            } else {
                                info!("Hotkey Pressed - Starting Recording");
                                {
                                    let mut st = st_gtk.write().unwrap_or_else(|e| e.into_inner());
                                    *st = "Recording".to_string();
                                }
                                th_gtk.update(|_| {});
                                lbl_gtk.set_text("🎙️ Recording...");
                                win_gtk.set_visible(true);
                                let _ = audio_tx.send(true).await;
                            }
                        }
                        HotkeyEvent::Release => {
                            let current_status = {
                                let st = st_gtk.read().unwrap_or_else(|e| e.into_inner());
                                st.clone()
                            };

                            if current_status != "Disconnected" && current_status != "Offline" {
                                info!("Hotkey Released - Stopping Recording");
                                {
                                    let mut st = st_gtk.write().unwrap_or_else(|e| e.into_inner());
                                    *st = "Processing".to_string();
                                }
                                th_gtk.update(|_| {});
                                lbl_gtk.set_text("⚙️ Transcribing...");
                                let _ = audio_tx.send(false).await;
                            }
                        }
                    }
                }
            });
        }

        if let Some(mut rx) = status_rx_opt.borrow_mut().take() {
            let st_gtk2 = status_state_gtk.clone();
            let th_gtk2 = tray_handle_gtk.clone();
            main_context.spawn_local(async move {
                info!("Listening for server status updates...");
                while let Some((status, pct)) = rx.recv().await {
                    if status == "ws_connected" {
                        {
                            let mut st = st_gtk2.write().unwrap_or_else(|e| e.into_inner());
                            if *st != "Disconnected" && *st != "Offline" && *st != "Recording" {
                                *st = "WS_Open".to_string();
                            }
                        }
                        th_gtk2.update(|_| {});
                    } else if status == "ws_disconnected" {
                        {
                            let mut st = st_gtk2.write().unwrap_or_else(|e| e.into_inner());
                            if *st != "Disconnected" && *st != "Offline" && *st != "Recording" {
                                *st = "Connected".to_string();
                            }
                        }
                        th_gtk2.update(|_| {});
                    } else if status == "downloading" {
                        if !window_clone.is_visible() {
                            window_clone.set_visible(true);
                        }
                        if let Some(p) = pct {
                            label_clone.set_text(&format!("📥 Downloading Model... {:.1}%", p));
                        } else {
                            label_clone.set_text("📥 Downloading Model...");
                        }
                    } else if status == "ready" || status == "done" {
                        {
                            let mut st = st_gtk2.write().unwrap_or_else(|e| e.into_inner());
                            if *st != "Disconnected" && *st != "Offline" && *st != "Recording" {
                                *st = "WS_Open".to_string();
                            }
                        }
                        th_gtk2.update(|_| {});
                        window_clone.set_visible(false);
                    } else if status == "toast_clipboard" {
                        label_clone.set_text("📋 Mode: Clipboard");
                        window_clone.set_visible(true);
                    } else if status == "toast_type" {
                        label_clone.set_text("⌨️ Mode: Typing");
                        window_clone.set_visible(true);
                    } else if status == "toast_no_key" {
                        label_clone.set_text("🔑 Admin Key Required");
                        window_clone.set_visible(true);
                        let win = window_clone.clone();
                        gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(2000), move || {
                            win.set_visible(false);
                        });
                    } else if status == "toast_swapping" {
                        label_clone.set_text("⚙️ Hot-Swapping Model...");
                        window_clone.set_visible(true);
                    } else if status == "toast_test_fail" {
                        label_clone.set_text("❌ Server Connection Failed");
                        window_clone.set_visible(true);
                        let win = window_clone.clone();
                        gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(2000), move || {
                            win.set_visible(false);
                        });
                    } else if status.starts_with("toast_test_ok_") {
                        let ms = status.trim_start_matches("toast_test_ok_");
                        label_clone.set_text(&format!("✅ Server OK ({}ms)", ms));
                        window_clone.set_visible(true);
                        let win = window_clone.clone();
                        gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(2000), move || {
                            win.set_visible(false);
                        });
                    } else if status == "toast_hide" {
                        let current_text = label_clone.text();
                        if current_text.starts_with("📋") || current_text.starts_with("⌨️") {
                            window_clone.set_visible(false);
                        }
                    }
                }
            });
        }
    });

    app.run();
    Ok(())
}
