#!/usr/bin/env python3
"""Exercise the real Vega profile command with private persistent GSettings.

No display or user settings are accessed. Requires a built Vega binary and the
Sheliak schemas; tests save/restore, favorites isolation and fail-closed reads.
"""
import argparse
import ast
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', type=Path, required=True)
parser.add_argument('--schemas', type=Path, required=True)
parser.add_argument('--suite-helper', type=Path)
args = parser.parse_args()
binary = str(args.binary.resolve())
checks = []
with tempfile.TemporaryDirectory(prefix='sheliak-pins-profile-contract-' if args.suite_helper else 'vega-profile-contract-') as tmp:
    root = Path(tmp)
    env = os.environ.copy()
    for key in ['DISPLAY', 'WAYLAND_DISPLAY', 'DBUS_SESSION_BUS_ADDRESS', 'DBUS_SYSTEM_BUS_ADDRESS']:
        env.pop(key, None)
    for key, directory in [('XDG_CONFIG_HOME', 'config'), ('XDG_DATA_HOME', 'data'),
                           ('XDG_CACHE_HOME', 'cache'), ('XDG_RUNTIME_DIR', 'run'), ('XDG_STATE_HOME', 'state'), ('HOME', 'home')]:
        (root/directory).mkdir(mode=0o700)
        env[key] = str(root/directory)
    env['GSETTINGS_BACKEND'] = 'keyfile'
    extension = root/'data/gnome-shell/extensions/sheliak@lyraos.com.br'
    extension.mkdir(parents=True)
    (extension/'metadata.json').write_text('{}')
    shutil.copytree(args.schemas.resolve(), extension/'schemas')
    production_ids = ['sheliak@lyraos.com.br']
    if args.suite_helper:
        helper = args.suite_helper.resolve()
        env.update(SHELIAK_PRIVATE_NATIVE_TEST='1', LYRA_NATIVE_SUITE_HELPER=str(helper))
        production_ids = [f'{role}@lyraos.com.br' for role in
            ['dock', 'panel', 'menus', 'search', 'animations', 'desktop-icons']]
        for uuid in production_ids:
            directory = extension.parent / uuid
            directory.mkdir()
            (directory / 'metadata.json').write_text(json.dumps({'uuid': uuid, 'lyra-suite-api': 1}))
            shutil.copytree(args.schemas.resolve(), directory / 'schemas')
        extension = extension.parent / 'dock@lyraos.com.br'
        desktop = extension.parent / 'desktop-icons@lyraos.com.br'
        shutil.copy2(helper.parent.parent / 'extensions/desktop-icons/schemas/org.gnome.shell.extensions.lyra-desktop-icons.gschema.xml', desktop / 'schemas')
        shutil.copytree(helper.parent / 'legacy-schemas', desktop / 'legacy-schemas')
        for directory in [desktop / 'schemas', desktop / 'legacy-schemas']:
            subprocess.run(['glib-compile-schemas', '--strict', str(directory)], check=True)
    schema = 'org.gnome.shell.extensions.sheliak'
    def run(command, *, ok=True, overrides=None):
        result = subprocess.run(command, env=env | (overrides or {}), text=True,
                                capture_output=True, timeout=15)
        assert (result.returncode == 0) == ok, (command, result.returncode, result.stdout, result.stderr)
        return result.stdout.strip()
    def settings(action, key, value=None, shell=False):
        command = ['gsettings', '--schemadir', str(extension/'schemas'), action,
                   'org.gnome.shell' if shell else schema, key]
        if value is not None: command.append(value)
        return run(command)
    def read(key, shell=False):
        value = settings('get', key, shell=shell)
        if value in ['true', 'false']: return value == 'true'
        return ast.literal_eval(value.removeprefix('@as ').removeprefix('uint32 '))
    def check(name, value):
        assert value, name
        checks.append(name)
    def choose(id):
        check('apply '+id, run([binary, '--desktop-profile', 'set', id]) == id)
        check('read '+id, run([binary, '--desktop-profile', 'get']) == id)
    settings('set', 'enabled-extensions', str(production_ids + ['unrelated@fixture']), shell=True)
    settings('set', 'disabled-extensions', "['disabled@fixture']", shell=True)
    settings('set', 'disable-user-extensions', 'false', shell=True)
    settings('set', 'favorite-apps', "['original.desktop']", shell=True)
    for key in ['windows10-panel-apps', 'windows11-panel-apps', 'windows10-menu-apps', 'windows11-menu-apps']:
        settings('set', key, "['"+key+".desktop']")
    settings('set', 'windows10-tile-sizes', "{'windows10-menu-apps.desktop': 'large'}")
    settings('set', 'desktop-profile', 'lyra')
    settings('set', 'position', 'right')
    settings('set', 'icon-size', '48')
    choose('lyra')
    choose('ubuntu')
    check('Ubuntu left/full/start', read('position') == 'left' and read('extend-to-edges') and read('extended-content-alignment') == 'start')
    settings('set', 'icon-size', '44')
    choose('windows10')
    check('Windows 10 bottom/start', read('position') == 'bottom' and read('extended-content-alignment') == 'start')
    settings('set', 'icon-size', '36')
    choose('windows11')
    check('Windows 11 bottom/center', read('position') == 'bottom' and read('extended-content-alignment') == 'center')
    settings('set', 'icon-size', '40')
    choose('macos')
    check('MacOS X bottom floating center', read('position') == 'bottom' and not read('extend-to-edges') and read('content-alignment') == 'center')
    check('MacOS X magnification enabled', read('animation'))
    check('MacOS X top bar and L menu', not read('floating-panel') and read('show-applications-menu') and read('panel-menu-position') == 'left')
    settings('set', 'icon-size', '52')
    choose('vanilla')
    check('Vanilla preserves unrelated extensions', set(read('enabled-extensions', True)) == set(['unrelated@fixture'] + (['desktop-icons@lyraos.com.br'] if args.suite_helper else [])))
    choose('windows11')
    check('Windows 11 customized size restored', read('icon-size') == 40)
    choose('macos')
    check('MacOS X customized size restored', read('icon-size') == 52)
    choose('lyra')
    check('Lyra customized settings restored', read('icon-size') == 48 and read('position') == 'right')
    choose('windows10')
    check('Windows 10 customized size restored', read('icon-size') == 36)
    choose('ubuntu')
    check('Ubuntu customized size restored', read('icon-size') == 44)
    check('GNOME favorites preserved', read('favorite-apps', True) == ['original.desktop'])
    for key in ['windows10-panel-apps', 'windows11-panel-apps', 'windows10-menu-apps', 'windows11-menu-apps']:
        check(key+' preserved', read(key) == [key+'.desktop'])
    check('tile sizes preserved', read('windows10-tile-sizes') == {'windows10-menu-apps.desktop': 'large'})
    check('unrelated disabled extensions preserved', read('disabled-extensions', True) == ['disabled@fixture'])
    before = (root/'config/glib-2.0/settings/keyfile').read_bytes()
    run([binary, '--desktop-profile', 'set', 'invalid'], ok=False)
    check('invalid profile has no effects', before == (root/'config/glib-2.0/settings/keyfile').read_bytes())
    settings('set', 'desktop-profile-settings', 'broken')
    run([binary, '--desktop-profile', 'set', 'lyra'], ok=False)
    check('corrupt saved preferences leave current profile intact', run([binary, '--desktop-profile', 'get']) == 'ubuntu')
    settings('set', 'disable-user-extensions', 'true', shell=True)
    check('disabled Shell backend does not invent profile', run([binary, '--desktop-profile', 'get'], ok=False) == '')
    settings('set', 'disable-user-extensions', 'false', shell=True)
    # A preferred user extension without schemas must not fall through to the
    # global extension and silently report a different installation.
    (extension/'schemas').rename(extension/'missing-schemas')
    check('missing schemas rejected', run([binary, '--desktop-profile', 'get'], ok=False) == '')
print(json.dumps({'passed': len(checks), 'checks': checks}, indent=2))
