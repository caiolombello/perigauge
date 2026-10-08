import QtQuick
import QtQuick.Shapes
import org.kde.kirigami as Kirigami

// Battery gauge in the brand shape: a 270° arc open at the bottom. The state badge
// sits in the opening. Stale readings use a dashed track; readings without a
// percentage draw an approximate arc from the level and say "approx.".
Item {
    id: gauge

    property real value: 0              // 0..100, used when hasValue
    property bool hasValue: true
    property string level: ""           // critical|low|good|full, for the approximate arc
    property string severity: "ok"
    property bool charging: false
    property bool stale: false
    property color accent: Kirigami.Theme.textColor
    property string iconName: ""
    property string text: ""            // centre text (percentage or level)
    property bool showIcon: true
    property bool showBadge: true
    property string accessibleName: ""
    // Upper bound so a lone device does not grow the arc to the whole widget.
    property real maxDiameter: Number.POSITIVE_INFINITY

    readonly property real diameter: Math.min(width, height, maxDiameter)
    readonly property real stroke: Math.max(3, Math.min(Math.round(diameter * 0.09), Math.round(Kirigami.Units.gridUnit * 0.9)))
    readonly property real arcRadius: (diameter - stroke) / 2
    readonly property real approxFraction: level === "critical" ? 0.1 : level === "low" ? 0.25 : level === "good" ? 0.65 : level === "full" ? 1 : 0
    readonly property real fraction: Math.max(0, Math.min(1, hasValue ? value / 100 : approxFraction))
    readonly property bool approx: !hasValue && fraction > 0
    readonly property bool roomy: diameter >= Kirigami.Units.gridUnit * 5

    implicitWidth: Kirigami.Units.gridUnit * 6
    implicitHeight: Kirigami.Units.gridUnit * 6

    Accessible.role: Accessible.ProgressBar
    Accessible.name: accessibleName

    Shape {
        anchors.centerIn: parent
        width: gauge.diameter
        height: gauge.diameter

        // Track (dashed for last-known readings)
        ShapePath {
            strokeColor: Qt.alpha(Kirigami.Theme.textColor, gauge.stale ? 0.45 : 0.2)
            strokeWidth: gauge.stroke
            fillColor: "transparent"
            capStyle: gauge.stale ? ShapePath.FlatCap : ShapePath.RoundCap
            strokeStyle: gauge.stale ? ShapePath.DashLine : ShapePath.SolidLine
            dashPattern: [0.5, 1.3]
            PathAngleArc {
                centerX: gauge.diameter / 2
                centerY: gauge.diameter / 2
                radiusX: gauge.arcRadius
                radiusY: gauge.arcRadius
                startAngle: 135
                sweepAngle: 270
            }
        }
        // Value
        ShapePath {
            strokeColor: gauge.accent
            strokeWidth: gauge.stroke
            fillColor: "transparent"
            capStyle: gauge.stale ? ShapePath.FlatCap : ShapePath.RoundCap
            strokeStyle: gauge.approx ? ShapePath.DashLine : ShapePath.SolidLine
            dashPattern: [3, 1.5]
            PathAngleArc {
                centerX: gauge.diameter / 2
                centerY: gauge.diameter / 2
                radiusX: gauge.arcRadius
                radiusY: gauge.arcRadius
                startAngle: 135
                sweepAngle: 270 * gauge.fraction
                Behavior on sweepAngle { NumberAnimation { duration: Kirigami.Units.longDuration; easing.type: Easing.OutCubic } }
            }
        }
        opacity: gauge.stale ? 0.75 : 1
    }

    Column {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: -gauge.diameter * 0.04
        spacing: Math.round(gauge.diameter * 0.02)
        width: gauge.diameter * 0.66

        Kirigami.Icon {
            visible: gauge.showIcon && gauge.iconName !== "" && gauge.diameter >= Kirigami.Units.gridUnit * 4
            anchors.horizontalCenter: parent.horizontalCenter
            width: Math.round(Math.min(gauge.diameter * 0.2, Kirigami.Units.iconSizes.large))
            height: width
            source: gauge.iconName
            opacity: gauge.stale ? 0.6 : 1
        }
        Text {
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            text: gauge.text
            color: gauge.accent
            font.bold: true
            font.pixelSize: Math.round(Math.min(gauge.diameter * 0.24, Kirigami.Units.gridUnit * 3.2))
            fontSizeMode: Text.HorizontalFit
            minimumPixelSize: 8
        }
        Text {
            visible: gauge.approx && gauge.roomy
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            text: i18nc("approximate reading", "approx.")
            color: Kirigami.Theme.textColor
            opacity: 0.75
            font.pixelSize: Math.max(8, Math.min(Math.round(gauge.diameter * 0.09), Kirigami.Units.gridUnit))
            fontSizeMode: Text.HorizontalFit
            minimumPixelSize: 7
        }
    }

    StateBadge {
        visible: gauge.showBadge
        anchors.horizontalCenter: parent.horizontalCenter
        y: (gauge.height - gauge.diameter) / 2 + gauge.diameter * 0.5 + gauge.arcRadius * 0.82 - height / 2
        severity: gauge.severity
        charging: gauge.charging
        size: Math.round(Math.max(Kirigami.Units.iconSizes.small * 0.8, Math.min(gauge.diameter * 0.16, Kirigami.Units.iconSizes.smallMedium)))
    }
}
