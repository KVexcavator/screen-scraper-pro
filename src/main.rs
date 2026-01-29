use game_screen_scraper::linux::pipewire::get_wayland_portal;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO если Linux то пошли проверять xdg
    // TODO если Windows то идем работать с WinAPI

    // catch screens
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".into());
    if session_type == "wayland"
        && let Err(e) = get_wayland_portal().await
    {
        eprintln!("Portal error: {e}");
    }

    let app = AppWindow::new()?;

    app.run()?;
    Ok(())
}
