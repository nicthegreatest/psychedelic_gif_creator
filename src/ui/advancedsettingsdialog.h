// advancedsettingsdialog.h - effect settings, edited on a copy with live
// preview; OK keeps the changes, Cancel throws them away.
#pragma once

#include "gif_settings.h"
#include <QDialog>

class SettingsForm;

class AdvancedSettingsDialog : public QDialog {
    Q_OBJECT
public:
    AdvancedSettingsDialog(const GifSettings& settings, QWidget* parent = nullptr);
    const GifSettings& settings() const { return m_working; }

    // Keys edited by this dialog (used for "Effect defaults" and randomise).
    static QStringList keys();

signals:
    // Emitted after every change, once the value is stored in settings().
    void settingsChanged(const GifSettings& settings);

private:
    void randomise();
    void resetToDefaults();

    GifSettings m_working;
    SettingsForm* m_form = nullptr;
};
