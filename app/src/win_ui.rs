use screen_ui::run_app;
use windows_backend::catcher::get_titles;
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