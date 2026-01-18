use scrap::Display;
use slint::SharedString;

pub fn get_displays() -> Result<Vec<SharedString>, Box<dyn std::error::Error>> {
    let displays = Display::all()?;

    let displays = displays
        .iter()
        .enumerate()
        .map(|(i, d)| {
            format!("Display {} ::: {}x{}", i, d.width(), d.height()).into()
        })
        .collect();

    Ok(displays)
}
