slint::include_modules!();

pub fn run_app() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;
    app.run()
}