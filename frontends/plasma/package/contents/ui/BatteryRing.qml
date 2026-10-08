import QtQuick
import QtQuick.Shapes
import org.kde.kirigami as Kirigami

// A battery meter with a device icon, shared by panel and desktop.
Item {
    id: ring

    property real value: 0
    property bool hasValue: true
    property string iconName: "battery"
    property string severity: "ok"
    property bool charging: false
    property bool stale: false
    property bool absent: false
    property bool estimated: false
    property string accessibleName: ""
    property string displayStyle: "rings"
    readonly property bool barStyle: displayStyle === "bars"

    readonly property real diameter: Math.min(width, height)
    readonly property real stroke: Math.max(2, diameter * 0.065)
    readonly property real radius: (diameter - stroke) / 2
    readonly property color accent: absent || !hasValue ? Kirigami.Theme.disabledTextColor
        : severity === "critical" ? Kirigami.Theme.negativeTextColor
        : severity === "warn" ? Kirigami.Theme.neutralTextColor
        : Kirigami.Theme.positiveTextColor
    readonly property real fraction: hasValue ? Math.max(0, Math.min(1, value / 100)) : 0
    readonly property string deviceIcon: {
        switch (iconName) {
        case "input-mouse": return Qt.resolvedUrl("../icons/device-mouse.svg")
        case "input-keyboard": return Qt.resolvedUrl("../icons/device-keyboard.svg")
        case "audio-headphones": return Qt.resolvedUrl("../icons/device-earbuds.svg")
        case "battery-case": return Qt.resolvedUrl("../icons/device-case.svg")
        default: return iconName
        }
    }

    implicitWidth: 72
    implicitHeight: 72
    opacity: absent || stale ? 0.6 : 1
    Accessible.role: Accessible.ProgressBar
    Accessible.name: accessibleName

    Shape {
        visible: !ring.barStyle
        anchors.centerIn: parent
        width: ring.diameter
        height: width

        ShapePath {
            strokeColor: Qt.alpha(Kirigami.Theme.textColor, 0.2)
            strokeWidth: ring.stroke
            fillColor: "transparent"
            strokeStyle: ring.stale || ring.absent ? ShapePath.DashLine : ShapePath.SolidLine
            dashPattern: [1.5, 2]
            PathAngleArc {
                centerX: ring.diameter / 2
                centerY: ring.diameter / 2
                radiusX: ring.radius
                radiusY: ring.radius
                startAngle: -90
                sweepAngle: 360
            }
        }
        ShapePath {
            strokeColor: ring.fraction > 0 ? ring.accent : "transparent"
            strokeWidth: ring.stroke
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            strokeStyle: ring.estimated || ring.stale || ring.absent ? ShapePath.DashLine : ShapePath.SolidLine
            dashPattern: [2, 1.5]
            PathAngleArc {
                centerX: ring.diameter / 2
                centerY: ring.diameter / 2
                radiusX: ring.radius
                radiusY: ring.radius
                startAngle: -90
                sweepAngle: 360 * ring.fraction
            }
        }
    }

    BatteryBar {
        objectName: "visualBar"
        visible: ring.barStyle
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: ring.diameter * 0.1
        width: ring.diameter
        barHeight: Math.max(3, Math.round(ring.diameter * 0.15))
        value: ring.hasValue ? ring.value : 0
        barColor: ring.accent
        accessibleName: ring.accessibleName
        Accessible.ignored: true
    }

    Kirigami.Icon {
        anchors.centerIn: parent
        anchors.verticalCenterOffset: ring.barStyle ? -ring.diameter * 0.15 : 0
        width: Math.round(ring.diameter * 0.46)
        height: width
        source: ring.deviceIcon
        isMask: true
        color: Kirigami.Theme.textColor
    }
    Bolt {
        anchors.right: parent.right
        anchors.bottom: ring.barStyle ? parent.verticalCenter : parent.bottom
        visible: ring.charging && !ring.stale && !ring.absent
        size: Math.max(9, Math.round(ring.diameter * 0.23))
        color: ring.accent
    }
    Kirigami.Icon {
        objectName: "severityBadge"
        anchors.right: parent.right
        anchors.top: parent.top
        visible: !ring.hasValue && (ring.severity === "warn" || ring.severity === "critical")
        width: Math.min(18, Math.max(10, Math.round(ring.diameter * 0.5)))
        height: width
        source: ring.severity === "critical" ? "data-error" : "data-warning"
    }
    Kirigami.Icon {
        anchors.right: parent.right
        anchors.bottom: ring.barStyle ? parent.verticalCenter : parent.bottom
        visible: ring.stale || ring.absent
        width: Math.max(9, Math.round(ring.diameter * 0.22))
        height: width
        source: ring.absent ? "network-disconnect-symbolic" : "appointment-soon-symbolic"
        isMask: true
        color: Kirigami.Theme.textColor
    }
}
