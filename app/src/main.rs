
fn main(){
    #[cfg(target_os = "linux")]
        {
            let _ = linux_backend::run();
        }
    #[cfg(target_os = "windows")]
        {
            let _ = windows_backend::run();
        }
}

