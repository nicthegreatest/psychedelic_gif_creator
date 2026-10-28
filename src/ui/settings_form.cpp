// settings_form.cpp
#include "settings_form.h"

#include <QCheckBox>
#include <QComboBox>
#include <QDoubleSpinBox>
#include <QGridLayout>
#include <QHBoxLayout>
#include <QLabel>
#include <QSlider>
#include <cmath>

SliderSpin::SliderSpin(const FieldInfo& f, QWidget* parent) : QWidget(parent), m_min(f.min), m_step(f.step) {
    auto* layout = new QHBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    const int steps = int(std::lround((f.max - f.min) / f.step));
    if (steps <= 2000) {
        m_slider = new QSlider(Qt::Horizontal, this);
        m_slider->setRange(0, steps);
        m_slider->setToolTip(f.tooltip);
        layout->addWidget(m_slider, 1);
    }
    m_spin = new QDoubleSpinBox(this);
    m_spin->setRange(f.min, f.max);
    m_spin->setSingleStep(f.step);
    m_spin->setDecimals(f.decimals);
    m_spin->setKeyboardTracking(false); // typed values apply on Enter / focus out
    m_spin->setToolTip(f.tooltip);
    m_spin->setMinimumWidth(90);
    layout->addWidget(m_spin, m_slider ? 0 : 1);

    if (m_slider) {
        connect(m_slider, &QSlider::valueChanged, this, [this](int i) {
            const double v = m_min + i * m_step;
            if (std::abs(v - m_spin->value()) < m_step / 2) return;
            m_spin->blockSignals(true);
            m_spin->setValue(v);
            m_spin->blockSignals(false);
            emit valueChanged(m_spin->value());
        });
    }
    connect(m_spin, &QDoubleSpinBox::valueChanged, this, [this](double v) {
        if (m_slider) {
            m_slider->blockSignals(true);
            m_slider->setValue(toSlider(v));
            m_slider->blockSignals(false);
        }
        emit valueChanged(v);
    });
}

int SliderSpin::toSlider(double v) const { return int(std::lround((v - m_min) / m_step)); }

double SliderSpin::value() const { return m_spin->value(); }

void SliderSpin::setValue(double v) {
    const QSignalBlocker a(m_spin);
    m_spin->setValue(v);
    if (m_slider) {
        const QSignalBlocker b(m_slider);
        m_slider->setValue(toSlider(v));
    }
}

SettingsForm::SettingsForm(GifSettings* target, QObject* parent) : QObject(parent), m_target(target) {}

void SettingsForm::addRow(QGridLayout* grid, const QString& key) {
    const FieldInfo* f = findField(key);
    Q_ASSERT(f);
    if (!f) return;
    Row row;
    row.field = f;
    row.label = new QLabel(f->label);
    row.label->setToolTip(f->tooltip);

    switch (f->kind) {
    case FieldInfo::Kind::Choice: {
        auto* combo = new QComboBox;
        combo->addItems(f->choices);
        connect(combo, &QComboBox::currentIndexChanged, this, [this, key](int i) { store(key, i); });
        row.editor = combo;
        break;
    }
    case FieldInfo::Kind::Bool: {
        auto* box = new QCheckBox;
        connect(box, &QCheckBox::toggled, this, [this, key](bool on) { store(key, on ? 1.0 : 0.0); });
        row.editor = box;
        break;
    }
    default: {
        auto* edit = new SliderSpin(*f);
        connect(edit, &SliderSpin::valueChanged, this, [this, key](double v) { store(key, v); });
        row.editor = edit;
        break;
    }
    }
    row.editor->setToolTip(f->tooltip);
    row.label->setBuddy(row.editor);
    const int r = grid->rowCount();
    grid->addWidget(row.label, r, 0);
    grid->addWidget(row.editor, r, 1);
    m_rows.insert(key, row);
    m_order << key;
    load();
}

void SettingsForm::store(const QString& key, double value) {
    const Row& row = m_rows[key];
    row.field->set(*m_target, value);
    updateVisibility();
    emit changed(key);
}

void SettingsForm::load() {
    for (auto it = m_rows.cbegin(); it != m_rows.cend(); ++it) {
        const double v = it->field->get(*m_target);
        const QSignalBlocker block(it->editor);
        if (auto* combo = qobject_cast<QComboBox*>(it->editor)) combo->setCurrentIndex(int(v));
        else if (auto* box = qobject_cast<QCheckBox*>(it->editor)) box->setChecked(v >= 0.5);
        else if (auto* edit = qobject_cast<SliderSpin*>(it->editor)) edit->setValue(v);
    }
    updateVisibility();
}

void SettingsForm::setRowVisible(const QString& key, bool visible) {
    auto it = m_rows.find(key);
    if (it == m_rows.end()) return;
    it->label->setVisible(visible);
    it->editor->setVisible(visible);
}

void SettingsForm::setEnabled(bool enabled) {
    for (const Row& r : std::as_const(m_rows)) r.editor->setEnabled(enabled);
}

void SettingsForm::updateVisibility() {
    const GifSettings& s = *m_target;
    const bool radial = s.kaleidoscope_mode == KaleidoscopeMode::Radial3 || s.kaleidoscope_mode == KaleidoscopeMode::Radial4 ||
                        s.kaleidoscope_mode == KaleidoscopeMode::Radial6;
    setRowVisible("kaleidoscope_turns", radial);
    const bool stars = s.star_pattern != StarPattern::None;
    setRowVisible("num_stars", stars);
    setRowVisible("star_seed", stars && s.star_pattern == StarPattern::Random);
    const bool wave = s.wave_direction != WaveDirection::None;
    for (const char* k : {"wave_amplitude", "wave_frequency", "wave_cycles"}) setRowVisible(k, wave);
    const bool osc = s.zoom_mode == ZoomMode::Oscillating;
    for (const char* k : {"zoom_amplitude", "zoom_cycles", "zoom_midpoint"}) setRowVisible(k, osc);
    setRowVisible("tunnel_steps", s.zoom_mode == ZoomMode::Tunnel);
    setRowVisible("rotation_turns", s.rotation_direction != RotationDirection::None);
    setRowVisible("saturation_pulses", s.saturation_pulse > 0.0);
}
