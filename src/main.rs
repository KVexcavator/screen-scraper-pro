use game_screen_scraper::linux::pipewire::get_wayland_portal;
use game_screen_scraper::linux::pw_stream::run_pipewire_capture;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO если Linux то пошли проверять xdg
    // TODO если Windows то идем работать с WinAPI

    // catch screens
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".into());
    let node_id = if session_type == "wayland" {
        Some(get_wayland_portal().await?)
    } else {
        None
    };
    // if let Some(node_id) = node_id {
    //     println!("Using PipeWire node {}", node_id);
    // } else {
    //     println!("No PipeWire node selected");
    // }

    if let Some(node_id) = node_id {
        println!("Using PipeWire node {}", node_id);

        // Запускаем PipeWire capture в отдельном Tokio блокирующем потоке
        let _handle = tokio::task::spawn_blocking(move || {
            run_pipewire_capture(node_id).unwrap();
        });
    } else {
        println!("No PipeWire node selected");
    }

    let app = AppWindow::new()?;

    app.run()?;
    Ok(())
}
