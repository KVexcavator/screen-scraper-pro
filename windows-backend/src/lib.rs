#![cfg(target_os = "windows")]
use screen_ui::run_app;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running Windows backend");

    // WinAPI capture тут

    run_app()?;
    Ok(())
}