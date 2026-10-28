// advancedsettingsdialog.cpp
#include "advancedsettingsdialog.h"

#include "settings_form.h"
#include "settings_schema.h"

#include <QDialogButtonBox>
#include <QGridLayout>
#include <QGroupBox>
#include <QHBoxLayout>
#include <QLabel>
#include <QPushButton>
#include <QRandomGenerator>
#include <QScrollArea>
#include <QVBoxLayout>

namespace {
struct Group {
    const char* title;
    QStringList keys;
};
const QList<Group>& groups() {
    static const QList<Group> g = {
        {"Zoom", {"zoom_amplitude", "zoom_cycles", "zoom_midpoint", "tunnel_steps"}},
        {"Colour", {"saturation_pulse", "saturation_pulses", "invert_flashes"}},
        {"Kaleidoscope", {"kaleidoscope_mode", "kaleidoscope_turns"}},
        {"Stars", {"star_pattern", "num_stars", "star_seed"}},
        {"Wave", {"wave_direction", "wave_amplitude", "wave_frequency", "wave_cycles"}},
        {"Texture", {"pixelation_level", "blur_radius", "vignette_strength"}},
    };
    return g;
}
} // namespace

QStringList AdvancedSettingsDialog::keys() {
    QStringList k;
    for (const auto& g : groups()) k << g.keys;
    return k;
}

AdvancedSettingsDialog::AdvancedSettingsDialog(const GifSettings& settings, QWidget* parent)
    : QDialog(parent), m_working(settings) {
    setWindowTitle("Advanced Effects");
    resize(520, 680);
    m_form = new SettingsForm(&m_working, this);

    auto* content = new QWidget;
    auto* contentLayout = new QVBoxLayout(content);
    for (const auto& g : groups()) {
        auto* box = new QGroupBox(g.title);
        auto* grid = new QGridLayout(box);
        grid->setColumnStretch(1, 1);
        for (const auto& key : g.keys) m_form->addRow(grid, key);
        contentLayout->addWidget(box);
    }
    contentLayout->addStretch();
    auto* scroll = new QScrollArea;
    scroll->setWidgetResizable(true);
    scroll->setWidget(content);

    auto* hint = new QLabel("Changes show in the preview straight away. OK keeps them, Cancel restores the previous settings.");
    hint->setObjectName("hintLabel");
    hint->setWordWrap(true);

    auto* randomButton = new QPushButton("Cosmic Chaos");
    randomButton->setObjectName("dialogButton");
    randomButton->setToolTip("Randomise the effects on this page (sensible ranges). Output size, frames and spin are left alone.");
    auto* resetButton = new QPushButton("Effect defaults");
    resetButton->setObjectName("dialogButton");
    resetButton->setToolTip("Reset only the settings on this page to their defaults.");
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel);

    auto* bottom = new QHBoxLayout;
    bottom->addWidget(randomButton);
    bottom->addWidget(resetButton);
    bottom->addStretch();
    bottom->addWidget(buttons);

    auto* layout = new QVBoxLayout(this);
    layout->addWidget(hint);
    layout->addWidget(scroll, 1);
    layout->addLayout(bottom);

    connect(m_form, &SettingsForm::changed, this, [this] { emit settingsChanged(m_working); });
    connect(randomButton, &QPushButton::clicked, this, &AdvancedSettingsDialog::randomise);
    connect(resetButton, &QPushButton::clicked, this, &AdvancedSettingsDialog::resetToDefaults);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
}

void AdvancedSettingsDialog::randomise() {
    auto* rng = QRandomGenerator::global();
    auto uni = [rng](double lo, double hi) { return lo + (hi - lo) * rng->generateDouble(); };
    auto pick = [rng](int lo, int hi) { return int(rng->bounded(lo, hi + 1)); }; // inclusive
    auto chance = [rng](double p) { return rng->generateDouble() < p; };
    GifSettings& s = m_working;

    s.zoom_amplitude = uni(0.10, 0.40);
    s.zoom_cycles = pick(1, 3);
    s.zoom_midpoint = uni(0.85, 1.10);
    s.tunnel_steps = pick(1, 3);

    s.saturation_pulse = uni(0.2, 0.8);
    s.saturation_pulses = pick(1, 4);
    s.invert_flashes = chance(0.25) ? pick(1, 4) : 0;

    s.kaleidoscope_mode = chance(0.4) ? KaleidoscopeMode::None : static_cast<KaleidoscopeMode>(pick(1, 4));
    s.kaleidoscope_turns = pick(0, 2);

    s.star_pattern = static_cast<StarPattern>(pick(0, 2));
    s.num_stars = pick(100, 500);
    s.star_seed = pick(1, 99999);

    s.wave_direction = static_cast<WaveDirection>(pick(0, 2));
    s.wave_amplitude = uni(1.0, 10.0);
    s.wave_frequency = uni(0.03, 0.20);
    s.wave_cycles = pick(1, 3);

    s.pixelation_level = chance(0.2) ? pick(2, 6) : 0;
    s.blur_radius = chance(0.3) ? uni(0.3, 1.5) : 0.0;
    s.vignette_strength = uni(0.3, 1.0);

    clampSettings(s);
    m_form->load();
    emit settingsChanged(m_working);
}

void AdvancedSettingsDialog::resetToDefaults() {
    const GifSettings defaults;
    for (const QString& key : keys())
        if (const FieldInfo* f = findField(key)) f->set(m_working, f->get(defaults));
    m_form->load();
    emit settingsChanged(m_working);
}
