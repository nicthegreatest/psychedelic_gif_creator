//! Worker side of the live preview: a decoded-source cache and a renderer
//! cache shared with blocking jobs, so moving the frame slider or pressing
//! Play does not decode the file or rebuild the tunnel again.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use gifcore::image::{Bgr8, Bgra8};
use gifcore::loader::{self, LoadError};
use gifcore::render::FrameRenderer;
use gifcore::settings::GifSettings;
use gtk::{gdk, glib};

type Source = Result<Arc<Bgra8>, LoadError>;

#[derive(Default)]
struct State {
    source: Option<(PathBuf, Source)>,
    renderer: Option<(GifSettings, f64, Arc<FrameRenderer>)>,
}

/// Shared between the main thread and preview/export jobs.
#[derive(Default)]
pub struct SourceCache(Mutex<State>);

impl SourceCache {
    fn lock(&self) -> MutexGuard<'_, State> {
        // A panicking render never holds the lock, but stay usable if one did.
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Decoded image for `path`, loading it on first use (v2 `SourceCache::get`).
    pub fn get(&self, path: &Path) -> Source {
        let mut st = self.lock();
        if let Some((p, src)) = &st.source {
            if p == path {
                return src.clone();
            }
        }
        let src = loader::load_image(path).map(Arc::new);
        st.source = Some((path.to_path_buf(), src.clone()));
        st.renderer = None;
        src
    }

    /// Renders preview frame `frame` at `scale`, reusing the renderer when the
    /// settings and scale are unchanged.
    pub fn render(&self, s: &GifSettings, scale: f64, frame: i32) -> Result<Bgr8, String> {
        let src = self.get(&s.image_path).map_err(|e| e.to_string())?;
        let cached = {
            let st = self.lock();
            st.renderer
                .as_ref()
                .filter(|(cs, sc, _)| cs == s && *sc == scale)
                .map(|(_, _, r)| r.clone())
        };
        let renderer = match cached {
            Some(r) => r,
            None => {
                let r = Arc::new(FrameRenderer::with_scale(s, &src, scale));
                self.lock().renderer = Some((s.clone(), scale, r.clone()));
                r
            }
        };
        Ok(renderer.render_frame(frame.clamp(0, renderer.frame_count() - 1)))
    }
}

/// Wraps a BGR frame as a GDK texture without converting pixels.
pub fn texture(frame: Bgr8) -> gdk::MemoryTexture {
    let (w, h) = (frame.width, frame.height);
    let bytes = glib::Bytes::from_owned(frame.data);
    gdk::MemoryTexture::new(
        w as i32,
        h as i32,
        gdk::MemoryFormat::B8g8r8,
        &bytes,
        w as usize * 3,
    )
}

/// Text of a caught panic payload.
pub fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown error".to_string())
}
