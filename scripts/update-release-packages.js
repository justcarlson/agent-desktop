const { createHash } = require('node:crypto');
const { readFileSync, writeFileSync } = require('node:fs');
const { join } = require('node:path');
const { PLATFORMS, tarballName } = require('../npm/lib/platform');
const { checksumFor } = require('../npm/scripts/postinstall');
const { releaseVersion } = require('./release-version');

function updateReleasePackages(root, archives) {
  const version = releaseVersion(readFileSync(join(root, 'Cargo.toml'), 'utf8'));
  const checksums = readFileSync(join(archives, 'checksums.txt'), 'utf8');
  const packages = {};
  for (const [key, platform] of Object.entries(PLATFORMS)) {
    if (!platform.released) continue;
    const name = tarballName(version, platform.target);
    const hash = createHash('sha256').update(readFileSync(join(archives, name))).digest('hex');
    if (hash !== checksumFor(checksums, name)) throw new Error(`Checksum mismatch for ${name}`);
    packages[key] = { url: `https://github.com/lahfir/agent-desktop/releases/download/v${version}/${name}`, hash };
  }

  const formulaPath = join(root, 'Formula/agent-desktop.rb');
  let formula = readFileSync(formulaPath, 'utf8').replace(/\r\n/g, '\n');
  if ((formula.match(/^  version "[^"\n]+"$/gm) || []).length !== 1) {
    throw new Error('Expected one Homebrew version');
  }
  formula = formula.replace(/^  version "[^"\n]+"$/m, `  version "${version}"`);
  for (const [key, platform] of Object.entries(PLATFORMS)) {
    if (!platform.released || key.startsWith('win32-')) continue;
    const pattern = new RegExp(`url "[^"\\n]+-${platform.target}\\.tar\\.gz"\\r?\\n( +)sha256 "[0-9a-f]{64}"`, 'g');
    if ([...formula.matchAll(pattern)].length !== 1) throw new Error(`Expected one Homebrew archive for ${key}`);
    const { url, hash } = packages[key];
    formula = formula.replace(pattern, (_, indent) => `url "${url}"\n${indent}sha256 "${hash}"`);
  }

  const scoopPath = join(root, 'bucket/agent-desktop.json');
  const scoop = JSON.parse(readFileSync(scoopPath, 'utf8'));
  scoop.version = version;
  scoop.architecture = { '64bit': packages['win32-x64'], arm64: packages['win32-arm64'] };
  const npmPath = join(root, 'npm/package.json');
  const npm = JSON.parse(readFileSync(npmPath, 'utf8'));
  npm.version = version;

  writeFileSync(formulaPath, formula);
  writeFileSync(scoopPath, `${JSON.stringify(scoop, null, 4)}\n`);
  writeFileSync(npmPath, `${JSON.stringify(npm, null, 2)}\n`);
}

if (require.main === module) {
  if (!process.argv[2]) throw new Error('Usage: node scripts/update-release-packages.js ARCHIVE_DIRECTORY');
  updateReleasePackages(process.cwd(), process.argv[2]);
}
module.exports = { updateReleasePackages };
