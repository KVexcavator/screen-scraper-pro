use screen_ui::run_app;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running Windows backend");

    // WinAPI capture тут

    run_app()?;
    Ok(())
}