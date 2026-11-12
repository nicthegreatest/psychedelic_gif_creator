//! The main window (port of v2 `mainwindow.cpp`): settings on the left, live
//! preview on the right, Generate GIF with progress and Cancel.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use adw::prelude::*;
use gifcore::settings::GifSettings;
use gifcore::ui_model::{self, ExportPhase, Log, PreviewQueue, MAIN_GROUPS};
use gtk::{gdk, gio, glib};

use crate::form::Form;
use crate::preview::{self, SourceCache};

/// Debounce before a preview re-render after a setting changes (v2: 150 ms).
const PREVIEW_DEBOUNCE_MS: u64 = 150;
/// Re-render delay after the preview area is resized (v2: 250 ms).
const RESIZE_DEBOUNCE_MS: u64 = 250;
const IMAGE_SUFFIXES: &[&str] = &[
    "png", "jpg", "jpeg", "bmp", "webp", "tif", "tiff", "gif", "pnm", "ppm", "pgm",
];

pub struct Win {
    pub window: adw::ApplicationWindow,
    toasts: adw::ToastOverlay,
    title: adw::WindowTitle,
    pub settings: Rc<RefCell<GifSettings>>,
    pub preview_override: RefCell<Option<GifSettings>>,
    pub form: Form,
    input_group: adw::PreferencesGroup,
    path_row: adw::EntryRow,
    advanced_button: gtk::Button,
    reset_button: gtk::Button,
    pub generate_button: gtk::Button,
    pub cancel_button: gtk::Button,
    pub progress: gtk::ProgressBar,
    area: gtk::DrawingArea,
    picture: gtk::Picture,
    placeholder: gtk::Label,
    frame_scale: gtk::Scale,
    pub play: gtk::ToggleButton,
    live: gtk::CheckButton,
    console_toggle: gtk::CheckButton,
    console: gtk::Revealer,
    console_view: gtk::TextView,
    console_clear: gtk::Button,
    log: RefCell<Log>,
    /// Settings as last written to the console, to log only what changed.
    logged: RefCell<GifSettings>,
    /// Same for the Advanced Effects working copy while that window is open.
    pub adv_logged: RefCell<Option<GifSettings>>,
    pub cache: Arc<SourceCache>,
    queue: Cell<PreviewQueue>,
    debounce: RefCell<Option<glib::SourceId>>,
    play_timer: RefCell<Option<glib::SourceId>>,
    last_error: RefCell<String>,
    pub phase: Cell<ExportPhase>,
    pub cancel_flag: RefCell<Option<Arc<AtomicBool>>>,
    pub quit_after_export: Cell<bool>,
    pub last_output_dir: RefCell<Option<PathBuf>>,
    pub advanced: RefCell<Option<adw::Window>>,
}

fn button(label: &str, tooltip: &str) -> gtk::Button {
    gtk::Button::builder()
        .label(label)
        .tooltip_text(tooltip)
        .build()
}

pub fn build(app: &adw::Application) -> Rc<Win> {
    let settings = Rc::new(RefCell::new(GifSettings::default()));
    let form = Form::new(settings.clone());

    // --- Left: settings --------------------------------------------------
    let path_row = adw::EntryRow::builder()
        .title("Image path")
        .show_apply_button(true)
        .tooltip_text("Type or paste a path and press Enter, or use Browse. You can also drop an image on the window.")
        .build();
    let browse = gtk::Button::builder()
        .icon_name("document-open-symbolic")
        .tooltip_text("Browse…")
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    path_row.add_suffix(&browse);
    let input_group = adw::PreferencesGroup::builder()
        .title("Input image")
        .build();
    input_group.add(&path_row);

    let page = adw::PreferencesPage::new();
    page.add(&input_group);
    let aspect = gtk::Button::builder()
        .label("Match image aspect")
        .tooltip_text("Set the height so the GIF has the same shape as the input image.")
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    for (i, g) in MAIN_GROUPS.iter().enumerate() {
        let pg = form.build_group(g);
        if i == 0 {
            pg.set_header_suffix(Some(&aspect));
        }
        page.add(&pg);
    }

    let advanced_button = button(
        "Advanced effects…",
        "Zoom details, kaleidoscope, stars, wave, pixelation, blur and vignette.",
    );
    let reset_button = button(
        "Reset all",
        "Restore every setting to its default (the input image is kept).",
    );
    let cancel_button = button("Cancel", "Stop generating the GIF.");
    cancel_button.set_visible(false);
    let generate_button = button("Generate GIF", "Render every frame and save the GIF.");
    generate_button.add_css_class("suggested-action");
    let spacer = gtk::Box::builder().hexpand(true).build();
    let actions = gtk::Box::builder()
        .spacing(6)
        .margin_start(12)
        .margin_end(12)
        .margin_top(6)
        .margin_bottom(6)
        .build();
    actions.append(&advanced_button);
    actions.append(&reset_button);
    actions.append(&spacer);
    actions.append(&cancel_button);
    actions.append(&generate_button);
    let progress = gtk::ProgressBar::builder()
        .show_text(true)
        .visible(false)
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(12)
        .build();
    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 0);
    page.set_vexpand(true);
    sidebar.append(&page);
    sidebar.append(&actions);
    sidebar.append(&progress);

    // --- Right: preview ---------------------------------------------------
    let area = gtk::DrawingArea::builder()
        .hexpand(true)
        .vexpand(true)
        .width_request(320)
        .height_request(320)
        .build();
    let picture = gtk::Picture::builder()
        .content_fit(gtk::ContentFit::Contain)
        .can_shrink(true)
        .build();
    let placeholder = gtk::Label::builder()
        .label("Choose an image to see a preview")
        .wrap(true)
        .justify(gtk::Justification::Center)
        .css_classes(["dim-label", "title-4"])
        .margin_start(24)
        .margin_end(24)
        .build();
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&area));
    overlay.add_overlay(&picture);
    overlay.add_overlay(&placeholder);
    overlay.set_measure_overlay(&picture, false);
    overlay.set_clip_overlay(&picture, true);

    let play = gtk::ToggleButton::builder()
        .label("Play")
        .tooltip_text("Play the preview (renders as fast as it can; the exported GIF plays at the chosen fps).")
        .build();
    let n = settings.borrow().num_frames;
    let frame_scale = gtk::Scale::builder()
        .orientation(gtk::Orientation::Horizontal)
        .adjustment(&gtk::Adjustment::new(
            0.0,
            0.0,
            f64::from(n - 1),
            1.0,
            10.0,
            0.0,
        ))
        .round_digits(0)
        .digits(0)
        .draw_value(true)
        .value_pos(gtk::PositionType::Right)
        .hexpand(true)
        .tooltip_text("Preview frame")
        .build();
    let live = gtk::CheckButton::builder()
        .label("Live preview")
        .active(true)
        .tooltip_text("Re-render the preview whenever a setting changes.")
        .build();
    let scrub = gtk::Box::builder()
        .spacing(12)
        .margin_start(12)
        .margin_end(12)
        .margin_top(6)
        .margin_bottom(12)
        .build();
    scrub.append(&play);
    scrub.append(&frame_scale);
    scrub.append(&live);
    let console_toggle = gtk::CheckButton::builder()
        .label("Show console")
        .tooltip_text("Show a log of every setting change and export.")
        .build();
    scrub.append(&console_toggle);

    let console_view = gtk::TextView::builder()
        .editable(false)
        .cursor_visible(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .top_margin(6)
        .bottom_margin(6)
        .left_margin(6)
        .right_margin(6)
        .build();
    let console_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .height_request(160)
        .child(&console_view)
        .build();
    let console_clear = gtk::Button::builder()
        .label("Clear")
        .tooltip_text("Clear the console.")
        .css_classes(["flat"])
        .build();
    let console_head = gtk::Box::builder().spacing(6).build();
    console_head.append(
        &gtk::Label::builder()
            .label("Console")
            .xalign(0.0)
            .hexpand(true)
            .css_classes(["heading"])
            .build(),
    );
    console_head.append(&console_clear);
    let console_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(12)
        .build();
    console_box.append(&console_head);
    console_box.append(&console_scroll);
    let console = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::SlideUp)
        .reveal_child(false)
        .child(&console_box)
        .build();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    overlay.set_margin_start(12);
    overlay.set_margin_end(12);
    overlay.set_margin_top(12);
    content.append(&overlay);
    content.append(&scrub);
    content.append(&console);

    let split = adw::OverlaySplitView::builder()
        .sidebar(&sidebar)
        .content(&content)
        .min_sidebar_width(430.0)
        .max_sidebar_width(560.0)
        .sidebar_width_fraction(0.42)
        .build();

    let title = adw::WindowTitle::new("Psychedelic GIF Creator", "Choose an image to begin.");
    let header = adw::HeaderBar::builder().title_widget(&title).build();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&split));
    let toasts = adw::ToastOverlay::new();
    toasts.set_child(Some(&toolbar));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Psychedelic GIF Creator")
        .default_width(1100)
        .default_height(760)
        .content(&toasts)
        .build();

    let win = Rc::new(Win {
        window,
        toasts,
        title,
        settings,
        preview_override: RefCell::default(),
        form,
        input_group,
        path_row,
        advanced_button,
        reset_button,
        generate_button,
        cancel_button,
        progress,
        area,
        picture,
        placeholder,
        frame_scale,
        play,
        live,
        console_toggle,
        console,
        console_view,
        console_clear,
        log: RefCell::default(),
        logged: RefCell::new(GifSettings::default()),
        adv_logged: RefCell::default(),
        cache: Arc::default(),
        queue: Cell::default(),
        debounce: RefCell::default(),
        play_timer: RefCell::default(),
        last_error: RefCell::default(),
        phase: Cell::default(),
        cancel_flag: RefCell::default(),
        quit_after_export: Cell::new(false),
        last_output_dir: RefCell::default(),
        advanced: RefCell::default(),
    });
    win.connect(&browse, &aspect);
    win
}

impl Win {
    fn connect(self: &Rc<Self>, browse: &gtk::Button, aspect: &gtk::Button) {
        let w = self.clone();
        self.form.set_on_change(move |key| w.setting_changed(key));

        let w = self.clone();
        self.path_row
            .connect_apply(move |r| w.set_image_path(r.text().trim().into()));
        let w = self.clone();
        self.path_row
            .connect_entry_activated(move |r| w.set_image_path(r.text().trim().into()));
        let w = self.clone();
        browse.connect_clicked(move |_| w.browse_for_image());
        let w = self.clone();
        aspect.connect_clicked(move |_| w.match_image_aspect());
        let w = self.clone();
        self.advanced_button
            .connect_clicked(move |_| w.open_advanced());
        let w = self.clone();
        self.reset_button.connect_clicked(move |_| w.reset_all());
        let w = self.clone();
        self.generate_button
            .connect_clicked(move |_| w.start_export());
        let w = self.clone();
        self.cancel_button
            .connect_clicked(move |_| w.cancel_export());

        let w = self.clone();
        self.frame_scale
            .connect_value_changed(move |_| w.schedule_preview(0));
        let w = self.clone();
        self.play
            .connect_toggled(move |b| w.set_playing(b.is_active()));
        let w = self.clone();
        self.live.connect_toggled(move |b| {
            if b.is_active() {
                w.schedule_preview(0);
            }
        });
        let w = self.clone();
        self.console_toggle.connect_toggled(move |b| {
            w.console.set_reveal_child(b.is_active());
            if b.is_active() {
                w.refresh_console();
            }
        });
        let w = self.clone();
        self.console_clear.connect_clicked(move |_| {
            w.log.borrow_mut().clear();
            w.refresh_console();
        });
        let w = self.clone();
        self.area
            .connect_resize(move |_, _, _| w.schedule_preview(RESIZE_DEBOUNCE_MS));

        // Drag and drop an image file anywhere on the window.
        let drop = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
        let w = self.clone();
        drop.connect_drop(move |_, value, _, _| {
            if w.phase.get().is_busy() {
                return false;
            }
            let Ok(list) = value.get::<gdk::FileList>() else {
                return false;
            };
            match list.files().first().and_then(|f| f.path()) {
                Some(p) => {
                    w.set_image_path(p);
                    true
                }
                None => false,
            }
        });
        self.toasts.add_controller(drop);

        let w = self.clone();
        self.window
            .connect_close_request(move |_| w.close_requested());

        self.update_locks();
        self.schedule_preview(0);
    }

    /// Appends a timestamped line to the console.
    pub fn log(&self, message: &str) {
        let time = glib::DateTime::now_local()
            .and_then(|d| d.format("%H:%M:%S"))
            .map(|s| s.to_string())
            .unwrap_or_default();
        self.log.borrow_mut().push(&time, message);
        if self.console.reveals_child() {
            self.refresh_console();
        }
    }

    /// Logs every main setting that changed since the last call.
    pub fn log_changes(&self) {
        let now = self.settings.borrow().clone();
        for line in ui_model::settings_diff(&self.logged.borrow(), &now) {
            self.log(&line);
        }
        *self.logged.borrow_mut() = now;
    }

    fn refresh_console(&self) {
        let buf = self.console_view.buffer();
        buf.set_text(&self.log.borrow().text());
        // Auto-scroll once the new text is laid out.
        let view = self.console_view.clone();
        glib::idle_add_local_once(move || {
            let mut end = view.buffer().end_iter();
            view.scroll_to_iter(&mut end, 0.0, false, 0.0, 1.0);
        });
    }

    pub fn toast(&self, text: &str, seconds: u32) {
        let t = adw::Toast::builder().title(text).timeout(seconds).build();
        self.toasts.add_toast(t);
    }

    /// Header subtitle: busy status, else "Ready." / "Choose an image to begin.".
    pub fn update_status(&self) {
        let text = match self.phase.get().status() {
            Some(s) => s.to_string(),
            None if self.settings.borrow().image_path.as_os_str().is_empty() => {
                "Choose an image to begin.".to_string()
            }
            None => "Ready.".to_string(),
        };
        self.title.set_subtitle(&text);
    }

    /// Locks controls while exporting, and the main form while Advanced is open.
    pub fn update_locks(&self) {
        let busy = self.phase.get().is_busy();
        let advanced = self.advanced.borrow().is_some();
        self.input_group.set_sensitive(!busy);
        self.form.set_sensitive(!busy && !advanced);
        self.advanced_button.set_sensitive(!busy);
        self.reset_button.set_sensitive(!busy && !advanced);
        self.generate_button.set_sensitive(!busy);
        self.cancel_button.set_visible(busy);
        self.progress.set_visible(busy);
        self.update_status();
    }

    pub fn alert(&self, heading: &str, body: &str) {
        let d = adw::AlertDialog::new(Some(heading), Some(body));
        d.add_response("ok", "OK");
        d.set_default_response(Some("ok"));
        d.present(Some(&self.window));
    }

    fn setting_changed(self: &Rc<Self>, key: &'static str) {
        self.log_changes();
        if key == "num_frames" {
            let n = self.settings.borrow().num_frames;
            self.frame_scale.set_range(0.0, f64::from(n - 1));
        }
        if key == "rotation_turns" || key == "zoom_mode" {
            let msg = ui_model::spin_rounding_message(&self.settings.borrow());
            if let Some(msg) = msg {
                self.log(&msg);
            }
        }
        if key == "fps" && self.play.is_active() {
            self.set_playing(true);
        }
        self.schedule_preview(PREVIEW_DEBOUNCE_MS);
    }

    fn set_image_path(self: &Rc<Self>, path: PathBuf) {
        if path == self.settings.borrow().image_path {
            return;
        }
        let text = path.to_string_lossy().into_owned();
        if self.path_row.text() != text {
            self.path_row.set_text(&text);
        }
        if let Some(name) = path.file_name().filter(|_| self.live.is_active()) {
            self.title
                .set_subtitle(&format!("Loading {}…", name.to_string_lossy()));
            self.placeholder.set_label("Loading…");
        }
        self.log(&format!("Image: {}", path.display()));
        self.settings.borrow_mut().image_path = path;
        self.schedule_preview(0);
    }

    fn browse_for_image(self: &Rc<Self>) {
        let images = gtk::FileFilter::new();
        images.set_name(Some("Images"));
        for s in IMAGE_SUFFIXES {
            images.add_suffix(s);
        }
        let all = gtk::FileFilter::new();
        all.set_name(Some("All files"));
        all.add_pattern("*");
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&images);
        filters.append(&all);
        let dialog = gtk::FileDialog::builder()
            .title("Choose an image")
            .modal(true)
            .filters(&filters)
            .default_filter(&images)
            .build();
        if let Some(dir) = self.settings.borrow().image_path.parent() {
            if dir.is_dir() && !ui_model::is_portal_path(dir) {
                dialog.set_initial_folder(Some(&gio::File::for_path(dir)));
            }
        }
        let w = self.clone();
        glib::spawn_future_local(async move {
            if let Ok(file) = dialog.open_future(Some(&w.window)).await {
                match file.path() {
                    Some(p) => w.set_image_path(p),
                    None => w.toast("Only local files can be opened.", 5),
                }
            }
        });
    }

    fn match_image_aspect(self: &Rc<Self>) {
        let path = self.settings.borrow().image_path.clone();
        if path.as_os_str().is_empty() {
            self.toast("No image loaded.", 5);
            return;
        }
        let cache = self.cache.clone();
        let w = self.clone();
        glib::spawn_future_local(async move {
            let res = gio::spawn_blocking(move || cache.get(&path)).await;
            match res {
                Ok(Ok(img)) => {
                    let text = {
                        let mut s = w.settings.borrow_mut();
                        ui_model::match_aspect(&mut s, img.width, img.height);
                        format!("Output set to {} x {}.", s.output_width, s.output_height)
                    };
                    w.form.load();
                    w.log_changes();
                    w.log(&text);
                    w.schedule_preview(PREVIEW_DEBOUNCE_MS);
                }
                Ok(Err(e)) => w.toast(&e.to_string(), 5),
                Err(p) => w.toast(&preview::panic_text(&*p), 5),
            }
        });
    }

    fn reset_all(self: &Rc<Self>) {
        ui_model::reset_all(&mut self.settings.borrow_mut());
        self.form.load();
        let n = self.settings.borrow().num_frames;
        self.frame_scale.set_range(0.0, f64::from(n - 1));
        self.log_changes();
        self.log("All settings restored to defaults.");
        self.schedule_preview(0);
    }

    // --- Preview ------------------------------------------------------------

    /// (Re)starts the debounce timer for a preview render.
    pub fn schedule_preview(self: &Rc<Self>, delay_ms: u64) {
        if !self.live.is_active() {
            return;
        }
        if let Some(id) = self.debounce.borrow_mut().take() {
            id.remove();
        }
        let w = self.clone();
        let id = glib::timeout_add_local_once(Duration::from_millis(delay_ms), move || {
            w.debounce.borrow_mut().take();
            w.start_preview();
        });
        *self.debounce.borrow_mut() = Some(id);
    }

    fn start_preview(self: &Rc<Self>) {
        let mut q = self.queue.get();
        let now = q.request();
        self.queue.set(q);
        if now {
            self.run_preview_job();
        }
    }

    fn preview_job_done(self: &Rc<Self>) {
        let mut q = self.queue.get();
        let again = q.finished();
        self.queue.set(q);
        if again {
            self.run_preview_job();
        }
    }

    fn run_preview_job(self: &Rc<Self>) {
        let mut s = self
            .preview_override
            .borrow()
            .clone()
            .unwrap_or_else(|| self.settings.borrow().clone());
        // The Advanced window's working copy may predate a newly chosen image.
        s.image_path = self.settings.borrow().image_path.clone();
        if s.image_path.as_os_str().is_empty() {
            self.show_preview_text("Choose an image to see a preview");
            self.update_status();
            self.preview_job_done();
            return;
        }
        let frame = (self.frame_scale.value().round() as i32).min(s.num_frames - 1);
        let sf = f64::from(self.area.scale_factor().max(1));
        let bw = f64::from(self.area.width().max(320)) * sf;
        let bh = f64::from(self.area.height().max(320)) * sf;
        let scale = ui_model::preview_scale(&s, bw, bh);
        let cache = self.cache.clone();
        let w = self.clone();
        glib::spawn_future_local(async move {
            let res = gio::spawn_blocking(move || cache.render(&s, scale, frame)).await;
            match res {
                Ok(Ok(frame)) => {
                    w.picture.set_paintable(Some(&preview::texture(frame)));
                    w.picture.set_visible(true);
                    w.placeholder.set_visible(false);
                    w.last_error.borrow_mut().clear();
                    w.update_status();
                }
                Ok(Err(e)) => w.preview_error(e),
                Err(p) => w.preview_error(format!("Preview failed: {}", preview::panic_text(&*p))),
            }
            w.preview_job_done();
        });
    }

    fn show_preview_text(&self, text: &str) {
        self.picture.set_paintable(None::<&gdk::Paintable>);
        self.picture.set_visible(false);
        self.placeholder.set_label(text);
        self.placeholder.set_visible(true);
    }

    fn preview_error(&self, err: String) {
        self.show_preview_text(&err);
        self.update_status();
        // Toast once per distinct error, not on every slider step.
        if *self.last_error.borrow() != err {
            self.toast(&err, 6);
            self.log(&err);
            *self.last_error.borrow_mut() = err;
        }
    }

    pub fn set_playing(self: &Rc<Self>, play: bool) {
        if let Some(id) = self.play_timer.borrow_mut().take() {
            id.remove();
        }
        self.play.set_label(if play { "Pause" } else { "Play" });
        if !play {
            return;
        }
        let fps = self.settings.borrow().fps;
        let w = self.clone();
        let id = glib::timeout_add_local(
            Duration::from_millis(u64::from(ui_model::play_interval_ms(fps))),
            move || {
                // Drop frames rather than queue them while a render runs.
                if !w.queue.get().is_running() {
                    let n = w.settings.borrow().num_frames.max(1);
                    let next = (w.frame_scale.value().round() as i32 + 1) % n;
                    w.frame_scale.set_value(f64::from(next));
                }
                glib::ControlFlow::Continue
            },
        );
        *self.play_timer.borrow_mut() = Some(id);
    }
}
