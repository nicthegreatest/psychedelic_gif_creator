// settings_schema.h - one table describing every setting: key, label, tooltip,
// range. Drives the UI rows, QSettings persistence and value clamping, so a
// setting is described in exactly one place (defaults live in GifSettings).
#pragma once

#include "gif_settings.h"
#include <QString>
#include <QStringList>
#include <functional>
#include <vector>

class QSettings;

struct FieldInfo {
    enum class Kind { Int, Double, Bool, Choice };
    QString key;
    QString label;
    QString tooltip;
    Kind kind = Kind::Int;
    double min = 0, max = 1, step = 1;
    int decimals = 0;
    QStringList choices; // Kind::Choice only, index == enum value
    std::function<double(const GifSettings&)> get;
    std::function<void(GifSettings&, double)> set; // clamps and rounds

    double defaultValue() const { return get(GifSettings{}); }
};

const std::vector<FieldInfo>& settingsFields();
const FieldInfo* findField(const QString& key);

void clampSettings(GifSettings& s);
void saveSettings(QSettings& store, const GifSettings& s);
GifSettings loadSettings(QSettings& store);
