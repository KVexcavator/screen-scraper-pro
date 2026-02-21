use ashpd::desktop::{
    screencast::{CursorMode, Screencast, SourceType},
    PersistMode,
};

pub async fn run_portal(tx: std::sync::mpsc::Sender<u32>) -> ashpd::Result<()> {
    let screencast = Screencast::new().await?;
    let session = screencast.create_session().await?;

    screencast
        .select_sources(
            &session,
            CursorMode::Metadata,
            SourceType::Monitor | SourceType::Window,
            false,
            None,
            PersistMode::DoNot,
        )
        .await?;

    let response = screencast.start(&session, None).await?.response()?;
    let stream = &response.streams()[0];

    let node_id = stream.pipe_wire_node_id();
    eprintln!("[portal] node_id={node_id}");

    // отдаем node_id, но не выходим
    tx.send(node_id).unwrap();

    // держим session живой
    futures_util::future::pending::<()>().await;
    Ok(())
}
