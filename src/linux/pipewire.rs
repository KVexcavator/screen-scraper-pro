use ashpd::desktop::{
    screencast::{CursorMode, Screencast, SourceType},
    PersistMode,
};

pub async fn get_wayland_portal() -> ashpd::Result<()> {
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

    println!("Portal response streams: {:#?}", response.streams());

    Ok(())
}
