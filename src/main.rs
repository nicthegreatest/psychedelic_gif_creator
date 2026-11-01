mod application;
#[rustfmt::skip]
#[allow(dead_code)]
mod config;
mod window;

use gtk::glib;

fn main() -> glib::ExitCode {
    glib::set_application_name(&format!("Psychedelic GIF Creator{}", config::NAME_SUFFIX));
    application::run()
}
