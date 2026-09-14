// Run through Sheliak's native-pins harness in a disposable GNOME compositor.
import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Shell from 'gi://Shell';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const wait = ms => new Promise(resolve => GLib.timeout_add(GLib.PRIORITY_DEFAULT, ms,
    () => { resolve(); return GLib.SOURCE_REMOVE; }));
const lookup = role => Main.extensionManager.lookup(`${role}@lyraos.com.br`);

export default class UbuntuWorkspacesTest extends Extension {
    enable() {
        if (this.started) return;
        this.started = true;
        this.checks = [];
        this.run().then(() => this.report()).catch(error => this.report(error));
    }
    disable() {}
    report(error) {
        GLib.file_set_contents(GLib.getenv('SHELIAK_NATIVE_RESULT'), JSON.stringify({
            status: error ? 'failed' : 'passed', checks: this.checks,
            error: error ? String(error) : null, stack: error?.stack,
        }, null, 2));
    }
    check(name, passed) {
        this.checks.push({name, passed: !!passed});
        if (!passed) throw Error(name);
    }
    async command(argv) {
        const process = Gio.Subprocess.new(argv, Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_PIPE);
        const [out, err] = await new Promise((resolve, reject) => {
            process.communicate_utf8_async(null, null, (source, result) => {
                try { const [, stdout, stderr] = source.communicate_utf8_finish(result); resolve([stdout, stderr]); }
                catch (error) { reject(error); }
            });
        });
        if (!process.get_successful()) throw Error(`${argv.join(' ')}: ${err}`);
        return out.trim();
    }
    async choose(profile) {
        this.check(`${profile}: Vega confirms profile`,
            await this.command([this.vega, '--desktop-profile', 'set', profile]) === profile);
        await wait(650);
        this.check(`${profile}: reopened client confirms profile`,
            await this.command([this.vega, '--desktop-profile', 'get']) === profile);
    }
    async capture(name) {
        const path = `${GLib.path_get_dirname(GLib.getenv('SHELIAK_NATIVE_RESULT'))}/${name}.png`;
        const stream = Gio.File.new_for_path(path).replace(null, false, Gio.FileCreateFlags.NONE, null);
        await new Shell.Screenshot().screenshot(false, stream);
        stream.close(null);
    }
    async clickActivities() {
        const actor = Main.panel.statusArea.activities;
        this.check('Workspace button is mapped and clickable', actor.mapped && actor.visible);
        const [x, y] = actor.get_transformed_position();
        const [width, height] = actor.get_transformed_size();
        this.pointer.notify_absolute_motion(GLib.get_monotonic_time(), x + width / 2, y + height / 2);
        await wait(100);
        this.pointer.notify_button(GLib.get_monotonic_time(), Clutter.BUTTON_PRIMARY, Clutter.ButtonState.PRESSED);
        this.pointer.notify_button(GLib.get_monotonic_time(), Clutter.BUTTON_PRIMARY, Clutter.ButtonState.RELEASED);
        await wait(600);
    }
    async run() {
        if (GLib.getenv('SHELIAK_PRIVATE_NATIVE_TEST') !== '1') throw Error('Private compositor required');
        this.vega = GLib.getenv('LYRA_NATIVE_VEGA_BINARY');
        const helper = GLib.getenv('LYRA_NATIVE_SUITE_HELPER');
        if (!this.vega || !helper) throw Error('Real Vega binary and suite helper required');
        await wait(2200);
        await this.command([helper, 'migrate']);
        await wait(600);
        Main.overview.hide();
        await wait(350);
        const settings = lookup('dock').stateObj.getSettings('org.gnome.shell.extensions.sheliak');
        const button = () => Main.panel.statusArea.activities.container;
        this.pointer = Clutter.get_default_backend().get_default_seat()
            .create_virtual_device(Clutter.InputDeviceType.POINTER_DEVICE);
        this.check('Lyra initially keeps workspace button hidden', !button().visible);

        await this.choose('ubuntu');
        this.check('Ubuntu workspace preference visible', !settings.get_boolean('hide-workspace-button'));
        this.check('Ubuntu workspace button rendered', button().visible && button().mapped);
        this.check('Ubuntu menus and search remain disabled', lookup('menus').state !== 1 && lookup('search').state !== 1);
        this.check('Ubuntu dock remains extended on left', settings.get_string('position') === 'left' && settings.get_boolean('extend-to-edges'));
        await this.capture('ubuntu-workspaces');
        await this.clickActivities();
        this.check('Click opens native GNOME overview', Main.overview.visible);
        await this.capture('ubuntu-overview');
        await this.clickActivities();
        this.check('Second click closes overview', !Main.overview.visible);

        for (const profile of ['windows10', 'windows11', 'macos', 'lyra']) {
            await this.choose(profile);
            this.check(`${profile}: previous workspace preference preserved`, settings.get_boolean('hide-workspace-button'));
            await this.choose('ubuntu');
            this.check('Return to Ubuntu restores workspace button', button().visible && button().mapped);
        }
        await this.choose('vanilla');
        this.check('Vanilla restores native workspace button', button().visible);
        await this.choose('ubuntu');
        for (const state of ['off', 'on']) {
            await this.command([helper, 'toggle', 'panel', state]);
            await wait(500);
            this.check(`Workspace button remains available with panel ${state}`, button().visible);
        }
        settings.set_boolean('hide-workspace-button', true);
        await wait(250);
        this.check('Explicit Ubuntu customization hides workspace button', !button().visible);
        await this.choose('ubuntu');
        this.check('Re-selection preserves explicit Ubuntu customization', !button().visible);
        settings.set_boolean('hide-workspace-button', false);
        await wait(250);
        await this.choose('lyra');
        this.check('Lyra finishes with its original hidden workspace button', !button().visible);
    }
}
