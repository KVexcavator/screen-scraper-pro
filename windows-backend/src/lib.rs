// #![cfg(target_os = "windows")]
// use screen_ui::run_app;
// pub fn run() -> Result<(), Box<dyn std::error::Error>> {
//     println!("Running Windows backend");
//     // WinAPI capture тут
//     run_app()?;
//     Ok(())
// }
use windows::{
    Win32::{
        Foundation::{BOOL, HWND, LPARAM},
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible,
        },
    },
};

pub fn run() {
    unsafe {
        EnumWindows(Some(enum_windows_proc), LPARAM(0)).unwrap();
    }
}

extern "system" fn enum_windows_proc(hwnd: HWND, _: LPARAM) -> BOOL {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }

        let length = GetWindowTextLengthW(hwnd);
        if length == 0 {
            return BOOL(1);
        }

        let mut buffer = vec![0u16; (length + 1) as usize];

        let read = GetWindowTextW(hwnd, &mut buffer);
        if read == 0 {
            return BOOL(1);
        }

        let title = String::from_utf16_lossy(&buffer[..read as usize]);

        if !title.is_empty() {
            println!("{}", title);
        }

        BOOL(1)
    }
}