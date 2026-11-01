use adw::prelude::*;
use gtk::glib;

use crate::{config, window};

pub fn run() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id(config::APP_ID)
        .build();

    app.connect_activate(|app| {
        // Single window: re-activating (e.g. launching again) raises the existing one.
        if let Some(win) = app.active_window() {
            win.present();
            return;
        }
        window::build(app).present();
    });

    app.run()
}
