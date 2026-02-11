// это main первого варианта, пока в архиве
use std::process::Command;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO если Linux то пошли проверять xdg
    // TODO если Windows то идем работать с WinAPI

    let _session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".into());

    let output = Command::new("./target/debug/portal-helper")
        .output()?;

    if !output.status.success() {
        return Err("portal-helper failed".into());
    }

    let node_id: u32 = String::from_utf8(output.stdout)?
        .trim()
        .parse()?;

    println!("Using PipeWire node {node_id}");

    Command::new("target/debug/pipewire-capture")
        .arg(node_id.to_string())
        .spawn()?;

    // ошибка 
    // thread '<unnamed>' (97434) panicked at /home/excavator/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/zbus-5.13.2/src/abstractions/executor.rs:190:27:
    // there is no reactor running, must be called from the context of a Tokio 1.x runtime
    // note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
    // Error watching for xdg color schemes: org.freedesktop.DBus.Error.UnknownMethod: No such method “ReadOne”

    // ошибка пропадает елси закоментировать
    // let app = AppWindow::new()?;
    // app.run()?;

    Ok(())
}


