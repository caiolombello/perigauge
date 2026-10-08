pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.plasmoid
import org.kde.plasma.plasma5support as Plasma5Support

PlasmoidItem {
    id: root

    // Fixed commands: no user data is ever interpolated into them.
    readonly property string baseCommand: "/bin/sh -c 'p=\"$HOME/.local/bin/perigauge\"; [ -x \"$p\" ] || p=perigauge; exec \"$p\" status --json"
    readonly property string command: baseCommand + "'"
    readonly property string refreshCommand: baseCommand + " --refresh'"

    property var snapshot: ({})
    property bool hasData: false
    property bool refreshing: false
    property string errorMessage: ""
    property double now: Date.now()

    readonly property var visibleDevices: (snapshot.devices || []).filter(d => d && d.hidden !== true)
    readonly property var presentDevices: visibleDevices.filter(d => d.present !== false)
    readonly property var absentDevices: visibleDevices.filter(d => d.present === false)
    readonly property var issues: snapshot.issues || []
    readonly property bool showOutOfRange: Plasmoid.configuration.showOutOfRange
    readonly property string displayStyle: Plasmoid.configuration.displayStyle === "bars" ? "bars" : "rings"

    // Outside panels (Planar on the desktop, Application in plasmawindowed) the full
    // representation is shown in place; in a panel the compact item plus popup is used.
    readonly property bool onDesktop: Plasmoid.formFactor !== PlasmaCore.Types.Horizontal
                                      && Plasmoid.formFactor !== PlasmaCore.Types.Vertical
    preferredRepresentation: onDesktop ? fullRepresentation : compactRepresentation
    switchWidth: Kirigami.Units.gridUnit * 6
    switchHeight: Kirigami.Units.gridUnit * 6
    Plasmoid.backgroundHints: PlasmaCore.Types.DefaultBackground | PlasmaCore.Types.ConfigurableBackground

    Plasmoid.icon: "perigauge"
    Plasmoid.status: presentDevices.length > 0 || (showOutOfRange && absentDevices.length > 0)
        ? PlasmaCore.Types.ActiveStatus : PlasmaCore.Types.PassiveStatus
    toolTipMainText: "PeriGauge"
    toolTipSubText: tooltipText()

    Plasmoid.contextualActions: [
        PlasmaCore.Action {
            text: root.displayStyle === "bars" ? i18n("Switch to circles (gauge)") : i18n("Switch to battery bars")
            icon.name: "view-refresh"
            onTriggered: root.setDisplayStyle(root.displayStyle === "bars" ? "rings" : "bars")
        }
    ]

    // ---- helpers -----------------------------------------------------
    function isNum(v) { return typeof v === "number" && isFinite(v) }

    function setDisplayStyle(style) {
        Plasmoid.configuration.displayStyle = style
        if (onDesktop) Plasmoid.configuration.desktopLayout = "auto"
    }

    function kindIcon(kind) {
        switch (kind) {
        case "mouse": return "input-mouse"
        case "keyboard": return "input-keyboard"
        case "earbuds": return "audio-headphones"
        case "headset": return "audio-headset"
        case "touchpad": return "input-touchpad"
        case "gamepad": return "input-gamepad"
        default: return "battery"
        }
    }

    function levelWord(level) {
        switch (level) {
        case "critical": return i18n("Critical")
        case "low": return i18n("Low")
        case "good": return i18n("Good")
        case "full": return i18n("Full")
        default: return ""
        }
    }

    function severityOf(pct, level) {
        if (isNum(pct)) return pct <= 10 ? "critical" : (pct <= 20 ? "warn" : "ok")
        if (level === "critical") return "critical"
        if (level === "low") return "warn"
        if (level === "good" || level === "full") return "ok"
        return "unknown"
    }

    function severityColor(sev) {
        switch (sev) {
        case "critical": return Kirigami.Theme.negativeTextColor
        case "warn": return Kirigami.Theme.neutralTextColor
        default: return Kirigami.Theme.textColor
        }
    }

    function componentLabel(id) {
        switch (id) {
        case "left": return i18nc("earbud side", "Left")
        case "right": return i18nc("earbud side", "Right")
        case "case": return i18nc("earbuds charging case", "Case")
        default: return id
        }
    }

    // Normalised view of one device: percentage, severity, charging and texts.
    function info(dev) {
        const bat = dev.battery || {}
        const comps = dev.components ? Array.from(dev.components) : []
        const sides = comps.filter(c => (c.id === "left" || c.id === "right") && isNum(c.percent))
        let pct = null
        if (sides.length > 0) pct = Math.min(...sides.map(c => c.percent))
        else if (isNum(bat.percent)) pct = bat.percent
        if (pct !== null) pct = Math.round(pct)
        const charging = bat.charging === "charging" || sides.some(c => c.charging === "charging")
        const full = !charging && bat.charging === "full"
        const sev = severityOf(pct, bat.level)
        let pctText = "—"
        if (pct !== null) pctText = (bat.estimated === true && sides.length === 0 ? "~" : "") + pct + "%"
        else if (bat.level) pctText = (bat.estimated === true ? "~" : "") + levelWord(bat.level)
        let stateText
        if (charging) {
            stateText = (sev === "warn" || sev === "critical")
                ? i18n("Charging · %1", severityText(sev)) : i18n("Charging")
        } else if (full) {
            stateText = i18n("Fully charged")
        } else {
            stateText = severityText(sev)
        }
        return {
            pct: pct, pctText: pctText, severity: sev, charging: charging, full: full,
            stale: dev.stale === true, error: errorText(dev.error), stateText: stateText,
            icon: kindIcon(dev.kind)
        }
    }

    // Screen-reader summary: "MX Keys, 18 percent, Low, last known reading".
    function a11y(dev) {
        const i = info(dev)
        const bat = dev.battery || {}
        let level
        if (i.pct !== null) level = i18n("%1 percent", i.pct)
        else if (bat.level) level = i18n("%1 level, approximate", levelWord(bat.level))
        else level = i18n("level unknown")
        const parts = [dev.name, level, i.stateText]
        if (dev.present === false) parts.push(i18n("out of range"), i18n("last seen %1", relativeTime(dev.updated_at, now)))
        else if (i.stale) parts.push(i18n("last known reading"))
        if (i.error !== "") parts.push(i.error)
        return parts.join(", ")
    }

    function componentA11y(dev, component) {
        const inf = info(dev)
        const pct = isNum(component.percent) ? Math.round(component.percent) : null
        const parts = [dev.name, componentLabel(component.id),
            pct === null ? i18n("level unknown") : i18n("%1 percent", pct)]
        if (pct !== null) parts.push(severityText(severityOf(pct, null)))
        if (dev.present === false) {
            parts.push(i18n("out of range"), i18n("last seen %1", relativeTime(dev.updated_at, now)))
        } else if (inf.stale) {
            parts.push(i18n("last known reading"))
        } else if (component.charging === "charging") {
            parts.push(i18n("charging"))
        } else if (component.charging === "full") {
            parts.push(i18n("Fully charged"))
        }
        if (inf.error !== "") parts.push(inf.error)
        return parts.join(", ")
    }

    function severityText(sev) {
        switch (sev) {
        case "critical": return i18n("Critical")
        case "warn": return i18n("Low")
        case "ok": return i18n("Good")
        default: return i18n("Unknown level")
        }
    }

    function errorText(code) {
        if (!code) return ""
        switch (code) {
        case "asleep": return i18n("Asleep — move it to wake")
        case "permission-denied": return i18n("Permission denied")
        case "waiting-for-status": return i18n("Waiting for status")
        case "no-battery-info": return i18n("No battery information")
        case "protocol-error": return i18n("Protocol error")
        case "invalid-reading": return i18n("Invalid reading")
        case "gone": return i18n("Device disconnected")
        default:
            if (/^io-\d+$/.test(code)) return i18n("I/O error (%1)", code)
            return code
        }
    }

    function relativeTime(ts, ref) {
        if (!isNum(ts)) return i18n("never")
        const s = Math.max(0, Math.round(ref / 1000 - ts))
        if (s < 60) return i18n("just now")
        if (s < 3600) return i18n("%1 min ago", Math.floor(s / 60))
        if (s < 86400) return i18n("%1 h ago", Math.floor(s / 3600))
        return i18n("%1 d ago", Math.floor(s / 86400))
    }

    function tooltipText() {
        if (errorMessage && !hasData) return errorMessage
        const devices = presentDevices.concat(showOutOfRange ? absentDevices : [])
        if (devices.length === 0) return i18n("No battery-powered peripherals found")
        return devices.map(d => {
            const i = info(d)
            let t = d.name + ": " + i.pctText
            if (d.present === false) t += " · " + i18n("out of range")
            else if (i.stale) t += " · " + i18n("last reading")
            else if (i.charging) t += " · " + i18n("charging")
            return t
        }).join("\n")
    }

    // Shared entries keep component readings identical on the desktop and panel.
    function batteryEntries(devices) {
        const result = []
        for (const dev of devices) {
            const inf = info(dev)
            const parts = dev.components && dev.components.length > 0 ? Array.from(dev.components) : [null]
            for (const part of parts) {
                const pct = part ? (isNum(part.percent) ? Math.round(part.percent) : null) : inf.pct
                const charging = part ? part.charging === "charging" : inf.charging
                const label = part ? componentLabel(part.id) : dev.name
                result.push({
                    name: part ? dev.name + " · " + label : dev.name,
                    deviceName: dev.name, label: label, component: part !== null,
                    pct: pct, text: part ? (pct === null ? "—" : pct + "%") : inf.pctText,
                    icon: part && part.id === "case" ? "battery-case" : inf.icon,
                    severity: part ? severityOf(pct, null) : inf.severity,
                    charging: charging, stale: inf.stale, absent: dev.present === false,
                    estimated: !part && (dev.battery || {}).estimated === true,
                    status: dev.present === false ? i18n("Out of range")
                        : inf.stale ? i18n("Last reading")
                        : charging ? i18n("Charging")
                        : pct === null ? (part ? severityText("unknown") : inf.stateText) : "",
                    description: part ? componentA11y(dev, part) : a11y(dev)
                })
            }
        }
        return result
    }

    // Entries shown in the panel according to the configured mode.
    function compactEntries() {
        let list = batteryEntries(presentDevices.concat(showOutOfRange ? absentDevices : [])).map(e => {
            return Object.assign({}, e, {
                pct: e.pct === null ? 101 : e.pct,
                text: e.component ? e.label + " " + e.text : e.text
            })
        })
        if (list.length === 0) return []
        let mode = Plasmoid.configuration.compactMode
        if (mode === "icon") {
            const rank = { critical: 3, warn: 2, ok: 1, unknown: 0 }
            const worst = list.reduce((a, b) => rank[b.severity] > rank[a.severity] ? b : a)
            return [{ icon: "", text: "", severity: worst.severity,
                      charging: list.some(e => e.charging && !e.stale && !e.absent),
                      stale: list.every(e => e.stale), pct: 101, name: "PeriGauge",
                      description: "PeriGauge, " + worst.description }]
        }
        if (mode === "lowest") {
            const live = list.filter(e => !e.absent)
            const low = (live.length > 0 ? live : list).reduce((a, b) => b.pct < a.pct ? b : a)
            return [low]
        }
        return list
    }

    // ---- data source -------------------------------------------------
    function refresh() {
        if (refreshing) return
        refreshing = true
        executable.connectSource(refreshCommand)
        refreshGuard.restart()
    }

    function consume(out, exitCode, err) {
        const text = (out || "").trim()
        if (text.length === 0) {
            if (exitCode === 127) errorMessage = i18n("perigauge is not installed (looked in ~/.local/bin and PATH)")
            else errorMessage = i18n("perigauge returned no output") + (err ? ": " + err.trim().split("\n")[0] : "")
            return
        }
        let parsed
        try { parsed = JSON.parse(text) } catch (e) {
            errorMessage = i18n("perigauge returned invalid JSON")
            return
        }
        if (parsed === null || typeof parsed !== "object" || parsed.schema !== 1) {
            errorMessage = i18n("Unsupported perigauge output (schema %1, expected 1). Update the widget or perigauge.",
                                parsed && parsed.schema !== undefined ? parsed.schema : "?")
            return
        }
        snapshot = parsed
        hasData = true
        errorMessage = ""
    }

    Plasma5Support.DataSource {
        id: executable
        engine: "executable"
        connectedSources: [root.command]
        interval: Math.max(5, Plasmoid.configuration.updateInterval) * 1000
        onNewData: (sourceName, data) => {
            root.consume(data["stdout"], data["exit code"], data["stderr"])
            if (sourceName === root.refreshCommand) {
                executable.disconnectSource(sourceName)
                root.refreshing = false
                refreshGuard.stop()
            }
        }
    }

    // Prevents a stuck "refreshing" state if the command never reports back.
    Timer {
        id: refreshGuard
        interval: 30000
        onTriggered: {
            executable.disconnectSource(root.refreshCommand)
            root.refreshing = false
        }
    }

    Timer {
        interval: 30000
        running: true
        repeat: true
        onTriggered: root.now = Date.now()
    }

    compactRepresentation: CompactRepresentation { host: root }
    fullRepresentation: root.onDesktop ? desktopComponent : popupComponent

    Component { id: popupComponent; FullRepresentation { host: root } }
    Component { id: desktopComponent; DesktopRepresentation { host: root } }

    PlasmaCore.Action {
        id: refreshAction
        text: i18n("Refresh")
        icon.name: "view-refresh"
        enabled: !root.refreshing
        onTriggered: root.refresh()
    }
    Component.onCompleted: Plasmoid.setInternalAction("refresh", refreshAction)
}
