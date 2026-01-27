use std::collections::HashMap;
use zbus::zvariant::{OwnedValue, Value};
use zbus::{Connection, Proxy};
use uuid::Uuid;

pub async fn get_portal() -> zbus::Result<()> {
    // 1. connect to session bus
    let connection = Connection::session().await?;
    // 2. Proxy to ScreenCast portal
    let proxy = Proxy::new(
        &connection,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.ScreenCast",
    )
    .await?;
    
    // 3. CreateSession
    let token = format!("scraper_{}", Uuid::new_v4());
    let options: HashMap<&str, Value> = [
        ("handle_token", Value::from(token.as_str())),
        ("parent_window", Value::from("x11:0")),
    ]
    .into_iter()
    .collect();

    let session_handle: zbus::zvariant::OwnedObjectPath =
        proxy.call("CreateSession", &(options)).await?;

    // проблема до сюда программа не доходит
    // возможно не докрутил parent_window

    println!("Session created: {}", session_handle);
    // 4. SelectSources - here dialog window    
    let mut select_opts: HashMap<&str, Value> = HashMap::new();
    select_opts.insert("types", Value::U32(2)); // window
    select_opts.insert("multiple", Value::Bool(false));

    let _: () = proxy
        .call("SelectSources", &(session_handle.clone(), select_opts))
        .await?;


    println!("Waiting for user to select window…");
    // 5. Start
    let start_opts: HashMap<&str, Value> = HashMap::new();
    let response: (u32, HashMap<String, OwnedValue>) = proxy
        .call("Start", &(session_handle, "", start_opts))
        .await?;

    println!("Portal response: {:?}", response);
    // TODO later get pipewire_node_id
    Ok(())
}
