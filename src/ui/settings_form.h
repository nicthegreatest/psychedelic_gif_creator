// settings_form.h - builds editor rows from the settings schema and keeps a
// GifSettings in sync with them.
#pragma once

#include "settings_schema.h"
#include <QHash>
#include <QObject>
#include <QWidget>

class QDoubleSpinBox;
class QGridLayout;
class QLabel;
class QSlider;

// A slider and a spin box bound together (the slider is omitted when the
// range is too fine-grained to be useful, e.g. a seed).
class SliderSpin : public QWidget {
    Q_OBJECT
public:
    explicit SliderSpin(const FieldInfo& field, QWidget* parent = nullptr);
    double value() const;
    void setValue(double v); // does not emit valueChanged

signals:
    void valueChanged(double value);

private:
    int toSlider(double v) const;
    double m_min, m_step;
    QSlider* m_slider = nullptr;
    QDoubleSpinBox* m_spin = nullptr;
};

class SettingsForm : public QObject {
    Q_OBJECT
public:
    // `target` must outlive the form; the form writes edits straight into it.
    explicit SettingsForm(GifSettings* target, QObject* parent = nullptr);

    void setTarget(GifSettings* target) { m_target = target; load(); }
    void addRow(QGridLayout* grid, const QString& key);
    void load(); // refresh every editor from the target (no signals)
    void setRowVisible(const QString& key, bool visible);
    void setEnabled(bool enabled);
    QStringList keys() const { return m_order; }

    // Shows only the rows that matter for the current values (e.g. wave
    // rows only when a wave direction is chosen).
    void updateVisibility();

signals:
    void changed(const QString& key); // emitted after the value is stored

private:
    struct Row {
        const FieldInfo* field = nullptr;
        QLabel* label = nullptr;
        QWidget* editor = nullptr;
    };
    void store(const QString& key, double value);

    GifSettings* m_target;
    QHash<QString, Row> m_rows;
    QStringList m_order;
};
