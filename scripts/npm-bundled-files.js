const { helperNames, PLATFORMS } = require('../npm/lib/platform');

module.exports = function bundledFiles() {
  return ['README.md', 'THIRD_PARTY_NOTICES.md', ...Object.entries(PLATFORMS).flatMap(([key, entry]) => {
    if (!entry.released) return [];
    const names = [entry.binaryName];
    names.push(...helperNames(entry).map((name) => `${name}-${key}`));
    return names.flatMap((name) => [`bin/${name}`, `bin/${name}.sha256`]);
  })];
};
