import QtQuick
import org.kde.kirigami as Kirigami

// Thin level bar; the percentage text next to it carries the meaning, not the colour.
Item {
    id: bar

    property real value: 0          // 0..100
    property color barColor: Kirigami.Theme.highlightColor
    property int barHeight: Kirigami.Units.smallSpacing * 2
    property string accessibleName: ""

    implicitHeight: barHeight
    implicitWidth: Kirigami.Units.gridUnit * 6

    Accessible.role: Accessible.ProgressBar
    Accessible.name: accessibleName

    Rectangle {
        anchors.fill: parent
        radius: height / 2
        color: Kirigami.Theme.textColor
        opacity: 0.15
    }
    Rectangle {
        width: bar.width * Math.max(0, Math.min(100, bar.value)) / 100
        height: parent.height
        radius: height / 2
        color: bar.barColor
        Behavior on width { NumberAnimation { duration: Kirigami.Units.shortDuration } }
    }
}
