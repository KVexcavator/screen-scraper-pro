#[cfg(target_os = "linux")]
fn run_backend(){
  println!("LINUX RUN");
  linux_backend::run();
}

#[cfg(target_os = "windows")]
fn run_backend(){
  println!("WINDOWS RUN");
  windows_backend::run();
}

fn main(){
  run_backend();
}