use gtk4::{prelude::*, Application, ApplicationWindow, Label};
use gtk4_layer_shell::{Layer, LayerShell};

pub fn build_ui(app: &Application) -> ApplicationWindow {
    let label = Label::builder()
        .label("🎙️ Recording...")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(24)
        .margin_end(24)
        .build();

    let window = ApplicationWindow::builder()
        .application(app)
        .child(&label)
        .build();

    // Initialize Layer Shell for Wayland OSD
    window.init_layer_shell();
    
    // Set to the Overlay layer so it appears above other windows
    window.set_layer(Layer::Overlay);
    
    // Anchor to bottom center
    window.set_anchor(gtk4_layer_shell::Edge::Bottom, true);
    window.set_margin(gtk4_layer_shell::Edge::Bottom, 50);

    // Don't take keyboard focus
    window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);

    // Keep it hidden initially
    window.set_visible(false);
    
    window
}
