import os
from pathlib import Path
import subprocess
import tempfile
import unittest

HELPER = Path(__file__).resolve().parents[1] / 'scripts/obs-repositories.sh'

class ObsRepositories(unittest.TestCase):
    def test_target_uses_the_base_version_on_lyra_and_rejects_unknown_systems(self):
        with tempfile.TemporaryDirectory() as directory:
            system, base = (Path(directory) / name for name in ('os-release', 'base'))
            for distro, version, upstream, expected in [
                ('opensuse-leap', '16.0', '16.1', 'openSUSE_Leap_16.0'),
                ('opensuse-leap', '16.1', '16.0', 'openSUSE_Leap_16.1'),
                ('lyra-os', '1.1', '16.1', 'openSUSE_Leap_16.1'),
                ('opensuse-tumbleweed', '20260915', '16.1', None),
                ('lyra-os', '1.1', '99', None),
            ]:
                system.write_text(f'ID={distro}\nVERSION_ID={version}\n')
                base.write_text(f'ID=opensuse-leap\nVERSION_ID={upstream}\n')
                result = subprocess.run(['bash', '-c', '. "$1"; vega_obs_target "$2" "$3"',
                                         'test', str(HELPER), str(system), str(base)], capture_output=True, text=True)
                if expected:
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual(result.stdout.strip(), expected)
                else:
                    self.assertNotEqual(result.returncode, 0)

    def test_repository_setup_keeps_gpg_checks_and_rejects_wrong_existing_base(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            log = root / 'calls'
            fake = root / 'zypper'
            fake.write_text('''#!/bin/bash
printf '%s\\n' "$*" >> "$CALL_LOG"
if [ "$1" = "--xmlout" ]; then
  case "$REPO_STATE" in
    missing) exit 6 ;;
    correct) printf '<repo><url>https://download.opensuse.org/repositories/home:/rodrigosbrito:/lyra/openSUSE_Leap_16.1/</url></repo>' ;;
    wrong) printf '<repo><url>https://example.invalid/16.0/</url></repo>' ;;
  esac
fi
''')
            fake.chmod(0o755)
            for state in ('missing', 'correct', 'wrong'):
                log.write_text('')
                env = dict(os.environ, PATH=f'{root}:/usr/bin:/bin', CALL_LOG=str(log), REPO_STATE=state)
                result = subprocess.run(['bash', '-e', '-c', '. "$1"; vega_obs_configure lyra openSUSE_Leap_16.1',
                                         'test', str(HELPER)], env=env, capture_output=True, text=True)
                commands = log.read_text()
                if state == 'wrong':
                    self.assertNotEqual(result.returncode, 0)
                    self.assertNotIn('refresh', commands)
                    self.assertNotIn('addrepo', commands)
                else:
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertIn('--gpg-auto-import-keys refresh lyra-obs', commands)
                    self.assertNotIn('--no-gpg', commands)
                    self.assertIn('addrepo --refresh' if state == 'missing' else 'modifyrepo --enable --refresh --gpgcheck', commands)
