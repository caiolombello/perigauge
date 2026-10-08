import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3

// Small pill for a secondary device in the gauge layout.
Rectangle {
    id: chip

    required property var host
    required property var dev

    readonly property var inf: host.info(dev)
    readonly property bool absent: dev.present === false
    readonly property color accent: host.severityColor(inf.severity)

    implicitWidth: Math.min(row.implicitWidth + Kirigami.Units.largeSpacing * 2, Kirigami.Units.gridUnit * 12)
    implicitHeight: row.implicitHeight + Kirigami.Units.smallSpacing * 2
    radius: Kirigami.Units.cornerRadius
    color: Qt.alpha(Kirigami.Theme.textColor, 0.07)
    border.width: activeFocus ? 2 : 1
    border.color: activeFocus ? Kirigami.Theme.focusColor : Qt.alpha(Kirigami.Theme.textColor, 0.18)
    opacity: absent ? 0.6 : 1

    activeFocusOnTab: true
    Accessible.role: Accessible.ListItem
    Accessible.name: host.a11y(dev)
    Accessible.focusable: true

    RowLayout {
        id: row
        anchors.fill: parent
        anchors.margins: Kirigami.Units.smallSpacing
        anchors.leftMargin: Kirigami.Units.largeSpacing
        anchors.rightMargin: Kirigami.Units.largeSpacing
        spacing: Kirigami.Units.smallSpacing

        Kirigami.Icon {
            source: chip.inf.icon
            implicitWidth: Kirigami.Units.iconSizes.smallMedium
            implicitHeight: implicitWidth
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0
            PlasmaComponents3.Label {
                Layout.fillWidth: true
                text: chip.dev.name
                font: Kirigami.Theme.smallFont
                elide: Text.ElideRight
            }
            RowLayout {
                spacing: Kirigami.Units.smallSpacing
                PlasmaComponents3.Label {
                    text: chip.inf.pctText
                    font.bold: true
                    color: chip.accent
                }
                StateBadge {
                    visible: !chip.absent
                    size: Kirigami.Units.iconSizes.small
                    severity: chip.inf.severity
                    charging: chip.inf.charging
                }
                Kirigami.Icon {
                    visible: chip.inf.stale && !chip.absent
                    implicitWidth: Kirigami.Units.iconSizes.small
                    implicitHeight: implicitWidth
                    source: "appointment-soon-symbolic"
                    isMask: true
                    color: Kirigami.Theme.textColor
                }
                PlasmaComponents3.Label {
                    visible: chip.absent
                    Layout.fillWidth: true
                    text: i18n("last seen %1", chip.host.relativeTime(chip.dev.updated_at, chip.host.now))
                    font: Kirigami.Theme.smallFont
                    elide: Text.ElideRight
                }
            }
        }
    }
}
