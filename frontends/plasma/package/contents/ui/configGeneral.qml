import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.kcmutils as KCM

KCM.SimpleKCM {
    id: page

    property int cfg_updateInterval
    property int cfg_updateIntervalDefault
    property string cfg_compactMode
    property string cfg_compactModeDefault
    property bool cfg_showOutOfRange
    property bool cfg_showOutOfRangeDefault
    property string cfg_desktopLayout
    property string cfg_desktopLayoutDefault
    property bool cfg_showTitle
    property bool cfg_showTitleDefault

    readonly property var modes: [
        { value: "auto", text: i18n("Automatic (all if up to 3 devices, otherwise the lowest)") },
        { value: "all", text: i18n("All devices") },
        { value: "lowest", text: i18n("Lowest only") },
        { value: "icon", text: i18n("Icon only") }
    ]

    readonly property var desktopLayouts: [
        { value: "auto", text: i18n("Automatic (by widget size)") },
        { value: "gauge", text: i18n("Gauge (lowest device, others as chips)") },
        { value: "list", text: i18n("List") },
        { value: "grid", text: i18n("Grid of gauges") }
    ]

    Kirigami.FormLayout {
        QQC2.SpinBox {
            id: intervalSpin
            Kirigami.FormData.label: i18n("Refresh every:")
            from: 5
            to: 3600
            stepSize: 5
            editable: true
            value: page.cfg_updateInterval
            onValueModified: page.cfg_updateInterval = value
            textFromValue: (value) => i18n("%1 s", value)
            valueFromText: (text) => parseInt(text) || 30
        }

        QQC2.ComboBox {
            id: modeCombo
            Kirigami.FormData.label: i18n("Panel display:")
            Layout.preferredWidth: Kirigami.Units.gridUnit * 22
            model: page.modes
            textRole: "text"
            valueRole: "value"
            currentIndex: Math.max(0, page.modes.findIndex(m => m.value === page.cfg_compactMode))
            onActivated: page.cfg_compactMode = currentValue
        }

        QQC2.CheckBox {
            Kirigami.FormData.label: i18n("Popup and desktop:")
            text: i18n("Show devices that are out of range")
            checked: page.cfg_showOutOfRange
            onToggled: page.cfg_showOutOfRange = checked
        }

        QQC2.ComboBox {
            Kirigami.FormData.label: i18n("Desktop layout:")
            Layout.preferredWidth: Kirigami.Units.gridUnit * 22
            model: page.desktopLayouts
            textRole: "text"
            valueRole: "value"
            currentIndex: Math.max(0, page.desktopLayouts.findIndex(m => m.value === page.cfg_desktopLayout))
            onActivated: page.cfg_desktopLayout = currentValue
        }

        QQC2.CheckBox {
            Kirigami.FormData.label: i18n("Desktop header:")
            text: i18n("Show the title")
            checked: page.cfg_showTitle
            onToggled: page.cfg_showTitle = checked
        }

        QQC2.Label {
            Kirigami.FormData.isSection: true
            Layout.preferredWidth: Kirigami.Units.gridUnit * 22
            wrapMode: Text.WordWrap
            text: i18n("Notifications, low-battery thresholds and device renaming are configured in the PeriGauge daemon, in ~/.config/perigauge/config.toml.")
        }
    }
}
