// Run via Sheliak's tests/native-pins/run.py --probe in a private compositor.
import Gio from 'gi://Gio';
import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import Shell from 'gi://Shell';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
const wait = ms => new Promise(resolve => GLib.timeout_add(GLib.PRIORITY_DEFAULT, ms,
    () => { resolve(); return GLib.SOURCE_REMOVE; }));
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
const uuid = 'sheliak@lyraos.com.br';
export default class ProfileTest extends Extension {
    enable() {
        this.checks = [];
        this.run().then(() => this.report()).catch(error => this.report(error));
    }
    disable() {}
    report(error) {
        GLib.file_set_contents(GLib.getenv('SHELIAK_NATIVE_RESULT'), JSON.stringify({
            status: !error && this.checks.every(c => c.passed) ? 'passed' : 'failed',
            checks: this.checks, error: error ? String(error) : null, stack: error?.stack,
        }, null, 2));
    }
    check(name, passed) { this.checks.push({name, passed: !!passed}); }
    async command(...args) {
        const process = Gio.Subprocess.new([GLib.getenv('VEGA_PROFILE_TEST_BINARY'), '--desktop-profile', ...args],
            Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_PIPE);
        const [stdout, stderr] = await new Promise((resolve, reject) => {
            process.communicate_utf8_async(null, null, (p, result) => {
                try { const [, out, err] = p.communicate_utf8_finish(result); resolve([out, err]); }
                catch (error) { reject(error); }
            });
        });
        if (!process.get_successful()) throw new Error(stderr);
        return stdout.trim();
    }
    async choose(id) {
        this.check(`${id}: command confirmed`, await this.command('set', id) === id);
        await wait(700);
        this.check(`${id}: reopened client agrees`, await this.command('get') === id);
        const ext = Main.extensionManager.lookup(uuid);
        this.check(`${id}: extension state`, ext.state === (id === 'vanilla' ? 2 : 1));
        if (id !== 'vanilla') {
            this.ext = ext.stateObj;
            this.check(`${id}: rendered dock exists`, !!this.ext._dock);
            this.check(`${id}: stored layout matches`, this.ext._settings.get_string('desktop-profile') === id);
        }
    }
    async capture(id) {
        const path = GLib.build_filenamev([GLib.path_get_dirname(GLib.getenv('SHELIAK_NATIVE_RESULT')), `${id}.png`]);
        const stream = Gio.File.new_for_path(path).replace(null, false, Gio.FileCreateFlags.NONE, null);
        await new Shell.Screenshot().screenshot(false, stream); stream.close(null);
    }
    async run() {
        if (GLib.getenv('SHELIAK_PRIVATE_NATIVE_TEST') !== '1') throw new Error('private fixture required');
        await wait(3500); Main.overview.hide(); await wait(300);
        this.ext = Main.extensionManager.lookup(uuid).stateObj;
        const settings = this.ext._settings;
        const favorites = global.settings.get_strv('favorite-apps');
        settings.set_uint('icon-size', 48);
        for (const id of ['lyra', 'vanilla', 'ubuntu', 'windows10', 'windows11', 'macos', 'lyra']) {
            await this.choose(id);
            if (id === 'macos') {
                const dock = this.ext._dock.actor;
                const [x, y] = dock.get_transformed_position();
                const [width, height] = dock.get_transformed_size();
                const monitor = Main.layoutManager.primaryMonitor;
                this.check('macos: dock floats above bottom', y + height < monitor.y + monitor.height);
                this.check('macos: dock horizontally centered', Math.abs(x + width / 2 - monitor.x - monitor.width / 2) < 3);
                this.check('macos: top bar visible and flush', Main.panel.height > 0 && Main.panel.margin_top === 0);
                const menu = this.ext._panelMenus._applications;
                this.check('macos: L application menu present', !!menu && Main.panel._leftBox.contains(menu.button));
                const labels = [];
                const visit = a => { if (a.text) labels.push(a.text); for (const child of a.get_children()) visit(child); };
                visit(menu.button);
                this.check('macos: logo-only topbar button', !labels.includes(this.ext.gettext('Applications')));
                menu.button.menu.open(); await wait(200);
                this.check('macos: applications menu opens', menu.button.menu.isOpen);
                await this.capture(id); menu.button.menu.close(); await wait(200);
                const pointer = Clutter.get_default_backend().get_default_seat().create_virtual_device(Clutter.InputDeviceType.POINTER_DEVICE);
                const icon = this.ext._dock._icons[0];
                const [ix, iy] = icon.actor.get_transformed_position();
                const [iw, ih] = icon.actor.get_transformed_size();
                pointer.notify_absolute_motion(GLib.get_monotonic_time(), ix + iw/2, iy + ih/2); await wait(500);
                this.check('macos: approved hover magnification active', icon.zoomActor.scale_x > 1.2);
            } else if (id.startsWith('windows')) {
                this.check(`${id}: native Start menu exists`, !!this.ext._windowsPanel?._start);
                this.ext._windowsPanel._start.menu.open(); await wait(300);
                this.check(`${id}: Start opened`, this.ext._windowsPanel._start.menu.isOpen);
                await this.capture(id);
                this.ext._windowsPanel._start.menu.close();
            } else {
                await this.capture(id);
            }
            this.check(`${id}: GNOME favorites preserved`, same(global.settings.get_strv('favorite-apps'), favorites));
        }
        this.check('Lyra restores custom icon size', this.ext._settings.get_uint('icon-size') === 48);
        this.check('unrelated test extension stays enabled', global.settings.get_strv('enabled-extensions').includes(this.uuid));
    }
}
