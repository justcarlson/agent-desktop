const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { copyFileSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } = require('node:fs');
const { tmpdir } = require('node:os');
const { join, resolve } = require('node:path');
const { test } = require('node:test');
const { PLATFORMS, tarballName } = require('../../npm/lib/platform');
const { releaseVersion } = require('../../scripts/release-version');
const { updateReleasePackages } = require('../../scripts/update-release-packages');

const files = ['Formula/agent-desktop.rb', 'bucket/agent-desktop.json', 'npm/package.json'];
function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'ad-release-packages-'));
  for (const name of ['Formula', 'bucket', 'npm', 'archives']) mkdirSync(join(root, name));
  for (const file of files) copyFileSync(resolve(__dirname, '../..', file), join(root, file));
  writeFileSync(join(root, 'Cargo.toml'), '[workspace.package]\nversion = "0.12.3"\n');
  const archives = join(root, 'archives');
  const expected = {};
  for (const [key, platform] of Object.entries(PLATFORMS)) {
    const name = tarballName('0.12.3', platform.target);
    const content = Buffer.from(`release artifact for ${key}`);
    const hash = createHash('sha256').update(content).digest('hex');
    writeFileSync(join(archives, name), content);
    expected[key] = { name, hash };
  }
  writeFileSync(join(archives, 'checksums.txt'), Object.values(expected).map(({ name, hash }) => `${hash}  ${name}`).join('\n'));
  return { root, archives, expected };
}
const contents = (root) => files.map((file) => readFileSync(join(root, file), 'utf8'));

test('one Cargo version updates all six archives and npm; regeneration is a no-op', () => {
  const { root, archives, expected } = fixture();
  const packagePath = join(root, 'npm/package.json');
  const metadata = JSON.parse(readFileSync(packagePath, 'utf8'));
  metadata.repository.url = 'git+https://github.com/release-owner/desktop-tool.git';
  writeFileSync(packagePath, JSON.stringify(metadata));
  updateReleasePackages(root, archives);
  const [formula, scoopText, npmText] = contents(root);
  const scoop = JSON.parse(scoopText);
  assert.equal(JSON.parse(npmText).version, '0.12.3');
  assert.equal(scoop.version, '0.12.3');
  assert.match(formula, /version "0\.12\.3"/);
  for (const [key, { name, hash }] of Object.entries(expected)) {
    const url = `https://github.com/release-owner/desktop-tool/releases/download/v0.12.3/${name}`;
    if (key.startsWith('win32-')) {
      assert.deepEqual(scoop.architecture[key.endsWith('x64') ? '64bit' : 'arm64'], { url, hash });
    } else {
      assert.ok(formula.includes(`url "${url}"\n      sha256 "${hash}"`));
    }
  }
  assert.equal(scoop.env_add_path, '.');
  assert.equal(scoop.bin, undefined);
  assert.equal(scoop.post_install, undefined);
  assert.match(formula, /bin\.install "agent-desktop-macos-helper"/);
  assert.match(formula, /bin\.install "agent-desktop-linux-clipboard-owner"/);
  const once = contents(root);
  updateReleasePackages(root, archives);
  assert.deepEqual(contents(root), once);
});

test('metadata generation accepts a Windows checkout with CRLF', () => {
  const { root, archives } = fixture();
  const formula = join(root, files[0]);
  writeFileSync(formula, readFileSync(formula, 'utf8').replace(/\r?\n/g, '\r\n'));
  updateReleasePackages(root, archives);
  assert.match(readFileSync(formula, 'utf8'), /version "0\.12\.3"/);
});

for (const problem of ['corrupt archive', 'missing checksum', 'changed formula', 'invalid version']) {
  test(`${problem} fails before any package file is updated`, () => {
    const { root, archives, expected } = fixture();
    if (problem === 'corrupt archive') writeFileSync(join(archives, expected['win32-x64'].name), 'tampered');
    if (problem === 'missing checksum') writeFileSync(join(archives, 'checksums.txt'), '');
    if (problem === 'changed formula') writeFileSync(join(root, files[0]), 'unrecognized formula');
    if (problem === 'invalid version') writeFileSync(join(root, 'Cargo.toml'), '[workspace.package]\nversion = "../../bad"\n');
    const before = contents(root);
    assert.throws(() => updateReleasePackages(root, archives));
    assert.deepEqual(contents(root), before);
  });
}

test('version comes only from the workspace package section', () => {
  const toml = '[package]\nversion = "9.9.9"\n[workspace.package]\nversion = "1.2.3"\n[dependencies]\nversion = "8.8.8"\n';
  assert.equal(releaseVersion(toml), '1.2.3');
  assert.equal(releaseVersion(toml.replaceAll('\n', '\r\n')), '1.2.3');
  for (const version of ['', '1.2', 'v1.2.3', '1.2.3;echo bad', '01.2.3']) {
    assert.throws(() => releaseVersion(`[workspace.package]\nversion = "${version}"\n`));
  }
  assert.throws(() => releaseVersion('[package]\nversion = "1.2.3"\n'));
});
