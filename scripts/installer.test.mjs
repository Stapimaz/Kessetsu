import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, chmodSync, readFileSync, readlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawnSync, execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';

test('POSIX installer contracts: platforms, install/update, corruption, manifest integrity, and collisions', { skip: process.platform === 'win32' }, () => {
  const fixture = mkdtempSync(join(tmpdir(), 'kessetsu-installer-contracts-'));
  const tools = join(fixture, 'tools');
  mkdirSync(tools);
  function executable(name, source) { const path = join(tools, name); writeFileSync(path, source); chmodSync(path, 0o755); }
  executable('uname', '#!/bin/sh\ncase "$1" in -s) printf "%s\\n" "$KESSETSU_TEST_OS";; -m) printf "%s\\n" "$KESSETSU_TEST_ARCH";; esac\n');
  executable('curl', `#!/bin/sh
destination=''; effective=false
while [ "$#" -gt 0 ]; do
  case "$1" in -o) destination=$2; shift 2;; -w) effective=true; shift 2;; *) shift;; esac
done
if [ "$effective" = true ]; then printf 'https://github.com/Stapimaz/Kessetsu/releases/tag/v1.0.0'; exit 0; fi
case "$destination" in *.sha256) cp "$KESSETSU_FIXTURE_ARCHIVE.sha256" "$destination";; *) cp "$KESSETSU_FIXTURE_ARCHIVE" "$destination";; esac
`);
  const script = resolve(import.meta.dirname, '../webapp/public/install.sh');
  function invoke(root, system = 'Linux', arch = 'x86_64', archive = '') {
    return spawnSync('sh', [script, '--install-dir', root, '--no-path'], {
      encoding: 'utf8', env: { ...process.env, PATH: `${tools}:${process.env.PATH}`, KESSETSU_TEST_OS: system, KESSETSU_TEST_ARCH: arch, KESSETSU_FIXTURE_ARCHIVE: archive },
    });
  }
  function bundle(target, wrongBinaryHash = false) {
    const source = mkdtempSync(join(fixture, 'bundle-'));
    const name = `kessetsu-v1.0.0-${target}`;
    const stage = join(source, name);
    mkdirSync(stage);
    const binary = '#!/bin/sh\nprintf "kess 1.0.0\\n"\n';
    writeFileSync(join(stage, 'kess'), binary); chmodSync(join(stage, 'kess'), 0o755);
    writeFileSync(join(stage, 'release-manifest.json'), JSON.stringify({ schema_version: 'kessetsu.release.v1', version: '1.0.0', target, executable: 'kess', executable_sha256: wrongBinaryHash ? '0'.repeat(64) : createHash('sha256').update(binary).digest('hex') }, null, 2));
    const archive = join(source, `${name}.tar.gz`);
    execFileSync('tar', ['-czf', archive, '-C', source, name]);
    const checksum = `${createHash('sha256').update(readFileSync(archive)).digest('hex')}  ${name}.tar.gz\n`;
    writeFileSync(`${archive}.sha256`, checksum);
    return archive;
  }
  for (const [target, system, arch] of [['linux-x86_64', 'Linux', 'x86_64'], ['macos-x86_64', 'Darwin', 'x86_64'], ['macos-aarch64', 'Darwin', 'arm64']]) {
    const root = join(fixture, `managed-${target}`);
    const archive = bundle(target);
    let result = invoke(root, system, arch, archive);
    assert.equal(result.status, 0, result.stderr);
    const initial = readlinkSync(join(root, 'bin/kess'));
    result = invoke(root, system, arch, archive);
    assert.equal(result.status, 0, result.stderr);
    const updated = readlinkSync(join(root, 'bin/kess'));
    assert.notEqual(initial, updated);
    writeFileSync(`${archive}.sha256`, `${'0'.repeat(64)}  kessetsu-v1.0.0-${target}.tar.gz\n`);
    result = invoke(root, system, arch, archive);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /Archive checksum mismatch/);
    assert.equal(readlinkSync(join(root, 'bin/kess')), updated);
    result = invoke(root, system, arch, bundle(target, true));
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /Executable checksum mismatch/);
    assert.equal(readlinkSync(join(root, 'bin/kess')), updated);
  }
  let result = invoke(join(fixture, 'unsupported'), 'Linux', 'aarch64');
  assert.notEqual(result.status, 0); assert.match(result.stderr, /Unsupported platform/);
  const unmanaged = join(fixture, 'unmanaged'); mkdirSync(unmanaged); writeFileSync(join(unmanaged, 'keep'), 'keep');
  result = invoke(unmanaged); assert.notEqual(result.status, 0); assert.match(result.stderr, /Unmanaged directory/);
  assert.equal(readFileSync(join(unmanaged, 'keep'), 'utf8'), 'keep');
  const collision = join(fixture, 'collision'); mkdirSync(collision); writeFileSync(join(collision, '.kessetsu-install'), 'kessetsu.install.v1\n');
  mkdirSync(join(collision, 'bin')); writeFileSync(join(collision, 'bin/kess'), 'unrelated');
  result = invoke(collision, 'Linux', 'x86_64', bundle('linux-x86_64'));
  assert.notEqual(result.status, 0); assert.match(result.stderr, /not managed/);
  assert.equal(readFileSync(join(collision, 'bin/kess'), 'utf8'), 'unrelated');
});
