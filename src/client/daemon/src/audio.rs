use anyhow::Result;
use log::{info, error};
use tokio::sync::mpsc;
use tokio::process::Command;
use std::process::Stdio;
use tokio::io::AsyncReadExt;

/// Starts the Pipewire audio capture loop.
pub fn start_audio_capture(
    mut control_rx: mpsc::Receiver<bool>,
    audio_tx: mpsc::Sender<Vec<u8>>,
) -> Result<()> {
    tokio::spawn(async move {
        info!("Audio capture controller started (pw-record backend).");
        
        let mut child_process: Option<tokio::process::Child> = None;

        while let Some(start) = control_rx.recv().await {
            if start {
                info!("Microphone unmuted, starting pw-record...");
                
                // Spawn pw-record to capture 16kHz 16-bit PCM audio
                let mut child = Command::new("pw-record")
                    .arg("--format").arg("s16")
                    .arg("--rate").arg("16000")
                    .arg("--channels").arg("1")
                    .arg("-") // output to stdout
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .spawn()
                    .expect("Failed to spawn pw-record");

                let mut stdout = child.stdout.take().expect("Failed to open stdout");
                let tx = audio_tx.clone();
                
                // Read stdout in chunks and send to network
                tokio::spawn(async move {
                    let mut buffer = [0u8; 4096]; // 4KB chunks
                    while let Ok(n) = stdout.read(&mut buffer).await {
                        if n == 0 { break; }
                        let _ = tx.send(buffer[..n].to_vec()).await;
                    }
                });

                child_process = Some(child);
            } else {
                info!("Microphone muted, killing pw-record...");
                
                // Kill the recording process
                if let Some(mut child) = child_process.take() {
                    let _ = child.kill().await;
                }
                
                // Send an empty chunk to signal end-of-stream to the network layer
                let _ = audio_tx.send(vec![]).await;
            }
        }
    });

    Ok(())
}
