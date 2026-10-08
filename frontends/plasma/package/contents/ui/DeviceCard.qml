import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3

// Present-device card with selectable circular meters or bars, plus reading details.
Rectangle {
    id: card

    required property var host
    required property var dev

    readonly property var inf: host.info(dev)
    readonly property var comps: dev.components ? Array.from(dev.components) : []
    readonly property bool barStyle: host.displayStyle === "bars"
    readonly property color accent: host.severityColor(inf.severity)
    readonly property string summary: dev.name + ", " + inf.pctText + ", " + inf.stateText
        + (inf.stale ? ", " + i18n("last known reading") : "")
        + (inf.error !== "" ? ", " + inf.error : "")

    Layout.fillWidth: true
    implicitHeight: content.implicitHeight + Kirigami.Units.largeSpacing * 2
    radius: Kirigami.Units.cornerRadius
    color: Qt.alpha(Kirigami.Theme.textColor, 0.05)
    border.width: activeFocus ? 2 : 1
    border.color: activeFocus ? Kirigami.Theme.focusColor : Qt.alpha(Kirigami.Theme.textColor, 0.15)

    activeFocusOnTab: true
    Accessible.role: Accessible.ListItem
    Accessible.name: summary
    Accessible.focusable: true

    ColumnLayout {
        id: content
        anchors.fill: parent
        anchors.margins: Kirigami.Units.largeSpacing
        spacing: Kirigami.Units.smallSpacing

        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.largeSpacing

            Kirigami.Icon {
                visible: card.barStyle
                source: card.inf.icon
                implicitWidth: Kirigami.Units.iconSizes.large
                implicitHeight: implicitWidth
                opacity: card.inf.stale ? 0.6 : 1
            }
            ColumnLayout {
                visible: !card.barStyle
                spacing: Kirigami.Units.smallSpacing
                BatteryRing {
                    Layout.preferredWidth: 68
                    Layout.preferredHeight: 68
                    hasValue: card.inf.pct !== null
                    value: hasValue ? card.inf.pct : 0
                    iconName: card.inf.icon
                    severity: card.inf.severity
                    charging: card.inf.charging
                    stale: card.inf.stale
                    estimated: (card.dev.battery || {}).estimated === true
                    accessibleName: card.summary
                }
                PlasmaComponents3.Label {
                    Layout.alignment: Qt.AlignHCenter
                    text: card.inf.pctText
                    font.pointSize: Math.round(Kirigami.Theme.defaultFont.pointSize * 1.7)
                    font.bold: true
                    color: card.accent
                }
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0
                PlasmaComponents3.Label {
                    Layout.fillWidth: true
                    text: card.dev.name
                    font.bold: true
                    elide: Text.ElideRight
                }
                PlasmaComponents3.Label {
                    Layout.fillWidth: true
                    text: i18nc("connection · age", "%1 · updated %2",
                                card.dev.via || card.dev.link || "", card.host.relativeTime(card.dev.updated_at, card.host.now))
                    opacity: 0.7
                    font: Kirigami.Theme.smallFont
                    elide: Text.ElideRight
                }
            }
            PlasmaComponents3.Label {
                visible: card.barStyle
                text: card.inf.pctText
                font.pointSize: Math.round(Kirigami.Theme.defaultFont.pointSize * 1.7)
                font.bold: true
                color: card.accent
            }
        }

        BatteryBar {
            Layout.fillWidth: true
            visible: card.barStyle && card.inf.pct !== null
            value: card.inf.pct === null ? 0 : card.inf.pct
            barColor: card.accent
            opacity: card.inf.stale ? 0.5 : 1
            accessibleName: card.dev.name + " " + card.inf.pctText
        }

        // State: icon shape + text (never colour alone).
        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing
            Kirigami.Icon {
                implicitWidth: Kirigami.Units.iconSizes.small
                implicitHeight: implicitWidth
                source: card.inf.severity === "critical" ? "data-error"
                    : card.inf.severity === "warn" ? "data-warning"
                    : card.inf.stale ? "appointment-soon-symbolic"
                    : card.inf.charging ? "flash-symbolic" : "emblem-ok-symbolic"
                isMask: source === "flash-symbolic" || source === "emblem-ok-symbolic"
                color: Kirigami.Theme.textColor
            }
            PlasmaComponents3.Label {
                Layout.fillWidth: true
                text: card.inf.stateText
                color: card.accent
                font: Kirigami.Theme.smallFont
                elide: Text.ElideRight
            }
        }
        RowLayout {
            Layout.fillWidth: true
            visible: card.inf.stale
            spacing: Kirigami.Units.smallSpacing
            Kirigami.Icon {
                implicitWidth: Kirigami.Units.iconSizes.small
                implicitHeight: implicitWidth
                source: "appointment-soon-symbolic"
                isMask: true
                color: Kirigami.Theme.textColor
            }
            PlasmaComponents3.Label {
                Layout.fillWidth: true
                text: i18n("Last known reading")
                font: Kirigami.Theme.smallFont
                elide: Text.ElideRight
            }
        }
        RowLayout {
            Layout.fillWidth: true
            visible: card.inf.error !== ""
            spacing: Kirigami.Units.smallSpacing
            Kirigami.Icon {
                implicitWidth: Kirigami.Units.iconSizes.small
                implicitHeight: implicitWidth
                source: "dialog-information-symbolic"
                isMask: true
                color: Kirigami.Theme.textColor
            }
            PlasmaComponents3.Label {
                Layout.fillWidth: true
                text: card.inf.error
                font: Kirigami.Theme.smallFont
                wrapMode: Text.WordWrap
            }
        }

        // Earbuds: per-component rows.
        Repeater {
            model: card.comps
            delegate: RowLayout {
                id: compRow
                required property var modelData
                readonly property bool hasPct: card.host.isNum(modelData.percent)
                readonly property int pct: hasPct ? Math.round(modelData.percent) : 0
                readonly property string sev: card.host.severityOf(hasPct ? pct : null, null)
                readonly property bool isCharging: modelData.charging === "charging"
                readonly property string summary: card.host.componentLabel(modelData.id) + " " + (hasPct ? pct + "%" : "—")
                    + (card.inf.stale ? ", " + i18n("last known reading") : isCharging ? ", " + i18n("charging") : "")
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing
                Accessible.role: Accessible.StaticText
                Accessible.name: summary

                PlasmaComponents3.Label {
                    Layout.preferredWidth: Kirigami.Units.gridUnit * 3
                    text: card.host.componentLabel(compRow.modelData.id)
                    font: Kirigami.Theme.smallFont
                }
                BatteryBar {
                    visible: card.barStyle
                    Layout.fillWidth: true
                    Layout.alignment: Qt.AlignVCenter
                    barHeight: Kirigami.Units.smallSpacing
                    value: compRow.pct
                    barColor: card.host.severityColor(compRow.sev)
                }
                Item {
                    visible: !card.barStyle
                    Layout.fillWidth: true
                }
                BatteryRing {
                    visible: !card.barStyle
                    Layout.preferredWidth: 32
                    Layout.preferredHeight: 32
                    hasValue: compRow.hasPct
                    value: compRow.pct
                    iconName: compRow.modelData.id === "case" ? "battery-case" : card.inf.icon
                    severity: compRow.sev
                    charging: compRow.isCharging
                    stale: card.inf.stale
                    accessibleName: compRow.summary
                }
                Kirigami.Icon {
                    visible: card.barStyle
                    implicitWidth: Kirigami.Units.iconSizes.small
                    implicitHeight: implicitWidth
                    source: "flash-symbolic"
                    isMask: true
                    color: Kirigami.Theme.textColor
                    opacity: compRow.isCharging && !card.inf.stale ? 1 : 0
                }
                PlasmaComponents3.Label {
                    Layout.preferredWidth: Kirigami.Units.gridUnit * 2.5
                    horizontalAlignment: Text.AlignRight
                    text: compRow.hasPct ? compRow.pct + "%" : "—"
                    font: Kirigami.Theme.smallFont
                }
            }
        }
    }
}
