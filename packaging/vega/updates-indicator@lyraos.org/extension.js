import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';

const STATE_DIR = '/var/lib/vega';
const STATE_FILE = `${STATE_DIR}/update-status.json`;

class UpdatesIndicator {
    constructor(gettext, extensionPath) {
        this.button = new PanelMenu.Button(0.0, gettext('Atualizações do sistema'), true);
        this._gettext = gettext;
        this._count = null;
        this.button.visible = false;
        this._icon = new St.Icon({
            gicon: Gio.FileIcon.new(Gio.File.new_for_path(
                `${extensionPath}/icons/lyra-updates-symbolic.svg`)),
            style_class: 'system-status-icon',
        });
        this.button.add_child(this._icon);
        this.button.connect('button-press-event', (_actor, event) => {
            if (event.get_button() === Clutter.BUTTON_PRIMARY && this._count > 0) {
                GLib.spawn_command_line_async(
                    "/bin/sh -c 'gapplication action org.lyraos.Vega open-updates || env VEGA_START_PAGE=software /usr/bin/vega-gtk'"
                );
                return Clutter.EVENT_STOP;
            }
            return Clutter.EVENT_PROPAGATE;
        });
        this._refresh();
    }

    _refresh() {
        try {
            const file = Gio.File.new_for_path(STATE_FILE);
            const [ok, contents] = file.load_contents(null);
            if (!ok)
                throw new Error('estado indisponível');
            const status = JSON.parse(new TextDecoder().decode(contents));
            if (status.error)
                throw new Error(status.error);
            this._setCount(Number(status.totalCount));
        } catch (_error) {
            this._setUnknown();
        }
    }

    _setCount(count) {
        this._count = Number.isFinite(count) && count >= 0 ? count : null;
        if (this._count === null) {
            this._setUnknown();
        } else if (this._count > 0) {
            this.button.accessible_name = this._gettext('{count} atualização(ões) disponível(is)')
                .replace('{count}', this._count.toString());
            this.button.visible = true;
        } else {
            this.button.accessible_name = this._gettext('Sistema atualizado');
            this.button.visible = false;
        }
    }

    _setUnknown() {
        this._count = null;
        this.button.accessible_name = this._gettext('Aguardando verificação de atualizações');
        this.button.visible = false;
    }
}

export default class UpdatesIndicatorExtension extends Extension {
    enable() {
        this._indicator = new UpdatesIndicator(message => this.gettext(message), this.path);
        Main.panel.addToStatusArea(this.uuid, this._indicator.button, 0, 'right');

        const directory = Gio.File.new_for_path(STATE_DIR);
        try {
            this._monitor = directory.monitor_directory(Gio.FileMonitorFlags.NONE, null);
            this._monitor.connect('changed', (_monitor, file) => {
                if (file?.get_basename() === 'update-status.json')
                    this._indicator?._refresh();
            });
        } catch (_error) {
            this._monitor = null;
        }
        this._poll = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 60, () => {
            this._indicator?._refresh();
            return GLib.SOURCE_CONTINUE;
        });
    }

    disable() {
        if (this._poll) {
            GLib.source_remove(this._poll);
            this._poll = null;
        }
        this._monitor?.cancel();
        this._monitor = null;
        this._indicator?.button.destroy();
        this._indicator = null;
    }
}
