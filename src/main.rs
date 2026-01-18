use scrap::Display;
use slint::{SharedString, VecModel};
use std::rc::Rc;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let displays = Display::all()?;

    let targets: Vec<SharedString> = displays
        .iter()
        .enumerate()
        .map(|(i, d)| {
            format!("Display {} ::: {}x{}", i, d.width(), d.height()).into()
        })
        .collect();

    // 👇 КЛЮЧЕВОЙ МОМЕНТ
    let model = Rc::new(VecModel::from(targets));

    let app = AppWindow::new()?;

    app.set_targets(model.into());

    app.run()?;
    Ok(())
}
