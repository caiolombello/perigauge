import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3

// Card with a gauge arc for the grid layout; earbuds also show Left/Right/Case bars.
Rectangle {
    id: card

    required property var host
    required property var dev

    readonly property var inf: host.info(dev)
    readonly property bool absent: dev.present === false
    readonly property color accent: host.severityColor(inf.severity)
    readonly property var comps: dev.components ? Array.from(dev.components) : []
    readonly property string statusLine: absent
        ? i18n("Out of range · last seen %1", host.relativeTime(dev.updated_at, host.now))
        : inf.stateText

    Layout.fillWidth: true
    Layout.minimumWidth: 0
    Layout.preferredWidth: 0
    implicitHeight: content.implicitHeight + Kirigami.Units.largeSpacing * 2
    radius: Kirigami.Units.cornerRadius
    color: Qt.alpha(Kirigami.Theme.textColor, 0.07)
    border.width: activeFocus ? 2 : 1
    border.color: activeFocus ? Kirigami.Theme.focusColor : Qt.alpha(Kirigami.Theme.textColor, 0.15)
    opacity: absent ? 0.6 : 1

    activeFocusOnTab: true
    Accessible.role: Accessible.ListItem
    Accessible.name: host.a11y(dev)
    Accessible.focusable: true

    ColumnLayout {
        id: content
        anchors.fill: parent
        anchors.margins: Kirigami.Units.largeSpacing
        spacing: Kirigami.Units.smallSpacing

        PlasmaComponents3.Label {
            Layout.fillWidth: true
            text: card.dev.name
            font.bold: true
            horizontalAlignment: Text.AlignHCenter
            elide: Text.ElideRight
        }
        GaugeArc {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: Kirigami.Units.gridUnit * 6.5
            Layout.preferredHeight: Kirigami.Units.gridUnit * 6.5
            hasValue: card.inf.pct !== null
            value: card.inf.pct === null ? 0 : card.inf.pct
            level: (card.dev.battery || {}).level || ""
            severity: card.inf.severity
            charging: card.inf.charging
            showBadge: !card.absent
            stale: card.inf.stale || card.absent
            accent: card.accent
            iconName: card.inf.icon
            text: card.inf.pctText
            accessibleName: card.host.a11y(card.dev)
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing
            Kirigami.Icon {
                visible: card.inf.stale && !card.absent
                implicitWidth: Kirigami.Units.iconSizes.small
                implicitHeight: implicitWidth
                source: "appointment-soon-symbolic"
                isMask: true
                color: Kirigami.Theme.textColor
            }
            PlasmaComponents3.Label {
                Layout.fillWidth: true
                text: card.statusLine + (card.inf.stale && !card.absent ? " · " + i18n("last known reading") : "")
                horizontalAlignment: Text.AlignHCenter
                font: Kirigami.Theme.smallFont
                wrapMode: Text.WordWrap
                maximumLineCount: 2
                elide: Text.ElideRight
            }
        }
        PlasmaComponents3.Label {
            visible: card.inf.error !== "" && !card.absent
            Layout.fillWidth: true
            text: card.inf.error
            horizontalAlignment: Text.AlignHCenter
            font: Kirigami.Theme.smallFont
            opacity: 0.8
            wrapMode: Text.WordWrap
        }

        // Earbuds: Left / Right / Case
        Repeater {
            model: card.comps
            delegate: RowLayout {
                id: compRow
                required property var modelData
                readonly property bool hasPct: card.host.isNum(modelData.percent)
                readonly property int pct: hasPct ? Math.round(modelData.percent) : 0
                readonly property string sev: card.host.severityOf(hasPct ? pct : null, null)
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing
                Accessible.role: Accessible.StaticText
                Accessible.name: card.host.componentLabel(modelData.id) + " " + (hasPct ? i18n("%1 percent", pct) : "—")
                    + (modelData.charging === "charging" ? ", " + i18n("charging") : "")

                PlasmaComponents3.Label {
                    Layout.preferredWidth: Kirigami.Units.gridUnit * 2.5
                    text: card.host.componentLabel(compRow.modelData.id)
                    font: Kirigami.Theme.smallFont
                }
                BatteryBar {
                    Layout.fillWidth: true
                    Layout.alignment: Qt.AlignVCenter
                    barHeight: Kirigami.Units.smallSpacing
                    value: compRow.pct
                    barColor: card.host.severityColor(compRow.sev)
                }
                Bolt {
                    opacity: compRow.modelData.charging === "charging" ? 1 : 0
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
