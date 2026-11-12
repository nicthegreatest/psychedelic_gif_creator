//! Window size and last folders, kept in GSettings between runs.
//!
//! Effect settings, presets and the console toggle are deliberately not saved.
//! When the schema is not installed (plain `cargo run`), nothing is stored.

use std::path::{Path, PathBuf};

use gifcore::ui_model;
use gtk::gio;
use gtk::prelude::*;

use crate::config;

pub struct State(Option<gio::Settings>);

impl State {
    pub fn load() -> Self {
        let installed = gio::SettingsSchemaSource::default()
            .and_then(|src| src.lookup(config::SCHEMA_ID, true))
            .is_some();
        Self(installed.then(|| gio::Settings::new(config::SCHEMA_ID)))
    }

    /// Saved (width, height, maximized), with defaults for missing or odd values.
    pub fn window(&self) -> (i32, i32, bool) {
        let Some(s) = &self.0 else {
            let (w, h) = ui_model::DEFAULT_WINDOW_SIZE;
            return (w, h, false);
        };
        let (w, h) = ui_model::restored_window_size(s.int("window-width"), s.int("window-height"));
        (w, h, s.boolean("window-maximized"))
    }

    pub fn save_window(&self, window: &impl IsA<gtk::Window>) {
        let Some(s) = &self.0 else { return };
        let (w, h) = window.default_size();
        let _ = s.set_int("window-width", w);
        let _ = s.set_int("window-height", h);
        let _ = s.set_boolean("window-maximized", window.is_maximized());
    }

    fn dir(&self, key: &str) -> Option<PathBuf> {
        self.0
            .as_ref()
            .and_then(|s| ui_model::restored_dir(&s.string(key)))
    }

    fn remember(&self, key: &str, file: &Path) {
        if let (Some(s), Some(dir)) = (&self.0, ui_model::folder_to_remember(file)) {
            let _ = s.set_string(key, &dir);
        }
    }

    pub fn open_dir(&self) -> Option<PathBuf> {
        self.dir("last-open-dir")
    }

    pub fn remember_open(&self, image: &Path) {
        self.remember("last-open-dir", image);
    }

    pub fn save_dir(&self) -> Option<PathBuf> {
        self.dir("last-save-dir")
    }

    pub fn remember_save(&self, gif: &Path) {
        self.remember("last-save-dir", gif);
    }
}
