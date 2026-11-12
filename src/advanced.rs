//! "Advanced effects" window (port of v2 `advancedsettingsdialog.cpp`).
//!
//! Non-modal, so it can be moved off the preview on GNOME. It edits a working
//! copy that the preview shows live; OK applies it, Cancel or closing restores
//! the previous settings. The main form is locked while it is open.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use adw::prelude::*;
use gifcore::settings::GifSettings;
use gifcore::ui_model::{self, ADVANCED_GROUPS};
use gtk::glib;

use crate::form::Form;
use crate::window::Win;

impl Win {
    pub fn open_advanced(self: &Rc<Self>) {
        if let Some(w) = self.advanced.borrow().as_ref() {
            w.present();
            return;
        }
        let working = Rc::new(RefCell::new(self.settings.borrow().clone()));
        let form = Form::new(working.clone());
        let page = adw::PreferencesPage::new();
        page.set_vexpand(true);
        for g in ADVANCED_GROUPS {
            page.add(&form.build_group(g));
        }

        let hint = gtk::Label::builder()
            .label("Changes show in the preview straight away. OK keeps them, Cancel restores the previous settings.")
            .wrap(true)
            .xalign(0.0)
            .css_classes(["dim-label"])
            .margin_start(12)
            .margin_end(12)
            .margin_top(6)
            .build();
        let chaos = gtk::Button::builder()
            .label("Cosmic Chaos")
            .tooltip_text("Randomise the effects on this page (sensible ranges). Output size, frames and spin are left alone.")
            .build();
        let defaults = gtk::Button::builder()
            .label("Effect defaults")
            .tooltip_text("Reset only the settings on this page to their defaults.")
            .build();
        let cancel = gtk::Button::with_label("Cancel");
        let ok = gtk::Button::with_label("OK");
        ok.add_css_class("suggested-action");
        let buttons = gtk::Box::builder()
            .spacing(6)
            .margin_start(12)
            .margin_end(12)
            .margin_top(6)
            .margin_bottom(12)
            .build();
        buttons.append(&chaos);
        buttons.append(&defaults);
        buttons.append(&gtk::Box::builder().hexpand(true).build());
        buttons.append(&cancel);
        buttons.append(&ok);

        let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
        body.append(&hint);
        body.append(&page);
        body.append(&buttons);
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&adw::HeaderBar::new());
        toolbar.set_content(Some(&body));
        let dialog = adw::Window::builder()
            .title("Advanced Effects")
            .default_width(520)
            .default_height(680)
            .transient_for(&self.window)
            .modal(false)
            .destroy_with_parent(true)
            .content(&toolbar)
            .build();

        let w = self.clone();
        let wk = working.clone();
        form.set_on_change(move |_| w.preview_working(&wk));
        let (w, wk, f) = (self.clone(), working.clone(), form.clone());
        chaos.connect_clicked(move |_| {
            let seed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(1, |d| d.subsec_nanos() ^ d.as_secs() as u32);
            ui_model::cosmic_chaos(&mut wk.borrow_mut(), ui_model::chaos_source(seed));
            f.load();
            w.preview_working(&wk);
        });
        let (w, wk, f) = (self.clone(), working.clone(), form.clone());
        defaults.connect_clicked(move |_| {
            ui_model::reset_advanced(&mut wk.borrow_mut());
            f.load();
            w.preview_working(&wk);
        });

        let accepted = Rc::new(Cell::new(false));
        let d = dialog.clone();
        cancel.connect_clicked(move |_| d.close());
        let (d, a) = (dialog.clone(), accepted.clone());
        ok.connect_clicked(move |_| {
            a.set(true);
            d.close();
        });
        let w = self.clone();
        dialog.connect_close_request(move |_| {
            w.advanced_finished(accepted.get().then(|| working.borrow().clone()));
            glib::Propagation::Proceed
        });

        *self.adv_logged.borrow_mut() = Some(self.settings.borrow().clone());
        self.log("Advanced effects opened (changes are previews until OK).");
        *self.advanced.borrow_mut() = Some(dialog.clone());
        self.update_locks();
        dialog.present();
    }

    fn preview_working(self: &Rc<Self>, working: &RefCell<GifSettings>) {
        let now = working.borrow().clone();
        let before = self.adv_logged.borrow_mut().replace(now.clone());
        if let Some(before) = before {
            for line in ui_model::settings_diff(&before, &now) {
                self.log(&format!("{line} (preview)"));
            }
        }
        *self.preview_override.borrow_mut() = Some(now);
        self.schedule_preview(150);
    }

    fn advanced_finished(self: &Rc<Self>, result: Option<GifSettings>) {
        if let Some(s) = result {
            // Keep the image path chosen meanwhile; only effects come from the dialog.
            let path = self.settings.borrow().image_path.clone();
            *self.settings.borrow_mut() = GifSettings {
                image_path: path,
                ..s
            };
            self.form.load();
            self.log("Advanced effects applied.");
            self.log_changes();
        } else {
            self.log("Advanced effects cancelled; previous settings kept.");
        }
        self.adv_logged.borrow_mut().take();
        self.preview_override.borrow_mut().take();
        self.advanced.borrow_mut().take();
        self.update_locks();
        self.schedule_preview(0);
    }
}
