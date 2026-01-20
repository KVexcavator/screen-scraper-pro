mod display;
use slint::VecModel;
use std::rc::Rc;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // catch screens
    let session_type = std::env::var("XDG_SESSION_TYPE")
        .unwrap_or_else(|_| "unknown".into());

    let session_label = match session_type.as_str() {
        "wayland" => "Wayland",
        "x11" => "X11",
        _ => "Unknown",
    };

    // catch displays
    let displays = display::get_displays()?;
    let displays = Rc::new(VecModel::from(displays));

    let app = AppWindow::new()?;
    app.set_displays(displays.into());
    app.set_session_type(session_label.into());

    app.run()?;
    Ok(())
}
