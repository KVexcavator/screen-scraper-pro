use pipewire as pw;
use pw::context::Context;
use pw::main_loop::MainLoop;
use pw::stream::{Stream, StreamFlags};
use pw::properties::properties;
use pw::spa::utils::Direction;

pub fn run_pipewire_capture(node_id: u32) -> Result<(), Box<dyn std::error::Error>> {
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

    let mut params: [&pw::spa::pod::Pod; 0] = [];

    stream.connect(
        Direction::Input,
        None,
        StreamFlags::AUTOCONNECT,
        &mut params,
    )?;

    main_loop.run();
    Ok(())
}
