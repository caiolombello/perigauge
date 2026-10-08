import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

// Equally-sized meters keep every device visible; earbuds expose each component.
GridLayout {
    id: grid

    required property var host
    property var devices: []
    property real columnBasis: width
    readonly property string displayStyle: host.displayStyle
    readonly property var entries: host.batteryEntries(devices)

    columns: {
        const capacity = Math.max(1, Math.floor((columnBasis + columnSpacing) / (96 + columnSpacing)))
        const rows = Math.max(1, Math.ceil(entries.length / capacity))
        return Math.max(1, Math.ceil(entries.length / rows))
    }
    columnSpacing: Kirigami.Units.smallSpacing
    rowSpacing: Kirigami.Units.largeSpacing
    uniformCellWidths: true

    Repeater {
        model: grid.entries
        delegate: ColumnLayout {
            id: tile
            required property var modelData
            Layout.fillWidth: true
            Layout.minimumWidth: 0
            Layout.preferredWidth: 0
            Layout.alignment: Qt.AlignTop
            spacing: Kirigami.Units.smallSpacing
            opacity: modelData.absent ? 0.7 : 1
            activeFocusOnTab: true
            Accessible.role: Accessible.ListItem
            Accessible.name: modelData.description
            Accessible.focusable: true

            BatteryRing {
                displayStyle: grid.displayStyle
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: 68
                Layout.preferredHeight: 68
                hasValue: tile.modelData.pct !== null
                value: hasValue ? tile.modelData.pct : 0
                iconName: tile.modelData.icon
                severity: tile.modelData.severity
                charging: tile.modelData.charging
                stale: tile.modelData.stale
                absent: tile.modelData.absent
                estimated: tile.modelData.estimated
                accessibleName: tile.modelData.description
                Rectangle {
                    anchors.fill: parent
                    anchors.margins: -3
                    radius: grid.displayStyle === "bars" ? Kirigami.Units.cornerRadius : width / 2
                    color: "transparent"
                    border.width: 2
                    border.color: Kirigami.Theme.focusColor
                    visible: tile.activeFocus
                }
            }
            Text {
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignHCenter
                text: tile.modelData.text
                color: Kirigami.Theme.textColor
                font.pixelSize: 20
                font.weight: Font.DemiBold
            }
            Text {
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignHCenter
                text: tile.modelData.label
                color: Kirigami.Theme.textColor
                font: Kirigami.Theme.smallFont
                elide: Text.ElideRight
            }
            Text {
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignHCenter
                text: tile.modelData.component ? tile.modelData.deviceName : tile.modelData.status
                color: Kirigami.Theme.textColor
                opacity: 0.7
                font: Kirigami.Theme.smallFont
                elide: Text.ElideRight
                visible: text !== ""
            }
            Text {
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignHCenter
                text: tile.modelData.status
                color: Kirigami.Theme.textColor
                opacity: 0.7
                font: Kirigami.Theme.smallFont
                elide: Text.ElideRight
                visible: tile.modelData.component && text !== ""
            }
        }
    }
}
