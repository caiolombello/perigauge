import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.plasmoid

MouseArea {
    id: compact

    required property var host

    readonly property bool vertical: Plasmoid.formFactor === PlasmaCore.Types.Vertical
    readonly property var entries: host.compactEntries()
    readonly property bool idle: entries.length === 0

    Layout.minimumWidth: vertical ? Kirigami.Units.iconSizes.small : layout.implicitWidth
    Layout.minimumHeight: vertical ? layout.implicitHeight : Kirigami.Units.iconSizes.small
    Layout.preferredWidth: layout.implicitWidth
    Layout.preferredHeight: layout.implicitHeight

    hoverEnabled: true
    activeFocusOnTab: true
    onClicked: host.expanded = !host.expanded
    Keys.onPressed: event => {
        if (event.key === Qt.Key_Space || event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            host.expanded = !host.expanded
            event.accepted = true
        }
    }
    Accessible.role: Accessible.Button
    Accessible.name: "PeriGauge"
    Accessible.description: host.tooltipText()
    Accessible.onPressAction: host.expanded = !host.expanded

    GridLayout {
        id: layout
        anchors.fill: parent
        columns: compact.vertical ? 1 : Math.max(1, compact.entries.length)
        rowSpacing: Kirigami.Units.smallSpacing
        columnSpacing: Kirigami.Units.largeSpacing

        // Nothing to show: discreet brand icon (with a warning badge on errors).
        CompactItem {
            visible: compact.idle
            Layout.alignment: Qt.AlignCenter
            vertical: compact.vertical
            iconName: ""
            severity: compact.host.errorMessage ? "warn" : "ok"
            stale: false
            opacity: 0.6
            accessibleName: compact.host.tooltipText()
        }

        Repeater {
            model: compact.entries
            delegate: CompactItem {
                required property var modelData
                Layout.alignment: Qt.AlignCenter
                Layout.fillWidth: compact.vertical
                vertical: compact.vertical
                iconName: modelData.icon
                text: modelData.text
                severity: modelData.severity
                charging: modelData.charging
                stale: modelData.stale
                accessibleName: modelData.name + " " + modelData.text
            }
        }
    }
}
