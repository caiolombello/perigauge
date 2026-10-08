import QtQuick
import QtQuick.Shapes
import org.kde.kirigami as Kirigami

// State shapes from the brand: ok check, warning triangle, critical octagon with "!",
// charging bolt. Shape + (separate) text carry the meaning, never colour alone.
Row {
    id: badge

    property string severity: "ok"
    property bool charging: false
    property int size: Kirigami.Units.iconSizes.small

    readonly property bool showSeverity: severity === "critical" || severity === "warn"

    spacing: Math.round(size / 4)

    Shape {
        id: sh
        visible: badge.showSeverity
        width: badge.size
        height: badge.size
        preferredRendererType: Shape.CurveRenderer
        readonly property bool critical: badge.severity === "critical"
        readonly property color fill: critical ? Kirigami.Theme.negativeTextColor : Kirigami.Theme.neutralTextColor

        ShapePath {
            strokeColor: "transparent"
            fillColor: sh.fill
            joinStyle: ShapePath.RoundJoin
            PathPolyline {
                path: {
                    const s = badge.size, c = s / 2, pts = []
                    if (sh.critical) {
                        for (let k = 0; k < 8; ++k) {
                            const a = (22.5 + 45 * k) * Math.PI / 180
                            pts.push(Qt.point(c + c * Math.cos(a) / Math.cos(Math.PI / 8), c + c * Math.sin(a) / Math.cos(Math.PI / 8)))
                        }
                    } else {
                        pts.push(Qt.point(c, s * 0.06), Qt.point(s * 0.97, s * 0.92), Qt.point(s * 0.03, s * 0.92))
                    }
                    pts.push(pts[0])
                    return pts
                }
            }
        }
        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            y: sh.critical ? (parent.height - height) / 2 : parent.height * 0.34
            text: "!"
            font.bold: true
            font.pixelSize: Math.round(badge.size * (sh.critical ? 0.72 : 0.58))
            color: Kirigami.Theme.backgroundColor
        }
    }

    Bolt {
        visible: badge.charging
        size: badge.size
    }

    Kirigami.Icon {
        visible: badge.severity === "ok" && !badge.charging
        width: badge.size
        height: badge.size
        source: "emblem-ok-symbolic"
        isMask: true
        color: Kirigami.Theme.textColor
    }
}
