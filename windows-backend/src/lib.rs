#![cfg(target_os = "windows")]
mod catcher;
use crate::catcher::get_titles;
use screen_ui::run_app;
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running Windows backend");

    let titles = get_titles();

    for title in &titles {
        println!("{}", title);
    }
    // WinAPI capture тут
    run_app(&titles)?;
    Ok(())
}
