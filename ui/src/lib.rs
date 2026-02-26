use slint::{Image, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel};
use std::rc::Rc;

slint::include_modules!();

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
        let data: Vec<SharedString> = titles.into_iter().map(SharedString::from).collect();

        self.model.set_vec(data);
    }

    pub fn set_frame(&self, width: u32, height: u32, data: Vec<u8>) {
        let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&data, width, height);

        let image = Image::from_rgba8(buffer);
        self.app.set_preview(image);
    }
}
