
mod utils_linux;
mod utils_windows;

fn main(){
    #[cfg(target_os = "linux")]
        {
            use crate::utils_linux::hello;

            hello();

            let _ = linux_backend::run();
        }
    #[cfg(target_os = "windows")]
        {
            use crate::utils_windows::hello;

            hello();

            let _ = windows_backend::run();
        }
}

