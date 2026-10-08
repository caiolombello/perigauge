// Peripherals Battery — unified indicator for Logitech MX Keys + Keychron M6.
// GNOME 50 (ESM). Reads both batteries from the `perif-battery` helper.
//
// SPDX-License-Identifier: MIT

import Clutter from 'gi://Clutter';
import GObject from 'gi://GObject';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';

const _MS = 1000;
const REFRESH_INTERVAL = 60 * _MS;  // panel refresh cadence
const LAYOUT_CHECK_INTERVAL = 5 * _MS;
const COMPACT_CHILDREN_THRESHOLD = 5;
const PANEL_WIDTH_TOLERANCE = 8;

const MOUSE_ICON = 'input-mouse-symbolic';
const KEYBOARD_ICON = 'input-keyboard-symbolic';

function batteryIcon(level, charging) {
    if (level == null)
        return 'battery-missing-symbolic';
    const suffix = charging === true ? '-charging' : '';
    if (level >= 80) return `battery-full${suffix}-symbolic`;
    if (level >= 50) return `battery-good${suffix}-symbolic`;
    if (level >= 20) return `battery-low${suffix}-symbolic`;
    if (level >= 5)  return `battery-caution${suffix}-symbolic`;
    return `battery-empty${suffix}-symbolic`;
}

function levelColor(level, stale) {
    if (stale) return '#888';
    if (level == null) return '#888';
    if (level < 20) return '#e01b24'; // red
    if (level < 40) return '#e5a50a'; // orange
    return null;                       // default
}

// relative time like "há 3 min"
function relTime(ts, now) {
    if (!ts) return '—';
    const sec = Math.max(0, Math.floor(now - ts));
    if (sec < 60) return 'agora';
    const min = Math.floor(sec / 60);
    if (min < 60) return `há ${min} min`;
    const h = Math.floor(min / 60);
    return `há ${h} h`;
}

const PerifIndicator = GObject.registerClass(
class PerifIndicator extends PanelMenu.Button {
    _init() {
        super._init(0.0, 'Peripherals Battery', false);
        this._data = null;
        this._lastUpdate = null;
        this._compact = false;
        this._layoutCheckId = null;
        this._rightBox = Main.panel?._rightBox ?? null;
        this._rightBoxSignal = 0;

        const box = new St.BoxLayout({
            style_class: 'perif-panel-box',
            reactive: true,
            can_focus: false,
            track_hover: true,
        });
        this._panelBox = box;

        this._summaryIcon = new St.Icon({
            icon_name: 'battery-missing-symbolic',
            style_class: 'system-status-icon perif-panel-summary',
        });
        this._summaryIcon.visible = false;
        box.add_child(this._summaryIcon);

        // Mouse (M6)
        this._mouseIcon = new St.Icon({icon_name: MOUSE_ICON, style_class: 'system-status-icon perif-panel-icon'});
        this._mouseLabel = new St.Label({y_align: Clutter.ActorAlign.CENTER, text: '—', style_class: 'perif-panel-label'});
        box.add_child(this._mouseIcon);
        box.add_child(this._mouseLabel);

        this._panelSeparator = new St.Icon({icon_name: 'go-next-symbolic', style_class: 'perif-panel-sep'});
        this._panelSeparator.visible = false;
        box.add_child(this._panelSeparator);

        // Keyboard (MX Keys)
        this._kbIcon = new St.Icon({icon_name: KEYBOARD_ICON, style_class: 'system-status-icon perif-panel-icon'});
        this._kbLabel = new St.Label({y_align: Clutter.ActorAlign.CENTER, text: '—', style_class: 'perif-panel-label'});
        box.add_child(this._kbIcon);
        box.add_child(this._kbLabel);

        this.add_child(box);
        this._expandedWidth = 0;

        this._buildMenu();
        if (this._rightBox)
            this._rightBoxSignal = this._rightBox.connect('notify::allocation', () => this._scheduleLayoutCheck());
        this._refresh();
        this._loop = GLib.timeout_add(GLib.PRIORITY_DEFAULT, REFRESH_INTERVAL, () => {
            this._refresh();
            return GLib.SOURCE_CONTINUE;
        });
        this._layoutLoop = GLib.timeout_add(GLib.PRIORITY_DEFAULT, LAYOUT_CHECK_INTERVAL, () => {
            this._scheduleLayoutCheck();
            return GLib.SOURCE_CONTINUE;
        });
        this._scheduleLayoutCheck();
    }

    _buildMenu() {
        // Header
        const title = new PopupMenu.PopupMenuItem('Bateria dos Periféricos', {reactive: false});
        title.label.add_style_class_name('perif-menu-title');
        this.menu.addMenuItem(title);

        // Mouse row
        this._menuMouse = this._makeRow(MOUSE_ICON, 'Keychron M6 (mouse)');
        this.menu.addMenuItem(this._menuMouse.section);
        this._menuMouse.section.actor.hide();

        this._menuDeviceSeparator = new PopupMenu.PopupSeparatorMenuItem();
        this.menu.addMenuItem(this._menuDeviceSeparator);
        this._menuDeviceSeparator.actor.hide();

        // Keyboard row
        this._menuKb = this._makeRow(KEYBOARD_ICON, 'Logitech MX Keys (teclado)');
        this.menu.addMenuItem(this._menuKb.section);
        this._menuKb.section.actor.hide();

        this._menuFooterSeparator = new PopupMenu.PopupSeparatorMenuItem();
        this.menu.addMenuItem(this._menuFooterSeparator);
        this._menuFooterSeparator.actor.hide();

        this._menuEmpty = new PopupMenu.PopupMenuItem('Nenhum dispositivo conectado', {reactive: false});
        this._menuEmpty.label.add_style_class_name('perif-menu-sub');
        this.menu.addMenuItem(this._menuEmpty);

        // Footer: last update + refresh
        const footer = new PopupMenu.PopupMenuSection();
        this._menuFooter = new PopupMenu.PopupMenuItem('Atualizado: —', {reactive: false});
        this._menuFooter.label.add_style_class_name('perif-menu-sub');
        footer.addMenuItem(this._menuFooter);
        const refreshItem = new PopupMenu.PopupMenuItem('Atualizar agora');
        refreshItem.connect('activate', () => this._refresh(true));
        footer.addMenuItem(refreshItem);
        this.menu.addMenuItem(footer);
    }

    // Builds a section returning an object with references to its labels so we can update.
    _makeRow(iconName, title) {
        const section = new PopupMenu.PopupMenuSection();
        const item = new PopupMenu.PopupBaseMenuItem({reactive: false, can_focus: false});
        const vbox = new St.BoxLayout({vertical: true, x_expand: true});
        const hbox = new St.BoxLayout({vertical: false, x_expand: true});

        const icon = new St.Icon({icon_name: iconName, icon_size: 20});
        hbox.add_child(icon);

        const labels = new St.BoxLayout({vertical: true, x_expand: true, style: 'margin-left: 10px;'});
        const titleLbl = new St.Label({text: title, style_class: 'perif-menu-title'});
        const subLbl = new St.Label({text: '—', style_class: 'perif-menu-sub'});
        labels.add_child(titleLbl);
        labels.add_child(subLbl);
        hbox.add_child(labels);

        const pct = new St.Label({text: '—', x_align: Clutter.ActorAlign.END, style_class: 'perif-menu-pct'});
        hbox.add_child(pct);
        vbox.add_child(hbox);
        item.add_child(vbox);
        section.addMenuItem(item);
        return {section, sub: subLbl, pct, icon};
    }

    _refresh(force = false) {
        // If a query is in flight, skip (the helper can take ~11s on cold cache).
        if (this._inflight)
            return;
        this._inflight = true;
        const argv = [this._helperPath()];
        try {
            const proc = new Gio.Subprocess({
                argv,
                flags: Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_PIPE,
            });
            proc.init(null);
            proc.communicate_utf8_async(null, null, (p, res) => {
                this._inflight = false;
                try {
                    const [, stdout] = p.communicate_utf8_finish(res);
                    this._apply(stdout);
                } catch (e) {
                    log(`[perif-battery] helper failed: ${e}`);
                    this._applyError();
                }
            });
        } catch (e) {
            this._inflight = false;
            log(`[perif-battery] spawn failed: ${e}`);
            this._applyError();
        }
    }

    _helperPath() {
        const home = GLib.get_home_dir();
        return GLib.build_filenamev([home, '.local', 'bin', 'perif-battery']);
    }

    _apply(stdout) {
        let data;
        try {
            data = JSON.parse(stdout);
        } catch (e) {
            log(`[perif-battery] bad json: ${stdout}`);
            this._applyError();
            return;
        }
        this._data = data;
        this._error = false;
        this._lastUpdate = Date.now() / 1000;
        this._render();
    }

    _applyError() {
        this._data = {};
        this._error = true;
        this._lastUpdate = Date.now() / 1000;
        this._render();
    }

    _render() {
        const data = this._data || {};
        const now = data.now || (Date.now() / 1000);
        const m6 = data.m6 || {};
        const mxk = data.mxkeys || {};

        const mouseVisible = this._deviceVisible(m6);
        const keyboardVisible = this._deviceVisible(mxk);

        // Panel: keep the detailed values ready, then choose the most compact
        // presentation that fits the current status area.
        this._setDevice(this._mouseIcon, this._mouseLabel, m6);
        this._setDevice(this._kbIcon, this._kbLabel, mxk);
        this._updatePanelLayout(mouseVisible, keyboardVisible);

        // Menu — mouse
        if (this._menuMouse) {
            const lvl = m6.level == null ? '—' : `${m6.level}%`;
            this._menuMouse.pct.set_text(lvl);
            let sub;
            if (m6.level == null) {
                sub = m6.error === 'asleep' ? 'dormindo — mexa o mouse' : 'indisponível';
            } else {
                sub = m6.stale ? `última leitura ${relTime(m6.ts, now)} (dormindo)` : `atualizado ${relTime(m6.ts, now)}`;
            }
            this._menuMouse.sub.set_text(sub);
            this._menuMouse.icon.icon_name = batteryIcon(m6.level, false);
            this._menuMouse.section.actor.visible = mouseVisible;
        }
        // Menu — keyboard
        if (this._menuKb) {
            const lvl = mxk.level == null ? '—' : `${mxk.level}%`;
            this._menuKb.pct.set_text(lvl);
            let sub;
            if (mxk.level == null) {
                sub = mxk.error ? String(mxk.error) : 'indisponível';
            } else {
                const status = mxk.status || (mxk.charging ? 'carregando' : 'descarregando');
                sub = `${status} · ${relTime(mxk.ts, now)}${mxk.stale ? ' (cache)' : ''}`;
            }
            this._menuKb.sub.set_text(sub);
            this._menuKb.icon.icon_name = batteryIcon(mxk.level, mxk.charging === true);
            this._menuKb.section.actor.visible = keyboardVisible;
        }
        if (this._menuDeviceSeparator)
            this._menuDeviceSeparator.actor.visible = mouseVisible && keyboardVisible;
        if (this._menuFooterSeparator)
            this._menuFooterSeparator.actor.visible = mouseVisible || keyboardVisible;
        if (this._menuEmpty)
            this._menuEmpty.actor.visible = !mouseVisible && !keyboardVisible;
        // Footer
        if (this._menuFooter)
            this._menuFooter.label.set_text(this._error
                ? 'Erro ao ler — tentando novamente'
                : `Atualizado ${relTime(this._lastUpdate, now)}`);
    }

    _deviceVisible(device) {
        return device?.connected === true && device.level != null;
    }

    _setDevice(icon, label, device) {
        const level = device.level;
        icon.icon_name = batteryIcon(level, device.charging === true);
        const text = level == null ? '—' : `${level}%`;
        label.set_text(text);
        const color = levelColor(level, device.stale);
        if (color)
            label.set_style(`color: ${color};`);
        else
            label.set_style('');
    }

    _updatePanelLayout(mouseVisible, keyboardVisible) {
        const anyVisible = mouseVisible || keyboardVisible;
        if (!anyVisible) {
            this.container.hide();
            this._setCompact(false, false, false);
            return;
        }

        this.container.show();
        if (!this._compact) {
            const [, expandedWidth] = this._panelBox.get_preferred_width(-1);
            this._expandedWidth = Math.max(this._expandedWidth, expandedWidth);
        }
        this._setCompact(this._shouldCompact(), mouseVisible, keyboardVisible);
    }

    _setCompact(compact, mouseVisible, keyboardVisible) {
        this._compact = compact;
        const showDetails = !compact;
        if (compact)
            this._panelBox.add_style_class_name('perif-panel-compact');
        else
            this._panelBox.remove_style_class_name('perif-panel-compact');

        this._mouseIcon.visible = showDetails && mouseVisible;
        this._mouseLabel.visible = showDetails && mouseVisible;
        this._kbIcon.visible = showDetails && keyboardVisible;
        this._kbLabel.visible = showDetails && keyboardVisible;
        this._panelSeparator.visible = showDetails && mouseVisible && keyboardVisible;
        this._summaryIcon.visible = compact && (mouseVisible || keyboardVisible);

        if (!compact)
            return;

        const devices = [];
        if (mouseVisible)
            devices.push(this._data?.m6);
        if (keyboardVisible)
            devices.push(this._data?.mxkeys);
        const lowest = devices.reduce((current, device) => device.level < current.level ? device : current);
        this._summaryIcon.icon_name = batteryIcon(lowest.level, lowest.charging === true);
    }

    _shouldCompact() {
        const rightBox = this._rightBox || Main.panel?._rightBox;
        if (!rightBox)
            return false;

        const visibleChildren = rightBox.get_children().filter(child => child.visible).length;
        if (visibleChildren >= COMPACT_CHILDREN_THRESHOLD)
            return true;

        // GNOME allocates less than the natural width when the panel side is
        // crowded. Compare that allocation with the width this indicator
        // would need in its expanded form, even while it is currently compact.
        const [, rightNaturalWidth] = rightBox.get_preferred_width(-1);
        const [, ownNaturalWidth] = this._panelBox.get_preferred_width(-1);
        const otherNaturalWidth = Math.max(0, rightNaturalWidth - ownNaturalWidth);
        const expandedWidth = otherNaturalWidth + this._expandedWidth;
        return rightBox.width > 0 && expandedWidth > rightBox.width + PANEL_WIDTH_TOLERANCE;
    }

    _measureExpandedWidth() {
        if (this._expandedWidth > 0)
            return;

        const actors = [this._summaryIcon, this._mouseIcon, this._mouseLabel,
            this._panelSeparator, this._kbIcon, this._kbLabel];
        const visibility = actors.map(actor => actor.visible);
        this._summaryIcon.hide();
        actors.slice(1).forEach(actor => actor.show());
        const [, expandedWidth] = this._panelBox.get_preferred_width(-1);
        actors.forEach((actor, index) => actor.visible = visibility[index]);
        this._expandedWidth = expandedWidth;
    }

    _scheduleLayoutCheck() {
        if (this._layoutCheckId)
            return;
        this._layoutCheckId = GLib.idle_add(GLib.PRIORITY_DEFAULT_IDLE, () => {
            this._layoutCheckId = null;
            this._measureExpandedWidth();
            if (this._data)
                this._updatePanelLayout(this._deviceVisible(this._data.m6), this._deviceVisible(this._data.mxkeys));
            return GLib.SOURCE_REMOVE;
        });
    }

    destroy() {
        if (this._loop) {
            GLib.source_remove(this._loop);
            this._loop = null;
        }
        if (this._layoutLoop) {
            GLib.source_remove(this._layoutLoop);
            this._layoutLoop = null;
        }
        if (this._layoutCheckId) {
            GLib.source_remove(this._layoutCheckId);
            this._layoutCheckId = null;
        }
        if (this._rightBox && this._rightBoxSignal) {
            this._rightBox.disconnect(this._rightBoxSignal);
            this._rightBoxSignal = 0;
        }
        super.destroy();
    }
});

export default class PerifBatteryExtension extends Extension {
    enable() {
        this._indicator = new PerifIndicator();
        Main.panel.addToStatusArea('perif-battery', this._indicator);
    }

    disable() {
        this._indicator?.destroy();
        this._indicator = null;
    }
}
