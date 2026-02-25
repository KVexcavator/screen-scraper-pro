slint::include_modules!();
use slint::{SharedString, VecModel, ModelRc};
use std::rc::Rc;

pub struct UiHandle {
    pub app: AppWindow,
    model: Rc<VecModel<SharedString>>,
}

impl UiHandle {
    pub fn new() -> Result<Self, slint::PlatformError> {
        let app = AppWindow::new()?;

        let model = Rc::new(VecModel::from(Vec::<SharedString>::new()));
        app.set_titles(ModelRc::from(model.clone()));

        Ok(Self { app, model })
    }

    pub fn set_titles(&self, titles: Vec<String>) {
        let data: Vec<SharedString> = titles
            .into_iter()
            .map(SharedString::from)
            .collect();

        self.model.set_vec(data);
    }
}