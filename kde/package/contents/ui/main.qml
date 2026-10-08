pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.components as PlasmaComponents3
import org.kde.plasma.extras as PlasmaExtras
import org.kde.plasma.plasmoid
import org.kde.plasma.plasma5support as Plasma5Support

PlasmoidItem {
    id: root
    preferredRepresentation: compactRepresentation
    // The fixed shell command expands HOME without interpolating user input.
    readonly property string command: "/bin/sh -c '$HOME/.local/bin/perif-battery'"
    property var batteryData: ({})
    property string errorMessage: ""
    readonly property var mouse: batteryData.m6 || ({})
    readonly property var keyboard: batteryData.mxkeys || ({})
    readonly property bool mouseConnected: mouse.connected === true
    readonly property bool keyboardConnected: keyboard.connected === true
    readonly property int connectedCount: (mouseConnected ? 1 : 0) + (keyboardConnected ? 1 : 0)

    Plasmoid.icon: connectedCount > 0 ? "battery" : "battery-missing"
    Plasmoid.status: connectedCount > 0 ? PlasmaCore.Types.ActiveStatus : PlasmaCore.Types.PassiveStatus
    toolTipMainText: i18n("Bateria de periféricos")
    toolTipSubText: connectedCount > 0 ? summaryText() : i18n("Nenhum periférico conectado")

    function levelText(device) {
        if (device.level !== undefined && device.level !== null) return device.level + "%"
        if (device.level_text) return i18n("aprox. %1", device.level_text)
        return "—"
    }
    function staleText(device) {
        if (device.stale !== true) return ""
        return device.last_read ? i18n(" (última leitura em cache)") : i18n(" (sem leitura atual)")
    }
    function levelColor(device) {
        if (device.stale === true || device.level === undefined || device.level === null)
            return Kirigami.Theme.disabledTextColor
        if (device.level <= 15) return Kirigami.Theme.negativeTextColor
        if (device.level <= 35) return Kirigami.Theme.neutralTextColor
        return Kirigami.Theme.textColor
    }
    function summaryText() {
        const parts = []
        if (mouseConnected) parts.push(i18n("Mouse %1", levelText(mouse)))
        if (keyboardConnected) parts.push(i18n("Teclado %1", levelText(keyboard)))
        return parts.join(" · ")
    }
    function relativeTime(timestamp) {
        if (!timestamp) return i18n("sem leitura")
        const seconds = Math.max(0, Math.round(Date.now() / 1000 - timestamp))
        if (seconds < 60) return i18n("agora")
        if (seconds < 3600) return i18n("há %1 min", Math.floor(seconds / 60))
        return i18n("há %1 h", Math.floor(seconds / 3600))
    }
    function errorText(device) {
        switch (device.error) {
        case "no-dongle": return i18n("Receptor não encontrado")
        case "asleep": return i18n("Dispositivo em repouso; mova-o e atualize")
        case "permission-denied": return i18n("Sem permissão para acessar o receptor")
        case "solaar-not-found": return i18n("Solaar não está instalado ou não está no PATH")
        case "solaar-timeout": return i18n("Solaar não respondeu a tempo")
        case "mxkeys-not-found": return i18n("MX Keys não foi encontrado pelo Solaar")
        case "battery-unavailable": return i18n("O Solaar encontrou o teclado, mas não informou a bateria")
        default:
            if (device.error && device.error.indexOf("open:") === 0)
                return i18n("Não foi possível abrir o receptor (%1)", device.error)
            if (device.error && device.error.indexOf("solaar-exit:") === 0)
                return i18n("O Solaar encerrou com erro (%1)", device.error.slice("solaar-exit:".length))
            return device.error || ""
        }
    }
    function deviceDescription(device, connected) {
        const error = errorText(device)
        if (error) return error + staleText(device)
        if (!connected) return i18n("Não conectado")
        return i18n("%1 · atualizado %2%3", levelText(device), relativeTime(device.ts || device.last_read), staleText(device))
    }
    function refresh() {
        executable.disconnectSource(command)
        executable.connectSource(command)
    }
    function consume(output) {
        try {
            const parsed = JSON.parse(output)
            if (parsed.error) errorMessage = parsed.error
            else { batteryData = parsed; errorMessage = "" }
        } catch (error) {
            errorMessage = i18n("Resposta inválida do leitor")
        }
    }

    Plasma5Support.DataSource {
        id: executable
        engine: "executable"
        connectedSources: [root.command]
        interval: 60000
        onNewData: (sourceName, data) => root.consume(data["stdout"] || "")
    }

    compactRepresentation: RowLayout {
        spacing: Kirigami.Units.smallSpacing
        Layout.minimumWidth: Kirigami.Units.gridUnit * 5
        Layout.preferredWidth: Layout.minimumWidth
        Layout.minimumHeight: Kirigami.Units.gridUnit * 1.5
        RowLayout {
            visible: root.mouseConnected
            spacing: 2
            Kirigami.Icon { source: "input-mouse"; implicitWidth: Kirigami.Units.iconSizes.small; implicitHeight: implicitWidth }
            PlasmaComponents3.Label { text: root.levelText(root.mouse); color: root.levelColor(root.mouse) }
        }
        RowLayout {
            visible: root.keyboardConnected
            spacing: 2
            Kirigami.Icon { source: "input-keyboard"; implicitWidth: Kirigami.Units.iconSizes.small; implicitHeight: implicitWidth }
            PlasmaComponents3.Label { text: root.levelText(root.keyboard); color: root.levelColor(root.keyboard) }
        }
        PlasmaComponents3.Label { visible: root.connectedCount === 0; text: "—"; color: Kirigami.Theme.disabledTextColor }
        MouseArea { anchors.fill: parent; onClicked: root.expanded = !root.expanded }
    }

    fullRepresentation: ColumnLayout {
        implicitWidth: Kirigami.Units.gridUnit * 19
        implicitHeight: content.implicitHeight + Kirigami.Units.largeSpacing * 2
        ColumnLayout {
            id: content
            Layout.fillWidth: true
            Layout.margins: Kirigami.Units.largeSpacing
            spacing: Kirigami.Units.smallSpacing
            PlasmaExtras.Heading { text: i18n("Bateria de periféricos"); level: 3 }
            Repeater {
                model: [
                    { name: i18n("Keychron M6"), icon: "input-mouse", device: root.mouse, connected: root.mouseConnected },
                    { name: i18n("Logitech MX Keys"), icon: "input-keyboard", device: root.keyboard, connected: root.keyboardConnected }
                ]
                delegate: RowLayout {
                    required property var modelData
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.smallSpacing
                    Kirigami.Icon { source: modelData.icon; implicitWidth: Kirigami.Units.iconSizes.medium; implicitHeight: implicitWidth }
                    ColumnLayout {
                        Layout.fillWidth: true
                        PlasmaComponents3.Label { text: modelData.name; font.bold: true }
                        PlasmaComponents3.Label {
                            Layout.fillWidth: true
                            text: root.deviceDescription(modelData.device, modelData.connected)
                            color: modelData.device.error ? Kirigami.Theme.negativeTextColor : (modelData.connected ? root.levelColor(modelData.device) : Kirigami.Theme.disabledTextColor)
                            wrapMode: Text.WordWrap
                        }
                    }
                    PlasmaComponents3.Label { text: modelData.connected ? root.levelText(modelData.device) : "—"; font.bold: true; color: root.levelColor(modelData.device) }
                }
            }
            PlasmaComponents3.Label { visible: root.errorMessage.length > 0; text: root.errorMessage; color: Kirigami.Theme.negativeTextColor; wrapMode: Text.WordWrap; Layout.fillWidth: true }
            QQC2.Button { text: i18n("Atualizar agora"); icon.name: "view-refresh"; onClicked: root.refresh() }
        }
    }

    PlasmaCore.Action { id: refreshAction; text: i18n("Atualizar baterias"); icon.name: "view-refresh"; onTriggered: root.refresh() }
    Component.onCompleted: Plasmoid.setInternalAction("refresh", refreshAction)
}
