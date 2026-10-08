import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3
import org.kde.plasma.extras as PlasmaExtras
import org.kde.plasma.plasmoid

// Always-visible representation for the desktop (Planar). The layout adapts to the
// widget size; the default keeps a battery ring per device always visible.
Item {
    id: desk

    required property var host

    readonly property string udevCommand: "sudo install -m 644 packaging/udev/70-perigauge.rules /etc/udev/rules.d/ && sudo udevadm control --reload && sudo udevadm trigger"
    readonly property real gu: Kirigami.Units.gridUnit
    readonly property string configured: Plasmoid.configuration.desktopLayout
    readonly property var present: host.presentDevices
    readonly property var absent: host.showOutOfRange ? host.absentDevices : []
    readonly property int total: present.length + absent.length
    readonly property string layoutMode: {
        if (host.displayStyle === "bars") return "rings"
        if (configured === "gauge" || configured === "list" || configured === "grid") return configured
        return "rings"
    }
    // Lowest present device is the gauge hero; the rest become chips.
    readonly property var hero: {
        if (present.length === 0) return null
        let best = present[0], bp = 1000
        for (const d of present) {
            const p = host.info(d).pct
            const v = p === null ? 500 : p
            if (v < bp) { best = d; bp = v }
        }
        return best
    }
    readonly property var others: present.filter(d => d !== hero).concat(absent)
    readonly property bool hasContent: host.hasData && total > 0

    Layout.minimumWidth: gu * 14
    Layout.minimumHeight: gu * 10
    Layout.preferredWidth: gu * 26
    Layout.preferredHeight: gu * 20

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Kirigami.Units.smallSpacing
        spacing: Kirigami.Units.smallSpacing

        // ---- header -------------------------------------------------
        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing

            Kirigami.Icon {
                visible: Plasmoid.configuration.showTitle
                source: Qt.resolvedUrl("../icons/perigauge-symbolic-24.svg")
                isMask: true
                color: Kirigami.Theme.textColor
                implicitWidth: Kirigami.Units.iconSizes.small
                implicitHeight: implicitWidth
            }
            PlasmaComponents3.Label {
                visible: Plasmoid.configuration.showTitle
                text: "PeriGauge"
                font.bold: true
                elide: Text.ElideRight
                Layout.maximumWidth: desk.width * 0.5
            }
            PlasmaComponents3.Label {
                Layout.fillWidth: true
                visible: desk.width > desk.gu * 11
                font: Kirigami.Theme.smallFont
                opacity: 0.7
                elide: Text.ElideRight
                text: desk.host.refreshing ? i18n("Reading…")
                    : (desk.host.isNum(desk.host.snapshot.generated_at)
                       ? i18n("Last read %1", desk.host.relativeTime(desk.host.snapshot.generated_at, desk.host.now))
                       : i18n("Not read yet"))
            }
            Item { Layout.fillWidth: true; visible: desk.width <= desk.gu * 11 }
            PlasmaComponents3.BusyIndicator {
                running: desk.host.refreshing
                visible: running
                implicitWidth: Kirigami.Units.iconSizes.small
                implicitHeight: implicitWidth
            }
            PlasmaComponents3.ToolButton {
                icon.name: "view-refresh"
                text: i18n("Refresh")
                display: PlasmaComponents3.AbstractButton.IconOnly
                enabled: !desk.host.refreshing
                opacity: (Plasmoid.configuration.showTitle || hovered || activeFocus) ? 1 : 0.5
                onClicked: desk.host.refresh()
                PlasmaComponents3.ToolTip.text: i18n("Refresh battery readings")
                PlasmaComponents3.ToolTip.visible: hovered
                PlasmaComponents3.ToolTip.delay: Kirigami.Units.toolTipDelay
                Accessible.name: i18n("Refresh battery readings")
            }
        }

        // ---- errors and issues --------------------------------------
        PlasmaComponents3.ScrollView {
            id: banners
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(bannerColumn.implicitHeight, desk.height * 0.4)
            visible: desk.host.errorMessage !== "" || desk.host.issues.length > 0
            contentWidth: availableWidth

            ColumnLayout {
                id: bannerColumn
                width: banners.availableWidth
                spacing: Kirigami.Units.smallSpacing
                Banner {
                    visible: desk.host.errorMessage !== ""
                    message: desk.host.errorMessage
                    iconName: "data-error"
                }
                Repeater {
                    model: desk.host.issues
                    delegate: Banner {
                        required property var modelData
                        message: modelData.detail || modelData.code || ""
                        hint: modelData.code === "hidraw-permission-denied" ? desk.udevCommand : ""
                        iconName: "data-warning"
                    }
                }
            }
        }

        // ---- body ---------------------------------------------------
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            PlasmaComponents3.BusyIndicator {
                anchors.centerIn: parent
                visible: !desk.host.hasData && desk.host.errorMessage === ""
                running: visible
            }

            PlasmaExtras.PlaceholderMessage {
                anchors.centerIn: parent
                width: parent.width - Kirigami.Units.largeSpacing * 2
                visible: desk.host.hasData && !desk.hasContent
                iconName: "battery-missing"
                text: i18n("No battery-powered peripherals found")
                explanation: desk.height > desk.gu * 10 ? i18n("Run “perigauge doctor” in a terminal to see what was detected.") : ""
            }

            PlasmaComponents3.ScrollView {
                id: ringScroll
                anchors.fill: parent
                visible: desk.hasContent && desk.layoutMode === "rings"
                contentWidth: availableWidth
                PlasmaComponents3.ScrollBar.horizontal.policy: PlasmaComponents3.ScrollBar.AlwaysOff

                BatteryGrid {
                    width: ringScroll.availableWidth
                    columnBasis: Math.max(1, ringScroll.width - Kirigami.Units.gridUnit)
                    host: desk.host
                    devices: desk.present.concat(desk.absent)
                }
            }

            // Gauge: hero device + chips
            ColumnLayout {
                anchors.fill: parent
                visible: desk.hasContent && desk.layoutMode === "gauge"
                spacing: Kirigami.Units.smallSpacing

                Item {
                    id: heroBox
                    readonly property var inf: desk.hero ? desk.host.info(desk.hero) : null
                    readonly property bool absentOnly: desk.hero === null
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.minimumHeight: desk.gu * 4
                    visible: !absentOnly
                    activeFocusOnTab: true
                    Accessible.role: Accessible.ListItem
                    Accessible.name: desk.hero ? desk.host.a11y(desk.hero) : ""
                    Accessible.focusable: true

                    Rectangle {
                        anchors.fill: parent
                        visible: heroBox.activeFocus
                        color: "transparent"
                        radius: Kirigami.Units.cornerRadius
                        border.width: 2
                        border.color: Kirigami.Theme.focusColor
                    }
                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: Kirigami.Units.smallSpacing
                        spacing: 0
                        GaugeArc {
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            Layout.minimumHeight: desk.gu * 3
                            maxDiameter: desk.gu * 12
                            visible: heroBox.inf !== null
                            hasValue: heroBox.inf ? heroBox.inf.pct !== null : false
                            value: heroBox.inf && heroBox.inf.pct !== null ? heroBox.inf.pct : 0
                            level: desk.hero ? (desk.hero.battery || {}).level || "" : ""
                            severity: heroBox.inf ? heroBox.inf.severity : "ok"
                            charging: heroBox.inf ? heroBox.inf.charging : false
                            stale: heroBox.inf ? heroBox.inf.stale : false
                            accent: heroBox.inf ? desk.host.severityColor(heroBox.inf.severity) : Kirigami.Theme.textColor
                            iconName: heroBox.inf ? heroBox.inf.icon : ""
                            text: heroBox.inf ? heroBox.inf.pctText : ""
                            accessibleName: desk.hero ? desk.host.a11y(desk.hero) : ""
                        }
                        PlasmaComponents3.Label {
                            Layout.fillWidth: true
                            text: desk.hero ? desk.hero.name : ""
                            font.bold: true
                            horizontalAlignment: Text.AlignHCenter
                            elide: Text.ElideRight
                        }
                        PlasmaComponents3.Label {
                            Layout.fillWidth: true
                            visible: desk.height > desk.gu * 9
                            font: Kirigami.Theme.smallFont
                            opacity: 0.8
                            horizontalAlignment: Text.AlignHCenter
                            wrapMode: Text.WordWrap
                            maximumLineCount: 2
                            elide: Text.ElideRight
                            text: heroBox.inf ? [heroBox.inf.stateText]
                                .concat(heroBox.inf.stale ? [i18n("last known reading")] : [])
                                .concat(heroBox.inf.error !== "" ? [heroBox.inf.error] : []).join(" · ") : ""
                        }
                    }
                }
                PlasmaComponents3.Label {
                    visible: heroBox.absentOnly
                    Layout.fillWidth: true
                    text: i18n("No devices in range.")
                    horizontalAlignment: Text.AlignHCenter
                    opacity: 0.7
                }

                PlasmaComponents3.ScrollView {
                    id: chipScroll
                    visible: desk.others.length > 0
                    Layout.fillWidth: true
                    Layout.fillHeight: heroBox.absentOnly
                    Layout.preferredHeight: Math.min(chipFlow.implicitHeight, desk.height * 0.38)
                    contentWidth: availableWidth

                    Flow {
                        id: chipFlow
                        width: chipScroll.availableWidth
                        spacing: Kirigami.Units.smallSpacing
                        Repeater {
                            model: desk.others
                            delegate: DesktopChip {
                                required property var modelData
                                host: desk.host
                                dev: modelData
                            }
                        }
                    }
                }
            }

            // List
            PlasmaComponents3.ScrollView {
                id: listScroll
                anchors.fill: parent
                visible: desk.hasContent && desk.layoutMode === "list"
                contentWidth: availableWidth

                ColumnLayout {
                    width: listScroll.availableWidth
                    spacing: Kirigami.Units.smallSpacing
                    Repeater {
                        model: desk.present.concat(desk.absent)
                        delegate: DesktopRow {
                            required property var modelData
                            host: desk.host
                            dev: modelData
                        }
                    }
                }
            }

            // Grid
            PlasmaComponents3.ScrollView {
                id: gridScroll
                anchors.fill: parent
                visible: desk.hasContent && desk.layoutMode === "grid"
                contentWidth: availableWidth
                PlasmaComponents3.ScrollBar.horizontal.policy: PlasmaComponents3.ScrollBar.AlwaysOff

                GridLayout {
                    width: gridScroll.availableWidth
                    // Based on the outer width: availableWidth depends on the scrollbar, which
                    // depends on the height, which depends on the columns (binding loop).
                    columns: Math.max(1, Math.floor((gridScroll.width - Kirigami.Units.gridUnit + columnSpacing) / (desk.gu * 13 + columnSpacing)))
                    columnSpacing: Kirigami.Units.smallSpacing
                    rowSpacing: Kirigami.Units.smallSpacing
                    uniformCellWidths: true
                    Repeater {
                        model: desk.present.concat(desk.absent)
                        delegate: DesktopCard {
                            required property var modelData
                            Layout.fillHeight: true
                            host: desk.host
                            dev: modelData
                        }
                    }
                }
            }
        }
    }
}
