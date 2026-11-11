#!/usr/bin/env python3
"""Run and bind the actual PER1 native persistence test before browser input."""
from __future__ import annotations

import argparse
import copy
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

sys.dont_write_bytecode = True
from hd1_reference import digest, source_identity, target_directory

BUILD_ARGV = ['cargo', 'test', '--locked', '-p', 'turts_client', '--test', 'persist',
              '--no-run', '--message-format=json']
TEST_ARGS = ['per1_native_paths', '--exact', '--nocapture', '--test-threads=1']
INPUTS = {'mod_id': 'forge', 'version': '0.2.0',
          'match_id_decimal': '18446744073709551613', 'slot': 1}
PATHS = {'bindings': 'turts/bindings-forge-0.2.json', 'settings': 'turts/settings.json',
         'groups': 'turts/groups-18446744073709551613-1.json'}
METHODS = {'reads': {'bindings': 'load_native_bindings', 'settings': 'SettingsStore::load',
                     'groups': 'load_native_groups'},
           'writes': {'bindings': 'save_native_bindings', 'settings': 'SettingsStore::set_volume',
                      'groups': 'save_native_groups'}}


def need(condition, message):
    if not condition:
        raise ValueError(message)


def write(path, value):
    data = value if isinstance(value, bytes) else (json.dumps(value, sort_keys=True, indent=2) + '\n').encode()
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, 'wb') as handle:
        handle.write(data)


def load(path):
    return json.loads(path.read_bytes())


def validated(root, output):
    before = load(output / 'source-before.json')
    need(before == load(output / 'source-after.json') == source_identity(root),
         'native persistence source changed or differs from browser source')
    run = load(output / 'launcher.json')
    target = target_directory(root)
    need(run['argv'][1:] == TEST_ARGS and run['build_argv'] == BUILD_ARGV and
         run['build_exit_code'] == 0 and run['cwd'] == str(root) and run['exit_code'] == 0,
         'native launcher did not complete the exact named test')
    need(run['target'] == str(target), 'native test used another cache')
    need(run['profile'] == {'CARGO_PROFILE_DEV_DEBUG': '0', 'CARGO_PROFILE_TEST_DEBUG': '0',
                            'CARGO_INCREMENTAL': '0'}, 'native profile differs')
    log = (output / 'native-process.log').read_bytes()
    need(digest(log) == run['log_sha256'] and b'test per1_native_paths ... ok' in log and
         b'1 passed; 0 failed; 0 ignored;' in log, 'named actual native test did not pass')
    need(digest((output / 'build.log').read_bytes()) == run['build_log_sha256'], 'native build log differs')
    raw_bytes = (output / 'native-raw.json').read_bytes()
    raw = json.loads(raw_bytes)
    need(raw['schema'] == 'per1-native-path-raw-v1' and raw['receipt_path'] == run['raw_receipt_path'],
         'raw native receipt identity differs')
    raw_path = Path(raw['receipt_path'])
    need(raw_path.is_absolute() and not raw_path.is_relative_to(root), 'raw receipt was in the served tree')
    process = raw['process']
    pid = process['pid']
    need(type(pid) is int and pid > 0 and pid == run['native_pid'] and
         process['argv'] == run['argv'],
         'native test process identity differs')
    executable = Path(process['executable']).resolve(strict=True)
    need(executable.is_relative_to(target / 'debug/deps') and executable.name.startswith('persist-') and
         digest(executable.read_bytes()) == process['executable_sha256'] == run['executable_sha256'], 'native binary differs')
    need(run['started_unix_ms'] <= process['started_unix_ms'] <= process['ended_unix_ms'] <= run['ended_unix_ms'],
         'native process clocks are outside actual launcher interval')
    config = Path(raw['config_root'])
    need(config.is_absolute() and config.name.startswith(f'turts-task16-persist-{pid}-'),
         'native root differs from actual isolated config root')
    need(raw['inputs'] == INPUTS and raw['paths'] == PATHS, 'native input or design path differs')
    for kind, operation in [('reads', 'read'), ('writes', 'write')]:
        rows = raw[kind]
        need(len(rows) == 3 and {r['path'] for r in rows} == {str(config / p) for p in PATHS.values()},
             'native file operations are missing or duplicated')
        for name, relative in PATHS.items():
            row = next(r for r in rows if r['path'] == str(config / relative))
            need(row['method'] == METHODS[kind][name] and row['operation'] == operation and
                 row['pid'] == pid and row['success'] is True and type(row['byte_length']) is int and
                 row['byte_length'] > 0 and process['started_unix_ms'] <= row['observed_unix_ms'] <= process['ended_unix_ms'],
                 'native operation provenance differs')
            sha = row['sha256']
            need(isinstance(sha, str) and len(sha) == 64 and all(c in '0123456789abcdef' for c in sha) and
                 sha == row['readback_sha256'], 'native readback bytes differ')
    for entry in raw['writes']:
        read = next(r for r in raw['reads'] if r['path'] == entry['path'])
        need(entry['observed_unix_ms'] <= read['observed_unix_ms'] and
             entry['sha256'] == read['sha256'] and entry['byte_length'] == read['byte_length'],
             'actual persisted write/reload bytes differ')
    audit = copy.deepcopy(raw)
    audit['schema'] = 'per1-native-path-audit-v2'
    audit['process'].update(exit_code=run['exit_code'],
                            source_sha256=digest((output / 'source-before.json').read_bytes()),
                            runner_receipt_sha256=digest((output / 'launcher.json').read_bytes()))
    audit['provenance'] = {'raw_receipt_sha256': digest(raw_bytes),
                            'operation_rows': 'emitted by actual test process; preserved without synthesis'}
    return audit


def create(root, output):
    output.mkdir(mode=0o700)
    target = target_directory(root)
    profile = {'CARGO_PROFILE_DEV_DEBUG': '0', 'CARGO_PROFILE_TEST_DEBUG': '0', 'CARGO_INCREMENTAL': '0'}
    need(all(os.environ.get(k) == v for k, v in profile.items()), 'bounded native profiles must be explicit')
    write(output / 'source-before.json', source_identity(root))
    # The Rust producer requires its create-only raw file outside the served tree.
    with tempfile.TemporaryDirectory(prefix='mc-per1-native-') as temporary:
        raw_path = Path(temporary) / 'raw.json'
        env = dict(os.environ, MC_PER1_NATIVE_RAW_RECEIPT=str(raw_path))
        built = subprocess.run(BUILD_ARGV, cwd=root, env=env, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, check=False)
        build_log = built.stdout + built.stderr
        write(output / 'build.log', build_log)
        need(built.returncode == 0, 'native compile failed; retain build.log')
        artifacts = [json.loads(line) for line in built.stdout.splitlines() if line.startswith(b'{')]
        executables = [entry['executable'] for entry in artifacts if entry.get('reason') == 'compiler-artifact'
                       and entry.get('target', {}).get('name') == 'persist'
                       and entry.get('profile', {}).get('test') is True and entry.get('executable')]
        need(len(executables) == 1, 'Cargo must discover exactly one persist test executable')
        executable = Path(executables[0]).resolve(strict=True)
        need(executable.is_relative_to(target / 'debug/deps'), 'discovered test is outside assigned cache')
        executable_sha = digest(executable.read_bytes())
        argv = [str(executable), *TEST_ARGS]
        started = time.time_ns() // 1_000_000
        with subprocess.Popen(argv, cwd=root, env=env, stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT) as process:
            native_pid = process.pid
            log, _ = process.communicate()
            exit_code = process.returncode
        ended = time.time_ns() // 1_000_000
        write(output / 'native-process.log', log)
        write(output / 'launcher.json', {'argv': argv, 'build_argv': BUILD_ARGV,
              'build_exit_code': built.returncode, 'build_log_sha256': digest(build_log),
              'native_pid': native_pid, 'executable_sha256': executable_sha,
              'cwd': str(root), 'target': str(target),
              'profile': profile, 'exit_code': exit_code, 'started_unix_ms': started,
              'ended_unix_ms': ended, 'raw_receipt_path': str(raw_path),
              'log_sha256': digest(log)})
        write(output / 'source-after.json', source_identity(root))
        need(exit_code == 0, 'native process failed; retain raw native-process.log')
        need(raw_path.stat().st_mode & 0o077 == 0 and raw_path.parent.stat().st_mode & 0o077 == 0,
             'native raw output was not private')
        write(output / 'native-raw.json', raw_path.read_bytes())
        write(output / 'native-path-audit.json', validated(root, output))
    validate(root, output)


def validate(root, output):
    need(load(output / 'native-path-audit.json') == validated(root, output),
         'bound native audit differs from actual raw process/launcher evidence')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('command', choices=['create', 'validate'])
    parser.add_argument('root', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    try:
        globals()[args.command](args.root.resolve(strict=True), args.output.resolve())
    except (ValueError, OSError, KeyError, StopIteration) as error:
        raise SystemExit(f'PER1 native evidence invalid: {error}')
