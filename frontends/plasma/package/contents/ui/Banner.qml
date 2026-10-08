import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3

// Banner used for errors and issues.
Rectangle {
    id: banner
    property string message: ""
    property string hint: ""
    property string iconName: "data-warning"

    Layout.fillWidth: true
    implicitHeight: bannerContent.implicitHeight + Kirigami.Units.largeSpacing * 2
    radius: Kirigami.Units.cornerRadius
    color: Qt.alpha(Kirigami.Theme.neutralTextColor, 0.15)
    border.width: 1
    border.color: Kirigami.Theme.neutralTextColor
    Accessible.role: Accessible.AlertMessage
    Accessible.name: message

    RowLayout {
        id: bannerContent
        anchors.fill: parent
        anchors.margins: Kirigami.Units.largeSpacing
        spacing: Kirigami.Units.largeSpacing

        Kirigami.Icon {
            Layout.alignment: Qt.AlignTop
            source: banner.iconName
            implicitWidth: Kirigami.Units.iconSizes.smallMedium
            implicitHeight: implicitWidth
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing
            PlasmaComponents3.Label {
                Layout.fillWidth: true
                text: banner.message
                wrapMode: Text.WordWrap
            }
            PlasmaComponents3.Label {
                visible: banner.hint !== ""
                text: i18n("Run:")
                font: Kirigami.Theme.smallFont
            }
            TextEdit {
                id: hintText
                Layout.fillWidth: true
                visible: banner.hint !== ""
                text: banner.hint
                readOnly: true
                selectByMouse: true
                wrapMode: TextEdit.WrapAnywhere
                color: Kirigami.Theme.textColor
                selectionColor: Kirigami.Theme.highlightColor
                selectedTextColor: Kirigami.Theme.highlightedTextColor
                font.family: "monospace"
                font.pointSize: Kirigami.Theme.smallFont.pointSize
                activeFocusOnTab: true
                Accessible.name: banner.hint
            }
            PlasmaComponents3.Button {
                visible: banner.hint !== ""
                icon.name: "edit-copy"
                text: i18n("Copy command")
                onClicked: { hintText.selectAll(); hintText.copy(); hintText.deselect() }
            }
        }
    }
}
