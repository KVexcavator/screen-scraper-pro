use pipewire as pw;
use pw::{
    context::Context,
    main_loop::MainLoop,
    properties::properties,
    spa::utils::Direction,
    stream::{Stream, StreamFlags},
};


pub fn run_pipewire(node_id: u32) -> anyhow::Result<()> {
    eprintln!("[pw] init");

    pw::init();

    let main_loop = MainLoop::new(None)?;
    let context = Context::new(&main_loop)?;
    let core = context.connect(None)?;

    let props = properties! {
        "media.type" => "Video",
        "media.category" => "Capture",
        "media.role" => "Screen",
        "target.object" => node_id.to_string(),
    };

    let stream = Stream::new(&core, "screen-capture", props)?;

    let _listener = stream
        .add_local_listener::<()>()
        .process(|stream, _| {
            while let Some(mut buffer) = stream.dequeue_buffer() {
                for data in buffer.datas_mut() {
                    let chunk = data.chunk();
                    eprintln!(
                        "[pw] frame size={} stride={} offset={}",
                        chunk.size(),
                        chunk.stride(),
                        chunk.offset(),
                    );
                }
            }
        });

    stream.connect(
        Direction::Input,
        None,
        StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS,
        &mut [],
    )?;

    eprintln!("[pw] running");
    main_loop.run();
    Ok(())
}
