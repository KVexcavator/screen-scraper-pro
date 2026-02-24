slint::include_modules!();
use slint::{SharedString, VecModel, ModelRc};
use std::rc::Rc;

pub fn run_app(titles: &Vec<String>) -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;

    let data: Vec<SharedString> = titles
        .iter()
        .map(|s| SharedString::from(s.as_str()))
        .collect();

    let model = Rc::new(VecModel::from(data));

    app.set_titles(ModelRc::from(model));

    app.run()
}