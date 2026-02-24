#![cfg(target_os = "windows")]
use windows::{
    Win32::{
        Foundation::{BOOL, HWND, LPARAM},
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible,
        },
    },
};

pub fn get_titles() -> Vec<String> {
    let mut titles = Vec::new();

    unsafe {
        EnumWindows(
            Some(enum_windows_proc),
            LPARAM(&mut titles as *mut _ as isize),
        ).unwrap();
    }

    titles
}

extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let titles = &mut *(lparam.0 as *mut Vec<String>);

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
            titles.push(title);
        }

        BOOL(1)
    }
}