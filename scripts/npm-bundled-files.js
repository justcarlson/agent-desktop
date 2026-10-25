const { MACOS_HELPER_NAME, PLATFORMS } = require('../npm/lib/platform');

module.exports = function bundledFiles() {
  return ['README.md', ...Object.entries(PLATFORMS).flatMap(([key, entry]) => {
    if (!entry.released) return [];
    const names = [entry.binaryName];
    if (entry.entries.includes(MACOS_HELPER_NAME)) names.push(`${MACOS_HELPER_NAME}-${key}`);
    return names.flatMap((name) => [`bin/${name}`, `bin/${name}.sha256`]);
  })];
};
