//! Generate GIF: save dialog, export on a worker thread with progress and
//! Cancel, result dialogs, and the close-during-export question (port of v2
//! `startExport`/`exportFinished`/`closeEvent`).

use std::ops::ControlFlow;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use adw::prelude::*;
use gifcore::export::{export_gif, ExportError, ExportOptions};
use gifcore::render::FrameRenderer;
use gifcore::schema;
use gifcore::ui_model;
use gtk::{gio, glib};

use crate::preview;
use crate::window::Win;

impl Win {
    pub fn start_export(self: &Rc<Self>) {
        if self.phase.get().is_busy() {
            return;
        }
        let mut settings = self.settings.borrow().clone();
        if settings.image_path.as_os_str().is_empty() {
            self.alert("Input required", "Please choose an input image first.");
            return;
        }
        let w = self.clone();
        glib::spawn_future_local(async move {
            let cache = w.cache.clone();
            let path = settings.image_path.clone();
            let source = match gio::spawn_blocking(move || cache.get(&path)).await {
                Ok(Ok(img)) => img,
                Ok(Err(e)) => return w.alert("Cannot read image", &e.to_string()),
                Err(p) => return w.alert("Cannot read image", &preview::panic_text(&*p)),
            };

            let Some(out) = w.ask_output_path(&settings.image_path).await else {
                return;
            };
            *w.last_output_dir.borrow_mut() = out.parent().map(PathBuf::from);
            w.state.remember_save(&out);

            w.play.set_active(false);
            let mut phase = w.phase.get();
            if !phase.start() {
                return;
            }
            w.phase.set(phase);
            let cancel = Arc::new(AtomicBool::new(false));
            *w.cancel_flag.borrow_mut() = Some(cancel.clone());
            let done = Arc::new(AtomicUsize::new(0));
            let total = settings.num_frames.max(1) as usize;
            w.cancel_button.set_sensitive(true);
            w.progress.set_fraction(0.0);
            w.progress
                .set_text(Some(&ui_model::progress_text(0, total)));
            w.update_locks();
            w.log(&format!(
                "Generating {} ({} x {}, {} frames)…",
                out.display(),
                settings.output_width,
                settings.output_height,
                total
            ));

            // Poll progress from the main loop; the worker only touches atomics.
            let (pw, pd) = (w.clone(), done.clone());
            let timer = glib::timeout_add_local(Duration::from_millis(100), move || {
                let d = pd.load(Ordering::Relaxed);
                pw.progress
                    .set_fraction(ui_model::progress_percent(d, total) as f64 / 100.0);
                pw.progress
                    .set_text(Some(&ui_model::progress_text(d, total)));
                glib::ControlFlow::Continue
            });

            let target = out.clone();
            let result = gio::spawn_blocking(move || {
                schema::clamp(&mut settings);
                let renderer = FrameRenderer::new(&settings, &source);
                export_gif(&renderer, &ExportOptions::default(), &target, |d, _| {
                    done.store(d, Ordering::Relaxed);
                    if cancel.load(Ordering::Relaxed) {
                        ControlFlow::Break(())
                    } else {
                        ControlFlow::Continue(())
                    }
                })
            })
            .await;

            timer.remove();
            let mut phase = w.phase.get();
            phase.finish();
            w.phase.set(phase);
            w.cancel_flag.borrow_mut().take();
            w.update_locks();

            if w.quit_after_export.get() {
                w.window.close();
                return;
            }
            match result {
                Ok(Ok(report)) => {
                    w.log(&format!(
                        "Saved {} ({} KB in {:.1} s).",
                        report.path.display(),
                        report.bytes.div_ceil(1024),
                        report.elapsed.as_secs_f64()
                    ));
                    w.export_succeeded(&report.path);
                }
                Ok(Err(ExportError::Cancelled)) => w.log("GIF generation cancelled."),
                Ok(Err(e)) => {
                    w.log(&format!("GIF generation failed: {e}"));
                    w.alert("Generation failed", &e.to_string());
                }
                Err(p) => {
                    let msg = format!("Unexpected error: {}", preview::panic_text(&*p));
                    w.log(&format!("GIF generation failed: {msg}"));
                    w.alert("Generation failed", &msg);
                }
            }
        });
    }

    async fn ask_output_path(self: &Rc<Self>, input: &std::path::Path) -> Option<PathBuf> {
        let gifs = gtk::FileFilter::new();
        gifs.set_name(Some("GIF images"));
        gifs.add_suffix("gif");
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&gifs);
        let dialog = gtk::FileDialog::builder()
            .title("Save GIF")
            .modal(true)
            .filters(&filters)
            .default_filter(&gifs)
            .initial_name(ui_model::suggested_output_name(input))
            .build();
        let dir = self
            .last_output_dir
            .borrow()
            .clone()
            .or_else(|| input.parent().map(PathBuf::from))
            .and_then(|d| ui_model::restored_dir(&d.to_string_lossy()));
        if let Some(dir) = dir {
            dialog.set_initial_folder(Some(&gio::File::for_path(dir)));
        }
        let file = dialog.save_future(Some(&self.window)).await.ok()?;
        match file.path() {
            Some(p) => Some(ui_model::ensure_gif_extension(p)),
            None => {
                self.alert("Cannot save here", "Please choose a local folder.");
                None
            }
        }
    }

    fn export_succeeded(self: &Rc<Self>, path: &std::path::Path) {
        let shown = path.display().to_string();
        let d = adw::AlertDialog::new(
            Some("GIF generation complete."),
            Some(&format!("Saved to:\n{shown}")),
        );
        d.add_responses(&[("show", "Show in folder"), ("ok", "OK")]);
        d.set_default_response(Some("ok"));
        d.set_close_response("ok");
        let w = self.clone();
        let file = gio::File::for_path(path);
        d.choose(Some(&self.window), None::<&gio::Cancellable>, move |r| {
            if r == "show" {
                gtk::FileLauncher::new(Some(&file)).open_containing_folder(
                    Some(&w.window),
                    None::<&gio::Cancellable>,
                    |_| {},
                );
            }
        });
    }

    pub fn cancel_export(&self) {
        let mut phase = self.phase.get();
        if phase.cancel() {
            self.phase.set(phase);
            if let Some(flag) = self.cancel_flag.borrow().as_ref() {
                flag.store(true, Ordering::Relaxed);
            }
            self.cancel_button.set_sensitive(false);
            self.update_status();
        }
    }

    /// Close button: ask first while a GIF is being generated.
    pub fn close_requested(self: &Rc<Self>) -> glib::Propagation {
        if !self.phase.get().is_busy() {
            if let Some(a) = self.advanced.borrow_mut().take() {
                a.destroy();
            }
            self.state.save_window(&self.window);
            return glib::Propagation::Proceed;
        }
        if self.quit_after_export.get() {
            return glib::Propagation::Stop; // already stopping; closes when done
        }
        let d = adw::AlertDialog::new(
            Some("Export in progress"),
            Some("A GIF is still being generated. Stop it and quit?"),
        );
        d.add_responses(&[("keep", "Keep generating"), ("quit", "Stop and quit")]);
        d.set_response_appearance("quit", adw::ResponseAppearance::Destructive);
        d.set_default_response(Some("keep"));
        d.set_close_response("keep");
        let w = self.clone();
        d.choose(Some(&self.window), None::<&gio::Cancellable>, move |r| {
            if r == "quit" {
                w.quit_after_export.set(true);
                w.cancel_export();
            }
        });
        glib::Propagation::Stop
    }
}
