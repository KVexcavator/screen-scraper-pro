mod display;
use slint::VecModel;
use std::rc::Rc;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // catch screens
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();

    match session_type.as_str() {
        "wayland" => println!("Wayland detected"),
        "x11" => println!("X11 detected"),
        _ => println!("Unknown session"),
    }

    // catch displays
    let displays = display::get_displays()?;
    let displays = Rc::new(VecModel::from(displays));

    let app = AppWindow::new()?;
    app.set_displays(displays.into());

    app.run()?;
    Ok(())
}
