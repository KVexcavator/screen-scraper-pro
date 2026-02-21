fn main() {    
    #[cfg(target_os = "linux")]
    {
        check_pkg("libpipewire-0.3");
        check_pkg("libspa-0.2");
    }
}

fn check_pkg(name: &str) {
    match pkg_config::Config::new().probe(name) {
        Ok(_) => {
          println!("cargo:warning=System dependency {} is OK", name);          
        }
        Err(_) => {
            println!("cargo:warning=");
            println!("cargo:warning=System library `{}` not found!", name);
            println!("cargo:warning=Install it with:");
            println!("cargo:warning=  sudo apt install {}", apt_name(name));
            panic!("Missing system dependency: {}", name);
        }
    }
}

fn apt_name(name: &str) -> String {
    match name {
        "libpipewire-0.3" => "libpipewire-0.3-dev".to_string(),
        "libspa-0.2" => "libspa-0.2-dev".to_string(),
        _ => name.to_string(),
    }
}