
fn main(){
    #[cfg(target_os = "linux")]
        {
            linux_backend::run();
        }
    #[cfg(target_os = "windows")]
        {
            windows_backend::run();
        }
}

