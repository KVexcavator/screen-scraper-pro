use game_screen_scraper::linux::display;
use slint::VecModel;
use std::rc::Rc;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO если Linux то пошли проверять xdg
    // TODO если Windows то идем работать с WinAPI

    // catch screens
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".into());

    let session_label = match session_type.as_str() {
        "wayland" => "Wayland",
        "x11" => "X11",
        _ => "Unknown",
    };

    // TODO если wayland - тогда пытаемся работаем с zbus
    if session_type == "wayland"
        && let Err(e) = game_screen_scraper::linux::wp::get_portal().await
    {
        eprintln!("Portal error: {e}");
    }

    // TODO eсли X11 - тогда работаем с ним

    // catch displays
    let displays = display::get_displays()?;
    let displays = Rc::new(VecModel::from(displays));

    let app = AppWindow::new()?;
    app.set_displays(displays.into());
    app.set_session_type(session_label.into());

    app.run()?;
    Ok(())
}
