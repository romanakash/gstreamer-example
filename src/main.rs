use anyhow::{bail, Result};
use gstreamer_video::VideoCapsBuilder;
use std::env;

use gstreamer::{
    prelude::{GstBinExtManual, ObjectExt},
    ClockTime, Element, ElementFactory, FlowError, MessageView, Pipeline, State,
};
use gstreamer_app::{
    prelude::{ElementExt, GstObjectExt, PadExt},
    AppSink, AppSinkCallbacks,
};

fn read_file_arg() -> String {
    let args: Vec<_> = env::args().collect();
    args.get(1).expect("Need file as argument").clone()
}

// Adds filesrc -> decodebin
fn add_src_decode_elements(pipeline: &Pipeline, filename: &str) -> Result<Element> {
    let src = ElementFactory::make_with_name("filesrc", Some("src_element"))?;
    src.set_property("location", filename);

    let decodebin = ElementFactory::make_with_name("decodebin", Some("decode_element"))?;

    // add elements to pipeline
    pipeline.add_many([&src, &decodebin])?;
    Element::link_many([&src, &decodebin])?;

    // Return the element so we can link it
    Ok(decodebin)
}

// Adds decodebin -> appsink
fn create_app_sink_element(pipeline: &Pipeline) -> Result<AppSink> {
    // raw video encoding and accept all formats
    let caps = VideoCapsBuilder::new().build();

    let appsink = AppSink::builder()
        .name("appsink_element")
        .caps(&caps)
        .build();

    // Emit signals so we can connect a closure to execute on every new frame
    appsink.set_property("emit-signals", true);
    // Ensure frames are processed in the right order
    appsink.set_property("sync", true);

    pipeline.add_many([&appsink])?;
    Ok(appsink)
}

fn connect_app_sink_to_frame_count(app_sink: &AppSink) -> Result<()> {
    let mut frame_count = 0;

    // Closure to execute on every sample received by appsink
    // Simply prints the index and dimensions of the frame
    let count_frames = move |appsink: &AppSink| {
        frame_count += 1;
        let sample = appsink.pull_sample().map_err(|_| FlowError::Eos)?;

        // Caps are used by elements to describe characteristics of the media type
        let caps = sample.caps().expect("Expected caps");

        let width = caps
            .structure(0)
            .unwrap()
            .value("width")
            .unwrap()
            .get::<i32>()
            .unwrap();

        let height = caps
            .structure(0)
            .unwrap()
            .value("height")
            .unwrap()
            .get::<i32>()
            .unwrap();

        // printttt
        println!("Frame {}: {}x{}", frame_count, width, height);
        Ok(gstreamer::FlowSuccess::Ok)
    };

    // Use callbacks to get data out of appsink
    app_sink.set_callbacks(
        // Build a callback to execute on new-sample signal
        AppSinkCallbacks::builder().new_sample(count_frames).build(),
    );

    Ok(())
}

fn link_app_sink_to_decode_bin(decode_bin: &Element, app_sink: AppSink) {
    // We move the ownership of app_sink into this closure
    decode_bin.connect_pad_added(move |dbin, src_pad| {
        let is_video = src_pad
            .current_caps()
            .and_then(|caps| {
                caps.structure(0).map(|s| {
                    let name = s.name();
                    name.starts_with("video/")
                })
            })
            .unwrap_or_else(|| false);

        if is_video {
            // get the sink_pad of the appsink element
            let sink_pad = app_sink
                .static_pad("sink")
                .expect("Appsink element should have sink pad");
            src_pad
                .link(&sink_pad)
                .expect("Src pad could not link with sink");
        }
    });
}

fn main() -> Result<()> {
    let file_path = read_file_arg();
    println!("File arg: {}", file_path);

    // Always remember to init gstreamer first
    gstreamer::init()?;

    let pipeline = Pipeline::default();
    let decodebin = add_src_decode_elements(&pipeline, &file_path)?;

    let appsink = create_app_sink_element(&pipeline)?;
    connect_app_sink_to_frame_count(&appsink)?;

    link_app_sink_to_decode_bin(&decodebin, appsink);

    pipeline.set_state(State::Playing)?;

    let bus = pipeline.bus().unwrap();

    for msg in bus.iter_timed(ClockTime::NONE) {
        match msg.view() {
            MessageView::Eos(..) => bail!("EOS"),
            MessageView::Error(_) => {
                pipeline.set_state(State::Null)?;
                bail!("Pipeline bus error");
            }
            MessageView::StateChanged(s) => {
                println!(
                    "State changed from {:?}: {:?} -> {:?} ({:?})",
                    s.src().map(|s| s.path_string()),
                    s.old(),
                    s.current(),
                    s.pending()
                );
            }
            _ => (),
        }
    }

    pipeline.set_state(State::Null)?;

    Ok(())
}
