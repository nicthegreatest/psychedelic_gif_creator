//! Headless export benchmark with v2 `gif_bench`'s arguments.
//!
//! usage: bench <image> <out.gif> [none|radial6] [threads] [WxH] [fit|fill] [dither]
//!
//! `cargo run -p gifcore --release --example bench -- in.png /tmp/out.gif`

use std::ops::ControlFlow;
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use gifcore::export::{export_gif, ExportOptions};
use gifcore::loader::load_image;
use gifcore::render::FrameRenderer;
use gifcore::settings::{FitMode, GifSettings, KaleidoscopeMode};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "usage: {} <image> <out.gif> [none|radial6] [threads] [WxH] [fit|fill] [dither]",
            args[0]
        );
        return ExitCode::from(2);
    }
    let mut s = GifSettings {
        image_path: args[1].clone().into(),
        ..GifSettings::default()
    };
    if args.get(3).is_some_and(|a| a == "radial6") {
        s.kaleidoscope_mode = KaleidoscopeMode::Radial6;
    }
    let threads = args.get(4).and_then(|a| a.parse().ok()).unwrap_or(0);
    for a in args.iter().skip(5) {
        if let Some((w, h)) = a
            .split_once('x')
            .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
        {
            s.output_width = w;
            s.output_height = h;
        } else if a == "fit" {
            s.fit_mode = FitMode::Fit;
        } else if a == "fill" {
            s.fit_mode = FitMode::Fill;
        } else if a == "dither" {
            s.dither = true;
        }
    }
    gifcore::schema::clamp(&mut s);

    let source = match load_image(Path::new(&args[1])) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("FAILED: {e}");
            return ExitCode::FAILURE;
        }
    };

    let t = Instant::now();
    let renderer = FrameRenderer::new(&s, &source);
    for i in 0..renderer.frame_count() {
        let _ = renderer.render_frame(i);
    }
    println!(
        "render only, 1 thread: {} ms for {} frames",
        t.elapsed().as_millis(),
        renderer.frame_count()
    );

    let t = Instant::now();
    let renderer = FrameRenderer::new(&s, &source);
    let opts = ExportOptions {
        threads,
        ..ExportOptions::default()
    };
    match export_gif(&renderer, &opts, &args[2], |_, _| ControlFlow::Continue(())) {
        Ok(report) => {
            println!(
                "ok: {} ms, {} bytes ({})",
                t.elapsed().as_millis(),
                report.bytes,
                report.path.display()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            println!("FAILED: {} ms ({e})", t.elapsed().as_millis());
            ExitCode::FAILURE
        }
    }
}
