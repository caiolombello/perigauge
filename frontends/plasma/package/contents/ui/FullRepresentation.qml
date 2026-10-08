import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3
import org.kde.plasma.extras as PlasmaExtras

PlasmaExtras.Representation {
    id: full

    required property var host

    readonly property string udevCommand: "sudo install -m 644 packaging/udev/70-perigauge.rules /etc/udev/rules.d/ && sudo udevadm control --reload && sudo udevadm trigger"
    property bool outExpanded: false

    Layout.minimumWidth: Kirigami.Units.gridUnit * 20
    Layout.preferredWidth: Kirigami.Units.gridUnit * 23
    Layout.minimumHeight: Kirigami.Units.gridUnit * 12
    Layout.preferredHeight: Kirigami.Units.gridUnit * 28
    collapseMarginsHint: true

    header: PlasmaExtras.PlasmoidHeading {
        RowLayout {
            anchors.fill: parent
            spacing: Kirigami.Units.smallSpacing

            Kirigami.Icon {
                source: Qt.resolvedUrl("../icons/perigauge-symbolic-24.svg")
                isMask: true
                color: Kirigami.Theme.textColor
                implicitWidth: Kirigami.Units.iconSizes.smallMedium
                implicitHeight: implicitWidth
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0
                PlasmaExtras.Heading {
                    level: 3
                    text: "PeriGauge"
                }
                PlasmaComponents3.Label {
                    Layout.fillWidth: true
                    font: Kirigami.Theme.smallFont
                    opacity: 0.7
                    elide: Text.ElideRight
                    text: full.host.refreshing ? i18n("Reading…")
                        : (full.host.isNum(full.host.snapshot.generated_at)
                           ? i18n("Last read %1", full.host.relativeTime(full.host.snapshot.generated_at, full.host.now))
                           : i18n("Not read yet"))
                }
            }
            PlasmaComponents3.BusyIndicator {
                running: full.host.refreshing
                visible: running
                implicitWidth: Kirigami.Units.iconSizes.smallMedium
                implicitHeight: implicitWidth
            }
            PlasmaComponents3.ToolButton {
                icon.name: "view-refresh"
                text: i18n("Refresh")
                display: PlasmaComponents3.AbstractButton.IconOnly
                enabled: !full.host.refreshing
                onClicked: full.host.refresh()
                PlasmaComponents3.ToolTip.text: text
                PlasmaComponents3.ToolTip.visible: hovered
                Accessible.name: i18n("Refresh battery readings")
            }
        }
    }

    PlasmaComponents3.ScrollView {
        id: scroll
        anchors.fill: parent
        contentWidth: availableWidth

        ColumnLayout {
            width: scroll.availableWidth
            spacing: Kirigami.Units.smallSpacing

            // Command / parsing error
            Banner {
                visible: full.host.errorMessage !== ""
                message: full.host.errorMessage
                iconName: "data-error"
            }

            // Issues reported by the core
            Repeater {
                model: full.host.issues
                delegate: Banner {
                    required property var modelData
                    message: modelData.detail || modelData.code || ""
                    hint: modelData.code === "hidraw-permission-denied" ? full.udevCommand : ""
                    iconName: "data-warning"
                }
            }

            // Loading
            PlasmaComponents3.BusyIndicator {
                Layout.alignment: Qt.AlignHCenter
                visible: !full.host.hasData && full.host.errorMessage === ""
                running: visible
            }

            // Empty state
            PlasmaExtras.PlaceholderMessage {
                Layout.fillWidth: true
                Layout.topMargin: Kirigami.Units.gridUnit * 2
                visible: full.host.hasData && full.host.presentDevices.length === 0
                         && (full.host.absentDevices.length === 0 || !full.host.showOutOfRange)
                iconName: "battery-missing"
                text: i18n("No battery-powered peripherals found")
                explanation: i18n("Run “perigauge doctor” in a terminal to see what was detected.")
            }
            PlasmaComponents3.Label {
                Layout.fillWidth: true
                visible: full.host.hasData && full.host.presentDevices.length === 0
                         && full.host.absentDevices.length > 0 && full.host.showOutOfRange
                text: i18n("No devices in range.")
                horizontalAlignment: Text.AlignHCenter
                opacity: 0.7
            }

            Repeater {
                model: full.host.presentDevices
                delegate: DeviceCard {
                    required property var modelData
                    host: full.host
                    dev: modelData
                }
            }

            // Out of range (collapsible)
            PlasmaComponents3.ToolButton {
                id: outToggle
                Layout.fillWidth: true
                visible: full.host.showOutOfRange && full.host.absentDevices.length > 0
                icon.name: full.outExpanded ? "arrow-down" : "arrow-right"
                text: i18n("Out of range (%1)", full.host.absentDevices.length)
                checkable: false
                onClicked: full.outExpanded = !full.outExpanded
                Accessible.name: text
                Accessible.description: full.outExpanded ? i18n("Expanded") : i18n("Collapsed")
            }
            Repeater {
                model: full.outExpanded && full.host.showOutOfRange ? full.host.absentDevices : []
                delegate: RowLayout {
                    id: absentRow
                    required property var modelData
                    readonly property var inf: full.host.info(modelData)
                    Layout.fillWidth: true
                    Layout.leftMargin: Kirigami.Units.smallSpacing
                    spacing: Kirigami.Units.largeSpacing
                    opacity: 0.6
                    activeFocusOnTab: true
                    Accessible.role: Accessible.ListItem
                    Accessible.name: modelData.name + ", " + i18n("out of range")
                        + ", " + i18n("last seen %1", full.host.relativeTime(modelData.updated_at, full.host.now))
                        + ", " + inf.pctText

                    Kirigami.Icon {
                        source: absentRow.inf.icon
                        implicitWidth: Kirigami.Units.iconSizes.medium
                        implicitHeight: implicitWidth
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 0
                        PlasmaComponents3.Label {
                            Layout.fillWidth: true
                            text: absentRow.modelData.name
                            elide: Text.ElideRight
                        }
                        PlasmaComponents3.Label {
                            Layout.fillWidth: true
                            font: Kirigami.Theme.smallFont
                            elide: Text.ElideRight
                            text: i18n("last seen %1 · %2", full.host.relativeTime(absentRow.modelData.updated_at, full.host.now), absentRow.inf.pctText)
                        }
                    }
                }
            }
        }
    }
}
