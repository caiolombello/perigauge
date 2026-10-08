import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

// One panel entry: kind icon + percentage, with shape badges for state.
GridLayout {
    id: item

    property string iconName: ""
    property string text: ""
    property string severity: "ok"
    property bool charging: false
    property bool stale: false
    property bool vertical: false
    property string accessibleName: ""

    readonly property int iconSize: Kirigami.Units.iconSizes.smallMedium
    readonly property string badge: severity === "critical" ? "data-error"
        : (severity === "warn" ? "data-warning" : (charging ? "flash-symbolic" : ""))

    columns: vertical ? 1 : 2
    rowSpacing: 0
    columnSpacing: Kirigami.Units.smallSpacing
    opacity: stale ? 0.6 : 1

    Accessible.role: Accessible.StaticText
    Accessible.name: accessibleName

    Item {
        visible: item.iconName !== "" || item.text === ""
        Layout.alignment: Qt.AlignHCenter | Qt.AlignVCenter
        implicitWidth: item.iconSize
        implicitHeight: item.iconSize

        Kirigami.Icon {
            anchors.fill: parent
            source: item.iconName !== "" ? item.iconName : Qt.resolvedUrl("../icons/perigauge-symbolic-24.svg")
            isMask: item.iconName === ""
            color: Kirigami.Theme.textColor
        }
        Kirigami.Icon {
            width: Math.round(parent.width * 0.55)
            height: width
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            source: item.badge
            visible: item.badge !== ""
        }
        Kirigami.Icon {
            width: Math.round(parent.width * 0.5)
            height: width
            anchors.right: parent.right
            anchors.top: parent.top
            source: "appointment-soon-symbolic"
            color: Kirigami.Theme.textColor
            isMask: true
            visible: item.stale
        }
    }

    Text {
        visible: item.text !== ""
        Layout.alignment: Qt.AlignHCenter | Qt.AlignVCenter
        Layout.fillWidth: item.vertical
        horizontalAlignment: Text.AlignHCenter
        text: item.text
        color: Kirigami.Theme.textColor
        font.pixelSize: item.vertical ? Kirigami.Theme.smallFont.pixelSize : Kirigami.Theme.defaultFont.pixelSize
        font.bold: item.severity === "critical"
        fontSizeMode: Text.HorizontalFit
        minimumPixelSize: 8
    }
}
