// mod portal;
// mod pipewire;

// use std::sync::mpsc;
// use std::thread;

// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     let (tx, rx) = mpsc::channel();

//     // Portal thread (Tokio)
//     thread::spawn(move || {
//         let rt = tokio::runtime::Runtime::new().unwrap();
//         rt.block_on(portal::run_portal(tx)).unwrap();
//     });

//     // ждём node_id
//     let node_id = rx.recv().unwrap();
//     eprintln!("[main] got node_id={node_id}");

//     // PipeWire thread (не Tokio)
//     thread::spawn(move || {
//         pipewire::run_pipewire(node_id).unwrap();
//     });

//     // 🔹 UI / app loop / просто спим
//     loop {
//         std::thread::park();
//     }
// }

// linux-backend/src/lib.rs

mod portal;
mod pipewire;

use std::sync::mpsc;
use std::thread;

pub fn run() {
    let (tx, rx) = mpsc::channel();

    // Portal thread (Tokio)
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(portal::run_portal(tx)).unwrap();
    });

    let node_id = rx.recv().unwrap();
    eprintln!("[linux-backend] got node_id={node_id}");

    thread::spawn(move || {
        pipewire::run_pipewire(node_id).unwrap();
    });

    loop {
        std::thread::park();
    }
}