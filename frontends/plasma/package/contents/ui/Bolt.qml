import QtQuick
import QtQuick.Shapes
import org.kde.kirigami as Kirigami

// Charging bolt drawn as a shape, so it does not depend on the icon theme.
Shape {
    id: bolt

    property int size: Kirigami.Units.iconSizes.small
    property color color: Kirigami.Theme.textColor

    width: size
    height: size
    preferredRendererType: Shape.CurveRenderer

    ShapePath {
        strokeColor: "transparent"
        fillColor: bolt.color
        joinStyle: ShapePath.RoundJoin
        PathPolyline {
            path: {
                const s = bolt.size
                const p = [[0.60, 0.03], [0.20, 0.57], [0.47, 0.57], [0.38, 0.97], [0.80, 0.39], [0.53, 0.39], [0.60, 0.03]]
                return p.map(q => Qt.point(q[0] * s, q[1] * s))
            }
        }
    }
}
