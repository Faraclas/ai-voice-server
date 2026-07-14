mod audio;
mod network;
mod ui;

use anyhow::Result;
use gtk4::{prelude::*, Application};
use log::{error, info};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use std::process::Command;
use std::env;

const DAEMON_ADDR: &str = "127.0.0.1:9999";
const APP_ID: &str = "com.github.faraclas.ai-voice-client";

#[derive(Debug, Clone)]
enum HotkeyEvent {
    Press,
    Release,
}

fn main() -> Result<()> {
    // Load environment variables from .env file if present
    let _ = dotenvy::dotenv();
    env_logger::init();
    
    info!("Starting AI Voice Server Client Daemon...");

    // Read the server URL from the environment or default to local testing
    let ws_url = env::var("AI_VOICE_SERVER_WS_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:3000/stream".to_string());
    
    info!("Configured to connect to AI Voice Server at: {}", ws_url);

    // Channels for inter-thread communication
    let (hotkey_tx, mut hotkey_rx) = mpsc::channel::<HotkeyEvent>(32);
    let (audio_ctl_tx, audio_ctl_rx) = mpsc::channel::<bool>(2);
    let (audio_data_tx, audio_data_rx) = mpsc::channel::<Vec<u8>>(100);
    let (text_tx, mut text_rx) = mpsc::channel::<String>(10);

    // 1. Start Audio Capture Subsystem
    audio::start_audio_capture(audio_ctl_rx, audio_data_tx)?;

    // 2. Start Tokio Runtime for Async Tasks (Networking and UDP)
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Spawn WebSocket Client Task
            let net_client = network::NetworkClient::new(&ws_url);
            tokio::spawn(async move {
                if let Err(e) = net_client.start(audio_data_rx, text_tx).await {
                    error!("WebSocket client error: {}", e);
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
                            let _ = hotkey_tx.send(HotkeyEvent::Release).await;
                        }
                    }
                    Err(e) => error!("UDP receive error: {}", e),
                }
            }
        });
    });

    // 3. Main GUI Thread (GTK)
    let app = Application::builder().application_id(APP_ID).build();
    
    app.connect_activate(move |app| {
        let window = ui::build_ui(app);
        
        // Setup GLib MainContext to process Tokio events safely in the GTK thread
        let main_context = gtk4::glib::MainContext::default();
        
        // Task 1: Handle Hotkey UI Toggling
        main_context.spawn_local(async move {
            info!("GTK event loop connected. Listening for hotkeys...");
            while let Some(event) = hotkey_rx.recv().await {
                match event {
                    HotkeyEvent::Press => {
                        info!("Hotkey Pressed - Starting Recording");
                        window.set_visible(true);
                        let _ = audio_ctl_tx.send(true).await;
                    }
                    HotkeyEvent::Release => {
                        info!("Hotkey Released - Stopping Recording");
                        window.set_visible(false);
                        let _ = audio_ctl_tx.send(false).await;
                    }
                }
            }
        });

        // Task 2: Handle Incoming Text and Ydotool Injection
        main_context.spawn_local(async move {
            info!("Listening for transcription results...");
            
            // Listen for final text transcriptions and inject them using ydotool
            while let Some(text) = text_rx.recv().await {
                info!("Injecting text: {}", text);
                
                let output = Command::new("ydotool")
                    .arg("type")
                    .arg(&text)
                    .output();
                
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
    });

    app.run();
    Ok(())
}

