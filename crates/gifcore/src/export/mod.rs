//! GIF export engine (port of v2 `gif_worker.cpp` and its vendored gif-h).
//!
//! Frames are rendered and given a palette in parallel on scoped threads, then
//! mapped (delta transparency needs the previous output frame) and LZW-coded
//! strictly in order. The output bytes depend only on the renderer, the
//! settings and [`ExportOptions::delays`], never on the thread count.
//!
//! The file is written to a hidden temp file next to the target and renamed
//! into place only on success; on cancel or error the temp file is removed.

mod delay;
pub mod palette;
pub mod quantize;
mod writer;

use std::collections::BTreeMap;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::ops::ControlFlow;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Mutex};
use std::time::{Duration, Instant};

pub use delay::{frame_delays, DelayMode, MIN_DELAY_CS};
pub use palette::Palette;

use crate::image::Bgr8;
use crate::render::FrameRenderer;

/// Export knobs that are not part of [`crate::settings::GifSettings`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExportOptions {
    /// Render threads; 0 means one per CPU core (as v2).
    pub threads: usize,
    /// How frame delays are derived from `fps`.
    pub delays: DelayMode,
}

/// What a successful export produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportReport {
    pub path: PathBuf,
    pub frames: usize,
    pub bytes: u64,
    /// Sum of all frame delays: one loop's play time in 1/100 s.
    pub total_delay_cs: u32,
    pub elapsed: Duration,
}

/// Why an export stopped. `Display` gives v2's messages.
#[derive(Debug)]
pub enum ExportError {
    /// The progress callback returned `ControlFlow::Break`.
    Cancelled,
    /// The output (temp) file could not be created.
    Create(io::Error),
    /// Writing frame data failed.
    Write(io::Error),
    /// Flushing or renaming the finished file failed.
    Finish(io::Error),
    /// Rendering a frame panicked. `frame` is 1-based, as v2 reports it.
    Render { frame: usize, message: String },
    /// The frame is wider or taller than a GIF can store (65535).
    TooLarge { width: u32, height: u32 },
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => f.write_str("GIF generation cancelled."),
            Self::Create(_) => f.write_str("Could not open the output file for writing."),
            Self::Write(_) => f.write_str("Failed to write to the GIF file (disk full?)."),
            Self::Finish(_) => f.write_str("Failed to finish writing the GIF file."),
            Self::Render { frame, message } => {
                write!(f, "Rendering frame {frame} failed: {message}")
            }
            Self::TooLarge { width, height } => {
                write!(
                    f,
                    "Output size {width}×{height} is larger than a GIF allows."
                )
            }
        }
    }
}

impl std::error::Error for ExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Create(e) | Self::Write(e) | Self::Finish(e) => Some(e),
            _ => None,
        }
    }
}

/// Streaming GIF encoder with v2's (gif-h) palette, mapping and LZW.
///
/// Every frame covers the whole canvas, is never disposed, and marks pixels
/// unchanged from the previous output frame with transparent index 0.
pub struct Encoder<W: Write> {
    out: W,
    width: u16,
    height: u16,
    dither: bool,
    prev: Option<Vec<[u8; 3]>>,
}

impl<W: Write> Encoder<W> {
    /// Writes the GIF header (infinite loop).
    pub fn new(mut out: W, width: u16, height: u16, dither: bool) -> io::Result<Self> {
        writer::write_header(&mut out, width, height)?;
        Ok(Self {
            out,
            width,
            height,
            dither,
            prev: None,
        })
    }

    /// Adds a frame of `width * height` RGB pixels, building its palette.
    pub fn add_frame(&mut self, rgb: &[[u8; 3]], delay_cs: u16) -> io::Result<()> {
        let pal = Palette::build(rgb, self.dither);
        self.add_frame_with_palette(rgb, &pal, delay_cs)
    }

    /// Adds a frame whose palette was built with
    /// `Palette::build(rgb, dither)` (lets callers build palettes in parallel).
    pub fn add_frame_with_palette(
        &mut self,
        rgb: &[[u8; 3]],
        pal: &Palette,
        delay_cs: u16,
    ) -> io::Result<()> {
        let expected = usize::from(self.width) * usize::from(self.height);
        if rgb.len() != expected {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "frame size does not match the GIF",
            ));
        }
        let prev = self.prev.as_deref();
        let mapped = if self.dither {
            quantize::dither(prev, rgb, usize::from(self.width), pal)
        } else {
            quantize::threshold(prev, rgb, pal)
        };
        writer::write_frame(
            &mut self.out,
            self.width,
            self.height,
            &mapped.indices,
            pal,
            delay_cs,
        )?;
        self.prev = Some(mapped.shown);
        Ok(())
    }

    /// Writes the trailer and returns the writer.
    pub fn finish(mut self) -> io::Result<W> {
        writer::write_trailer(&mut self.out)?;
        Ok(self.out)
    }
}

/// BGR frame to RGB triplets (v2's `COLOR_BGR2RGBA`, alpha dropped).
pub fn bgr_to_rgb(frame: &Bgr8) -> Vec<[u8; 3]> {
    frame
        .data
        .as_chunks::<3>()
        .0
        .iter()
        .map(|&[b, g, r]| [r, g, b])
        .collect()
}

/// Renders every frame of `renderer` and writes an infinitely looping GIF to
/// `path`.
///
/// `progress(done, total)` is called after each frame is written; returning
/// `ControlFlow::Break(())` cancels with [`ExportError::Cancelled`]. Nothing is
/// left at `path` (or beside it) unless the export succeeds.
pub fn export_gif(
    renderer: &FrameRenderer,
    opts: &ExportOptions,
    path: impl AsRef<Path>,
    mut progress: impl FnMut(usize, usize) -> ControlFlow<()>,
) -> Result<ExportReport, ExportError> {
    let start = Instant::now();
    let path = path.as_ref();
    let (w, h) = renderer.frame_size();
    let (width, height) = match (u16::try_from(w), u16::try_from(h)) {
        (Ok(a), Ok(b)) => (a, b),
        _ => {
            return Err(ExportError::TooLarge {
                width: w,
                height: h,
            })
        }
    };
    let settings = renderer.settings();
    let frames = usize::try_from(renderer.frame_count().max(1)).unwrap_or(1);
    let delays = frame_delays(settings.fps, frames, opts.delays);
    let dither = settings.dither;

    let tmp = temp_path(path);
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .map_err(ExportError::Create)?;
    let result = (|| {
        let mut enc = Encoder::new(BufWriter::new(file), width, height, dither)
            .map_err(ExportError::Write)?;
        let threads = match opts.threads {
            0 => std::thread::available_parallelism().map_or(1, |n| n.get()),
            n => n,
        };
        encode_parallel(renderer, &mut enc, &delays, dither, threads, &mut progress)?;
        let file = enc
            .finish()
            .map_err(ExportError::Finish)?
            .into_inner()
            .map_err(|e| ExportError::Finish(e.into_error()))?;
        file.sync_all().map_err(ExportError::Finish)?;
        drop(file);
        fs::rename(&tmp, path).map_err(ExportError::Finish)
    })();
    if let Err(e) = result {
        // Only ever the temp file this call created with `create_new`.
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    let bytes = fs::metadata(path).map_err(ExportError::Finish)?.len();
    Ok(ExportReport {
        path: path.to_path_buf(),
        frames,
        bytes,
        total_delay_cs: delays.iter().map(|&d| u32::from(d)).sum(),
        elapsed: start.elapsed(),
    })
}

/// `dir/.name.<pid>.part` beside the target, so the final rename stays on
/// one file system.
fn temp_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map_or_else(|| "export.gif".into(), |n| n.to_string_lossy());
    path.with_file_name(format!(".{name}.{}.part", std::process::id()))
}

type FrameJob = Result<(Vec<[u8; 3]>, Palette), String>;

/// Renders and builds palettes on `threads` workers at most `2 * threads`
/// frames ahead of the writer, and encodes the results in frame order.
fn encode_parallel<W: Write>(
    renderer: &FrameRenderer,
    enc: &mut Encoder<W>,
    delays: &[u16],
    dither: bool,
    threads: usize,
    progress: &mut impl FnMut(usize, usize) -> ControlFlow<()>,
) -> Result<(), ExportError> {
    let total = delays.len();
    let window = threads * 2;
    let (job_tx, job_rx) = mpsc::channel::<usize>();
    let job_rx = Mutex::new(job_rx);
    let (done_tx, done_rx) = mpsc::channel::<(usize, FrameJob)>();

    std::thread::scope(|scope| {
        for _ in 0..threads {
            let done_tx = done_tx.clone();
            let job_rx = &job_rx;
            scope.spawn(move || loop {
                let next = job_rx.lock().map(|rx| rx.recv());
                let Ok(Ok(index)) = next else { break };
                let job = panic::catch_unwind(AssertUnwindSafe(|| {
                    let rgb = bgr_to_rgb(&renderer.render_frame(index as i32));
                    let pal = Palette::build(&rgb, dither);
                    (rgb, pal)
                }))
                .map_err(panic_message);
                if done_tx.send((index, job)).is_err() {
                    break;
                }
            });
        }
        drop(done_tx);

        let mut submitted = 0;
        let mut ready: BTreeMap<usize, FrameJob> = BTreeMap::new();
        let result = (|| {
            for (i, &delay) in delays.iter().enumerate() {
                while submitted < total && submitted < i + window {
                    // Workers outlive this loop, so the send cannot fail.
                    let _ = job_tx.send(submitted);
                    submitted += 1;
                }
                let job = loop {
                    if let Some(job) = ready.remove(&i) {
                        break job;
                    }
                    match done_rx.recv() {
                        Ok((index, job)) => {
                            ready.insert(index, job);
                        }
                        Err(_) => {
                            break Err("render worker stopped".to_string());
                        }
                    }
                };
                let (rgb, pal) = job.map_err(|message| ExportError::Render {
                    frame: i + 1,
                    message,
                })?;
                enc.add_frame_with_palette(&rgb, &pal, delay)
                    .map_err(ExportError::Write)?;
                if progress(i + 1, total).is_break() {
                    return Err(ExportError::Cancelled);
                }
            }
            Ok(())
        })();
        // Closing the job queue lets idle workers exit; busy ones finish their
        // frame and are joined when the scope ends.
        drop(job_tx);
        drop(done_rx);
        result
    })
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown error".to_string())
}
