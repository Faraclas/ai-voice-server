use ksni::{Tray, MenuItem, menu::*};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct AppTray {
    pub status: Arc<RwLock<String>>,
    pub active_device: Arc<RwLock<String>>,
    pub loaded_model: Arc<RwLock<String>>,
    pub output_mode: Arc<RwLock<String>>,
    pub server_url: String,
    pub admin_key: Arc<RwLock<Option<String>>>,
    pub restart_tx: mpsc::Sender<()>,
    pub model_swap_tx: mpsc::Sender<String>,
    pub test_conn_tx: mpsc::Sender<()>,
    pub toggle_mode_tx: mpsc::Sender<()>,
}

impl Tray for AppTray {
    fn id(&self) -> String {
        "ai-voice-client".to_string()
    }

    fn icon_name(&self) -> String {
        let status = self.status.read().unwrap_or_else(|e| e.into_inner());
        match status.as_str() {
            "Recording" => "media-record".to_string(),
            "WS_Open" | "Processing" => "dialog-warning".to_string(),
            "Disconnected" | "Offline" => "dialog-error".to_string(),
            _ => "microphone-sensitivity-high-symbolic".to_string(),
        }
    }

    fn title(&self) -> String {
        "AI Voice Client".to_string()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        let status = self.status.read().unwrap_or_else(|e| e.into_inner());
        let device = self.active_device.read().unwrap_or_else(|e| e.into_inner());
        let model = self.loaded_model.read().unwrap_or_else(|e| e.into_inner());
        ksni::ToolTip {
            title: format!("AI Voice Client ({})", status),
            description: format!("Compute: {}\nModel: {}", device.to_uppercase(), model),
            icon_name: self.icon_name(),
            icon_pixmap: Vec::new(),
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let mode = self.output_mode.read().unwrap_or_else(|e| e.into_inner()).clone();
        let device = self.active_device.read().unwrap_or_else(|e| e.into_inner()).clone();
        let model = self.loaded_model.read().unwrap_or_else(|e| e.into_inner()).clone();
        let status = self.status.read().unwrap_or_else(|e| e.into_inner()).clone();

        let is_type = mode.to_lowercase() == "type";
        let is_clip = mode.to_lowercase() == "clipboard";

        vec![
            StandardItem {
                label: format!("Status: {}", status),
                enabled: false,
                ..Default::default()
            }.into(),
            StandardItem {
                label: format!("Compute: {}", device.to_uppercase()),
                enabled: false,
                ..Default::default()
            }.into(),
            StandardItem {
                label: format!("Model: {}", model),
                enabled: false,
                ..Default::default()
            }.into(),
            MenuItem::Separator,
            StandardItem {
                label: format!("{} Auto-Typing (ydotool)", if is_type { "●" } else { "○" }),
                activate: Box::new(|this: &mut AppTray| {
                    let _ = this.toggle_mode_tx.try_send(());
                }),
                ..Default::default()
            }.into(),
            StandardItem {
                label: format!("{} Clipboard Copy (wl-copy)", if is_clip { "●" } else { "○" }),
                activate: Box::new(|this: &mut AppTray| {
                    let _ = this.toggle_mode_tx.try_send(());
                }),
                ..Default::default()
            }.into(),
            MenuItem::Separator,
            SubMenu {
                label: "Remote Model Hot-Swap".to_string(),
                submenu: vec![
                    StandardItem {
                        label: "small.en".to_string(),
                        activate: Box::new(|this: &mut AppTray| {
                            let _ = this.model_swap_tx.try_send("small.en".to_string());
                        }),
                        ..Default::default()
                    }.into(),
                    StandardItem {
                        label: "medium.en".to_string(),
                        activate: Box::new(|this: &mut AppTray| {
                            let _ = this.model_swap_tx.try_send("medium.en".to_string());
                        }),
                        ..Default::default()
                    }.into(),
                    StandardItem {
                        label: "large-v3".to_string(),
                        activate: Box::new(|this: &mut AppTray| {
                            let _ = this.model_swap_tx.try_send("large-v3".to_string());
                        }),
                        ..Default::default()
                    }.into(),
                ],
                ..Default::default()
            }.into(),
            MenuItem::Separator,
            StandardItem {
                label: "🌐 Open Server Web Page".to_string(),
                activate: Box::new(|this: &mut AppTray| {
                    let url = this.server_url.clone();
                    let http_url = url
                        .replace("ws://", "http://")
                        .replace("wss://", "https://")
                        .replace("/stream", "/admin");
                    let _ = open::that(http_url);
                }),
                ..Default::default()
            }.into(),
            StandardItem {
                label: "⚡ Restart Remote Server Service".to_string(),
                activate: Box::new(|this: &mut AppTray| {
                    let _ = this.restart_tx.try_send(());
                }),
                ..Default::default()
            }.into(),
            StandardItem {
                label: "🔄 Restart Local Client Service".to_string(),
                activate: Box::new(|_| {
                    let _ = std::process::Command::new("systemctl")
                        .arg("--user")
                        .arg("restart")
                        .arg("ai-voice-client")
                        .spawn();
                }),
                ..Default::default()
            }.into(),
            StandardItem {
                label: "⚙️ Edit Client Config".to_string(),
                activate: Box::new(|_| {
                    if let Some(home) = dirs::config_dir() {
                        let path1 = home.join("ai-voice-server/client.env");
                        let path2 = home.join("ai-voice/client.conf");
                        let path3 = home.join("ai-voice/client.env");
                        if path1.exists() {
                            let _ = open::that(path1);
                        } else if path2.exists() {
                            let _ = open::that(path2);
                        } else if path3.exists() {
                            let _ = open::that(path3);
                        } else {
                            let _ = open::that(path1);
                        }
                    }
                }),
                ..Default::default()
            }.into(),
            StandardItem {
                label: "🔄 Test Connection".to_string(),
                activate: Box::new(|this: &mut AppTray| {
                    let _ = this.test_conn_tx.try_send(());
                }),
                ..Default::default()
            }.into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit Client".to_string(),
                activate: Box::new(|_| {
                    std::process::exit(0);
                }),
                ..Default::default()
            }.into(),
        ]
    }
}
