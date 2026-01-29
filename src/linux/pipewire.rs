use ashpd::desktop::{
    screencast::{CursorMode, Screencast, SourceType},
    PersistMode,
};
use ashpd::Error;

// call window for choose display
pub async fn get_wayland_portal() -> Result<u32, Error> {
    let screencast = Screencast::new().await?;

    let session = screencast.create_session().await?;

    screencast
        .select_sources(
            &session,
            CursorMode::Metadata,
            SourceType::Window | SourceType::Monitor,
            false,
            None, // ← parent_window
            PersistMode::DoNot,
        )
        .await?;

    let response = screencast.start(&session, None).await?.response()?;
    let stream = &response.streams()[0];

    Ok(stream.pipe_wire_node_id())
}
