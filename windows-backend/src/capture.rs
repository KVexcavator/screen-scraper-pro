use std::sync::mpsc::Sender;
use windows::Win32::Foundation::HWND;

pub fn start_capture(
    hwnd: HWND,
    sender: Sender<(u32, u32, Vec<u8>)>,
) {
    std::thread::spawn(move || {
        // здесь будет windows-capture
        // пока тест: фейковый кадр

        let width = 800;
        let height = 600;
        let data = vec![255u8; (width * height * 4) as usize];

        loop {
            sender.send((width, height, data.clone())).ok();
            std::thread::sleep(std::time::Duration::from_millis(33));
        }
    });
}