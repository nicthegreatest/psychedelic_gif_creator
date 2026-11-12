//! Settings rows built from the core schema (port of v2 `settings_form.cpp`).
//!
//! Numbers get a slider plus a spin button sharing one `gtk::Adjustment`,
//! choices an `AdwComboRow` and bools an `AdwSwitchRow`. Every edit is
//! snapped and clamped through the schema, so the form never holds a value the
//! renderer would not accept.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gifcore::schema::{self, FieldInfo, FieldKind};
use gifcore::settings::GifSettings;
use gifcore::ui_model::{self, Group};

type OnChange = Rc<dyn Fn(&'static str)>;

enum Editor {
    Number(gtk::Adjustment),
    Choice(adw::ComboRow),
    Bool(adw::SwitchRow),
}

struct Row {
    field: &'static FieldInfo,
    widget: gtk::Widget,
    editor: Editor,
}

struct Inner {
    settings: Rc<RefCell<GifSettings>>,
    rows: RefCell<Vec<Row>>,
    groups: RefCell<Vec<adw::PreferencesGroup>>,
    loading: Cell<bool>,
    on_change: RefCell<Option<OnChange>>,
}

/// A set of rows editing one `GifSettings`.
#[derive(Clone)]
pub struct Form(Rc<Inner>);

impl Form {
    pub fn new(settings: Rc<RefCell<GifSettings>>) -> Self {
        Self(Rc::new(Inner {
            settings,
            rows: RefCell::default(),
            groups: RefCell::default(),
            loading: Cell::new(false),
            on_change: RefCell::default(),
        }))
    }

    /// Called with the key after every user edit (not after [`Self::load`]).
    pub fn set_on_change(&self, f: impl Fn(&'static str) + 'static) {
        *self.0.on_change.borrow_mut() = Some(Rc::new(f));
    }

    /// Builds a preferences group with one row per key.
    pub fn build_group(&self, group: &Group) -> adw::PreferencesGroup {
        let pg = adw::PreferencesGroup::builder().title(group.title).build();
        for key in group.keys {
            let field = schema::find(key).expect("group key is in the schema");
            let row = self.build_row(field);
            pg.add(&row.widget);
            self.0.rows.borrow_mut().push(row);
        }
        self.0.groups.borrow_mut().push(pg.clone());
        self.load();
        pg
    }

    fn build_row(&self, field: &'static FieldInfo) -> Row {
        let key = field.key;
        match field.kind {
            FieldKind::Choice => {
                let combo = adw::ComboRow::builder()
                    .title(field.label)
                    .tooltip_text(field.tooltip)
                    .model(&gtk::StringList::new(field.choices))
                    .build();
                let form = self.clone();
                combo.connect_selected_notify(move |c| form.store(key, f64::from(c.selected())));
                Row {
                    field,
                    widget: combo.clone().upcast(),
                    editor: Editor::Choice(combo),
                }
            }
            FieldKind::Bool => {
                let switch = adw::SwitchRow::builder()
                    .title(field.label)
                    .tooltip_text(field.tooltip)
                    .build();
                let form = self.clone();
                switch.connect_active_notify(move |s| {
                    form.store(key, if s.is_active() { 1.0 } else { 0.0 })
                });
                Row {
                    field,
                    widget: switch.clone().upcast(),
                    editor: Editor::Bool(switch),
                }
            }
            FieldKind::Int | FieldKind::Double => {
                let adj = gtk::Adjustment::new(
                    field.default_value(),
                    field.min,
                    field.max,
                    field.step,
                    field.step * 10.0,
                    0.0,
                );
                let row = adw::ActionRow::builder()
                    .title(field.label)
                    .tooltip_text(field.tooltip)
                    .build();
                if ui_model::has_slider(field) {
                    let scale = gtk::Scale::builder()
                        .orientation(gtk::Orientation::Horizontal)
                        .adjustment(&adj)
                        .draw_value(false)
                        .round_digits(field.decimals as i32)
                        .width_request(170)
                        .valign(gtk::Align::Center)
                        .tooltip_text(field.tooltip)
                        .build();
                    row.add_suffix(&scale);
                }
                // Typed values apply on Enter or focus-out (GTK's default, as v2).
                let spin = gtk::SpinButton::builder()
                    .adjustment(&adj)
                    .digits(field.decimals)
                    .width_chars(7)
                    .valign(gtk::Align::Center)
                    .tooltip_text(field.tooltip)
                    .build();
                row.add_suffix(&spin);
                let form = self.clone();
                adj.connect_value_changed(move |a| form.store(key, a.value()));
                Row {
                    field,
                    widget: row.upcast(),
                    editor: Editor::Number(adj),
                }
            }
        }
    }

    fn store(&self, key: &'static str, value: f64) {
        if self.0.loading.get() {
            return;
        }
        let field = schema::find(key).expect("row key is in the schema");
        let value = if matches!(field.kind, FieldKind::Double) {
            ui_model::snap(field, value)
        } else {
            value
        };
        field.set_value(&mut self.0.settings.borrow_mut(), value);
        self.update_visibility();
        let cb = self.0.on_change.borrow().clone();
        if let Some(cb) = cb {
            cb(key);
        }
    }

    /// Shows the current settings in every row (no change callbacks).
    pub fn load(&self) {
        self.0.loading.set(true);
        {
            let s = self.0.settings.borrow();
            for row in self.0.rows.borrow().iter() {
                let v = row.field.value(&s);
                match &row.editor {
                    Editor::Number(adj) => adj.set_value(v),
                    Editor::Choice(c) => c.set_selected(v as u32),
                    Editor::Bool(b) => b.set_active(v >= 0.5),
                }
            }
        }
        self.0.loading.set(false);
        self.update_visibility();
    }

    /// Shows or hides rows by v2's rules.
    pub fn update_visibility(&self) {
        let s = self.0.settings.borrow();
        for row in self.0.rows.borrow().iter() {
            row.widget
                .set_visible(ui_model::row_visible(&s, row.field.key));
        }
    }

    /// Enables or disables every group built by this form.
    pub fn set_sensitive(&self, on: bool) {
        for g in self.0.groups.borrow().iter() {
            g.set_sensitive(on);
        }
    }
}
