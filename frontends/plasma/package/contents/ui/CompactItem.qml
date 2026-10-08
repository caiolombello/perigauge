import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

// One visible battery meter and percentage per device in the panel.
GridLayout {
    id: item

    property string iconName: ""
    property string text: ""
    property string severity: "ok"
    property bool charging: false
    property bool stale: false
    property bool absent: false
    property bool vertical: false
    property string accessibleName: ""
    property real percent: -1
    property bool estimated: false
    property string displayStyle: "rings"

    readonly property int iconSize: Kirigami.Units.iconSizes.smallMedium

    columns: vertical ? 1 : 2
    rowSpacing: 0
    columnSpacing: Kirigami.Units.smallSpacing

    Accessible.role: Accessible.StaticText
    Accessible.name: accessibleName

    BatteryRing {
        displayStyle: item.displayStyle
        visible: item.iconName !== "" || item.text === ""
        Layout.alignment: Qt.AlignHCenter | Qt.AlignVCenter
        Layout.preferredWidth: item.iconSize
        Layout.preferredHeight: item.iconSize
        hasValue: item.percent >= 0 && item.percent <= 100
        value: hasValue ? item.percent : 0
        iconName: item.iconName !== "" ? item.iconName : Qt.resolvedUrl("../icons/perigauge-symbolic-24.svg")
        severity: item.severity
        charging: item.charging
        stale: item.stale
        absent: item.absent
        estimated: item.estimated
        accessibleName: item.accessibleName
    }

    Text {
        visible: item.text !== ""
        Layout.alignment: Qt.AlignHCenter | Qt.AlignVCenter
        Layout.fillWidth: item.vertical
        horizontalAlignment: Text.AlignHCenter
        text: item.text
        color: Kirigami.Theme.textColor
        opacity: item.stale || item.absent ? 0.6 : 1
        font.family: Kirigami.Theme.defaultFont.family
        font.pointSize: item.vertical ? Kirigami.Theme.smallFont.pointSize : Kirigami.Theme.defaultFont.pointSize
        font.bold: item.severity === "critical"
        fontSizeMode: Text.HorizontalFit
        minimumPixelSize: 8
    }
}
