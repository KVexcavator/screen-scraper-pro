#![allow(unused)]

mod win_ui;

fn main() {
    #[cfg(target_os = "linux")]
    {
        linux_backend::run();
    }
    #[cfg(target_os = "windows")]
    {
        win_ui::run();
    }
}
