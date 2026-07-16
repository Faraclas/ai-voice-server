use gtk4::{prelude::*, Application, ApplicationWindow, Label};
use gtk4_layer_shell::{Layer, LayerShell};

pub fn build_ui(app: &Application) -> (ApplicationWindow, Label) {
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

    // Initialize Layer Shell for Wayland OSD if supported by the compositor
    if gtk4_layer_shell::is_supported() {
        window.init_layer_shell();
        
        // Set to the Overlay layer so it appears above other windows
        window.set_layer(Layer::Overlay);
        
        // Anchor to bottom center
        window.set_anchor(gtk4_layer_shell::Edge::Bottom, true);
        window.set_margin(gtk4_layer_shell::Edge::Bottom, 50);

        // Don't take keyboard focus
        window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);
    } else {
        log::info!("Layer Shell is not supported by your Wayland compositor (e.g. GNOME). The OSD will appear as a standard floating window.");
    }

    // Keep it hidden initially
    window.set_visible(false);
    
    (window, label)
}
