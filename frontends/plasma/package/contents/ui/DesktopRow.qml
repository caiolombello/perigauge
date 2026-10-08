import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3

// Compact one-line-ish entry for the list layout.
Rectangle {
    id: row

    required property var host
    required property var dev

    readonly property var inf: host.info(dev)
    readonly property bool absent: dev.present === false
    readonly property color accent: host.severityColor(inf.severity)
    readonly property var comps: dev.components ? Array.from(dev.components) : []
    readonly property string detail: {
        if (absent) return i18n("Out of range · last seen %1", host.relativeTime(dev.updated_at, host.now))
        if (comps.length > 0) {
            return comps.map(c => host.componentLabel(c.id) + " " + (host.isNum(c.percent) ? Math.round(c.percent) + "%" : "—")
                             + (c.charging === "charging" ? "⚡" : "")).join(" · ")
        }
        let t = inf.stateText
        if (inf.stale) t += " · " + i18n("last known reading")
        if (inf.error !== "") t += " · " + inf.error
        return t
    }

    Layout.fillWidth: true
    implicitHeight: content.implicitHeight + Kirigami.Units.smallSpacing * 2
    radius: Kirigami.Units.cornerRadius
    color: Qt.alpha(Kirigami.Theme.textColor, 0.07)
    border.width: activeFocus ? 2 : 1
    border.color: activeFocus ? Kirigami.Theme.focusColor : Qt.alpha(Kirigami.Theme.textColor, 0.15)
    opacity: absent ? 0.6 : 1

    activeFocusOnTab: true
    Accessible.role: Accessible.ListItem
    Accessible.name: host.a11y(dev)
    Accessible.focusable: true

    RowLayout {
        id: content
        anchors.fill: parent
        anchors.margins: Kirigami.Units.smallSpacing
        anchors.leftMargin: Kirigami.Units.largeSpacing
        anchors.rightMargin: Kirigami.Units.largeSpacing
        spacing: Kirigami.Units.largeSpacing

        Kirigami.Icon {
            source: row.inf.icon
            implicitWidth: Kirigami.Units.iconSizes.medium
            implicitHeight: implicitWidth
            opacity: row.inf.stale ? 0.6 : 1
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0
            PlasmaComponents3.Label {
                Layout.fillWidth: true
                text: row.dev.name
                font.bold: true
                elide: Text.ElideRight
            }
            PlasmaComponents3.Label {
                Layout.fillWidth: true
                text: row.detail
                font: Kirigami.Theme.smallFont
                opacity: 0.8
                elide: Text.ElideRight
            }
        }
        BatteryBar {
            visible: row.inf.pct !== null && row.width > Kirigami.Units.gridUnit * 17
            Layout.preferredWidth: Kirigami.Units.gridUnit * 5
            Layout.alignment: Qt.AlignVCenter
            barHeight: Kirigami.Units.smallSpacing
            value: row.inf.pct === null ? 0 : row.inf.pct
            barColor: row.accent
            opacity: row.inf.stale ? 0.5 : 1
        }
        Kirigami.Icon {
            visible: row.inf.stale && !row.absent
            implicitWidth: Kirigami.Units.iconSizes.small
            implicitHeight: implicitWidth
            source: "appointment-soon-symbolic"
            isMask: true
            color: Kirigami.Theme.textColor
        }
        StateBadge {
            visible: !row.absent
            size: Kirigami.Units.iconSizes.smallMedium
            severity: row.inf.severity
            charging: row.inf.charging
        }
        PlasmaComponents3.Label {
            Layout.minimumWidth: Kirigami.Units.gridUnit * 2.5
            horizontalAlignment: Text.AlignRight
            text: row.inf.pctText
            font.bold: true
            font.pointSize: Math.round(Kirigami.Theme.defaultFont.pointSize * 1.2)
            color: row.accent
        }
    }
}
