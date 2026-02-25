use screen_ui::*;
use screen_ui::UiHandle;
use windows_backend::catcher::get_titles;
use slint::SharedString;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let ui = UiHandle::new()?;

    let windows_cache =
        std::rc::Rc::new(std::cell::RefCell::new(Vec::<SharedString>::new()));

    {
        let cache = windows_cache.clone();
        let ui_ref = ui.app.as_weak();

        ui.app.on_request_titles(move || {
            let titles = get_titles();

            let titles: Vec<SharedString> =
                titles.into_iter().map(SharedString::from).collect();

            *cache.borrow_mut() = titles.clone();

            if let Some(app) = ui_ref.upgrade() {
                app.set_titles((&titles[..]).into());
            }
        });
    }

    {
        let cache = windows_cache.clone();
        ui.app.on_window_selected(move |index| {
            let windows = cache.borrow();
            if let Some(selected) = windows.get(index as usize) {
                println!("Selected title: {}", selected);
            }
        });
    }

    ui.app.run()?;
    Ok(())
}