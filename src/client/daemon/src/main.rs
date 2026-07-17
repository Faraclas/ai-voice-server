mod audio;
mod network;
mod ui;

use anyhow::Result;
use gtk4::{prelude::*, Application};
use log::{error, info};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::process::Command;
use std::env;
use std::rc::Rc;
use std::cell::RefCell;

const DAEMON_ADDR: &str = "127.0.0.1:9999";
const APP_ID: &str = "com.github.faraclas.ai-voice-client";

#[derive(Debug, Clone)]
enum HotkeyEvent {
    Press,
    Release,
}

fn main() -> Result<()> {
    // Load local .env first so it takes precedence, then user config, then system config
    let _ = dotenvy::dotenv();
    if let Some(home) = dirs::config_dir() {
        let _ = dotenvy::from_path(home.join("ai-voice-server/client.env"));
    }
    let _ = dotenvy::from_path("/etc/ai-voice-server/client.env");
    env_logger::init();
    
    info!("Starting AI Voice Server Client Daemon...");

    // Read the server URL from the environment or default to local testing
    let ws_url = env::var("AI_VOICE_SERVER_WS_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:3000/stream".to_string());
    
    info!("Configured to connect to AI Voice Server at: {}", ws_url);

    // Channels for inter-thread communication
    let (hotkey_tx, hotkey_rx) = mpsc::channel::<HotkeyEvent>(32);
    let (audio_ctl_tx, audio_ctl_rx) = mpsc::channel::<bool>(2);
    let (audio_data_tx, audio_data_rx) = mpsc::channel::<Vec<u8>>(100);
    let (text_tx, mut text_rx) = mpsc::channel::<String>(10);
    let (status_tx, status_rx) = mpsc::channel::<(String, Option<f64>)>(10);
    
    // Channel to coordinate the physical release of the modifier key
    let (mod_up_tx, mut mod_up_rx) = tokio::sync::watch::channel(true);

    // 2. Start Tokio Runtime for Async Tasks (Networking and UDP)
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // 1. Start Audio Capture Subsystem
            if let Err(e) = audio::start_audio_capture(audio_ctl_rx, audio_data_tx) {
                error!("Failed to start audio capture: {}", e);
            }

            // Spawn WebSocket Client Task
            let net_client = network::NetworkClient::new(&ws_url);
            tokio::spawn(async move {
                if let Err(e) = net_client.start(audio_data_rx, text_tx, status_tx).await {
                    error!("Network client error: {:#}", e);
                }
            });
            
            // Spawn Text Injection Task (ydotool) inside Tokio runtime
            tokio::spawn(async move {
                info!("Listening for transcription results...");
                while let Some(text) = text_rx.recv().await {
                    // Wait for physical modifier release to prevent global shortcut triggering
                    let mut rx = mod_up_rx.clone();
                    if !*rx.borrow() {
                        info!("Waiting for physical modifier key to be released...");
                        let _ = rx.changed().await;
                    }

                    info!("Injecting transcription ({} bytes)...", text.len());
                    log::debug!("Exact text: {}", text);
                    let output = Command::new("ydotool")
                        .arg("type")
                        .arg("-d").arg("0")
                        .arg("-H").arg("0")
                        .arg(&text)
                        .output()
                        .await;
                    match output {
                        Ok(o) if o.status.success() => {
                            info!("Successfully injected text.");
                        }
                        Ok(o) => {
                            error!("ydotool failed: {:?}", String::from_utf8_lossy(&o.stderr));
                        }
                        Err(e) => {
                            error!("Failed to execute ydotool (is the daemon running?): {}", e);
                        }
                    }
                }
            });

            // Spawn UDP Listener for Hotkeys
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
                        }
                    }
                    Err(e) => error!("UDP receive error: {}", e),
                }
            }
        });
    });

    // 3. Main GUI Thread (GTK)
    let app = Application::builder().application_id(APP_ID).build();
    
    // Wrap receivers in Rc<RefCell<Option<T>>> so they can be moved into the Fn closure once
    let hotkey_rx_opt = Rc::new(RefCell::new(Some(hotkey_rx)));
    let status_rx_opt = Rc::new(RefCell::new(Some(status_rx)));

    app.connect_activate(move |app| {
        let (window, label) = ui::build_ui(app);
        let window_clone = window.clone();
        let label_clone = label.clone();
        
        // Setup GLib MainContext to process Tokio events safely in the GTK thread
        let main_context = gtk4::glib::MainContext::default();
        
        if let Some(mut rx) = hotkey_rx_opt.borrow_mut().take() {
            let audio_tx = audio_ctl_tx.clone();
            main_context.spawn_local(async move {
                info!("GTK event loop connected. Listening for hotkeys...");
                while let Some(event) = rx.recv().await {
                    match event {
                        HotkeyEvent::Press => {
                            info!("Hotkey Pressed - Starting Recording");
                            label.set_text("🎙️ Recording...");
                            window.set_visible(true);
                            let _ = audio_tx.send(true).await;
                        }
                        HotkeyEvent::Release => {
                            info!("Hotkey Released - Stopping Recording");
                            window.set_visible(false);
                            let _ = audio_tx.send(false).await;
                        }
                    }
                }
            });
        }

        // Removed text_rx from GTK thread, moved to Tokio runtime
        if let Some(mut rx) = status_rx_opt.borrow_mut().take() {
            main_context.spawn_local(async move {
                info!("Listening for server status updates...");
                while let Some((status, pct)) = rx.recv().await {
                    if status == "downloading" {
                        if !window_clone.is_visible() {
                            window_clone.set_visible(true);
                        }
                        if let Some(p) = pct {
                            label_clone.set_text(&format!("📥 Downloading Model... {:.1}%", p));
                        } else {
                            label_clone.set_text("📥 Downloading Model...");
                        }
                    } else if status == "ready" {
                        window_clone.set_visible(false);
                        label_clone.set_text("🎙️ Recording...");
                    }
                }
            });
        }
    });

    app.run();
    Ok(())
}

