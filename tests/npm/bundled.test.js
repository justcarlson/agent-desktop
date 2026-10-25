const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const { createHash } = require('node:crypto');
const { cpSync, mkdirSync, mkdtempSync, realpathSync, readFileSync, writeFileSync } = require('node:fs');
const { tmpdir } = require('node:os');
const { join, resolve } = require('node:path');
const { test } = require('node:test');
const { PLATFORMS, helperNames, tarballName } = require('../../npm/lib/platform');

function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'agent-desktop-bundled-'));
  const source = resolve(__dirname, '../..');
  cpSync(join(source, 'npm'), join(root, 'npm'), { recursive: true });
  mkdirSync(join(root, 'scripts'));
  cpSync(join(source, 'scripts/prepare-npm-binaries.js'), join(root, 'scripts/prepare-npm-binaries.js'));
  const archives = join(root, 'archives');
  mkdirSync(archives);
  const { version } = require('../../npm/package.json');
  const hashes = [];
  for (const [key, entry] of Object.entries(PLATFORMS)) {
    if (!entry.released) continue;
    const stage = join(root, key);
    mkdirSync(stage);
    for (const name of entry.entries) {
      const contents = name === entry.entries[0]
        ? '#!/bin/sh\nprintf "%s" "$AGENT_DESKTOP_MACOS_HELPER_PATH"\n'
        : `#!/bin/sh\nprintf '%s' '${key}/${name}'\n`;
      writeFileSync(join(stage, name), contents, { mode: 0o755 });
    }
    const name = tarballName(version, entry.target);
    execFileSync('tar', ['-czf', join(archives, name), '-C', stage, ...entry.entries]);
    hashes.push(`${createHash('sha256').update(readFileSync(join(archives, name))).digest('hex')}  ${name}`);
  }
  writeFileSync(join(archives, 'checksums.txt'), hashes.join('\n'));
  return { root, archives };
}

test('release packaging runs with scripts skipped and selects the matching macOS helper', {
  skip: process.platform === 'win32',
}, () => {
  const { root, archives } = fixture();
  execFileSync(process.execPath, [join(root, 'scripts/prepare-npm-binaries.js'), archives]);
  const stub = join(root, 'os-stub.js');
  writeFileSync(stub, "const os = require('os'); os.platform = () => 'darwin'; os.arch = () => 'arm64';");
  const output = execFileSync(process.execPath,
    ['-r', stub, join(root, 'npm/bin/agent-desktop'), 'version'], { encoding: 'utf8' });
  assert.equal(output, realpathSync(join(root, 'npm/bin/agent-desktop-macos-helper-darwin-arm64')));
  for (const [key, entry] of Object.entries(PLATFORMS)) {
    if (!entry.released) continue;
    for (const name of helperNames(entry)) {
      assert.equal(readFileSync(join(root, 'npm/bin', `${name}-${key}`), 'utf8'),
        readFileSync(join(root, key, name), 'utf8'));
    }
  }
  const binary = join(root, 'npm/bin/agent-desktop-darwin-arm64');
  assert.equal(readFileSync(`${binary}.sha256`, 'utf8').trim(),
    createHash('sha256').update(readFileSync(binary)).digest('hex'));
});

test('release packaging rejects a modified archive before installing its executable', {
  skip: process.platform === 'win32',
}, () => {
  const { root, archives } = fixture();
  const { version } = require('../../npm/package.json');
  writeFileSync(join(archives, tarballName(version, PLATFORMS['darwin-arm64'].target)), 'corrupt');
  assert.throws(() => execFileSync(process.execPath,
    [join(root, 'scripts/prepare-npm-binaries.js'), archives], { stdio: 'pipe' }),
  (error) => error.stderr.toString().includes('Checksum mismatch'));
});
