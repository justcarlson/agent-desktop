const assert = require('node:assert/strict');
const { execFileSync, spawnSync } = require('node:child_process');
const { copyFileSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } = require('node:fs');
const { tmpdir } = require('node:os');
const { join, resolve } = require('node:path');
const { test } = require('node:test');

const source = resolve(__dirname, '../..');
const files = ['Formula/agent-desktop.rb', 'bucket/agent-desktop.json', 'npm/package.json'];

test('package publication updates main, excludes unrelated files, and never overwrites concurrent changes', { skip: process.platform === 'win32' }, () => {
  const root = mkdtempSync(join(tmpdir(), 'ad-package-publish-'));
  const remote = join(root, 'origin.git');
  const tools = join(root, 'tools');
  const calls = join(root, 'gh.log');
  mkdirSync(tools);
  writeFileSync(calls, '');
  writeFileSync(join(tools, 'gh'), `#!/bin/sh
set -eu
printf '%s\\n' "$*" >> "$TEST_GH_LOG"
case "$1 $2" in
  'api users/'*) printf 'Release Actor\\n' ;;
  'auth setup-git') ;;
  *) exit 99 ;;
esac
`, { mode: 0o755 });
  const runGit = (cwd, ...args) => execFileSync('git', args, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
  runGit(root, 'init', '--bare', '-q', remote);
  const main = join(root, 'main');
  runGit(root, 'clone', '-q', remote, main);
  runGit(main, 'switch', '-c', 'main');
  for (const directory of ['scripts', 'Formula', 'bucket', 'npm']) mkdirSync(join(main, directory));
  for (const script of ['release-version.js', 'publish-release-packages.sh']) copyFileSync(join(source, 'scripts', script), join(main, 'scripts', script));
  writeFileSync(join(main, 'Cargo.toml'), '[workspace.package]\nversion = "1.2.3"\n');
  for (const file of files) writeFileSync(join(main, file), 'previous release\n');
  runGit(main, 'add', '.');
  runGit(main, '-c', 'user.name=Lahfir', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'test: release fixture');
  runGit(main, 'push', '-q', 'origin', 'main');
  const execute = (name, changed, beforePublish = () => {}) => {
    const checkout = join(root, name);
    runGit(root, 'clone', '-q', '--branch', 'main', remote, checkout);
    if (changed) for (const file of files) writeFileSync(join(checkout, file), `new release ${name}\n`);
    writeFileSync(join(checkout, 'unrelated.txt'), 'leave untouched\n');
    runGit(checkout, 'add', 'unrelated.txt');
    beforePublish();
    return spawnSync('bash', ['scripts/publish-release-packages.sh'], {
      cwd: checkout, encoding: 'utf8', env: { ...process.env,
        PATH: `${tools}:${process.env.PATH}`, GITHUB_ACTOR: 'release-actor', GITHUB_ACTOR_ID: '123',
        TEST_GH_LOG: calls },
    });
  };
  for (const name of ['first', 'repeat']) {
    const result = execute(name, true);
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(runGit(remote, 'diff-tree', '--no-commit-id', '--name-only', '-r', 'main').split('\n'), files);
    assert.equal(runGit(remote, 'show', '-s', '--format=%an <%ae>', 'main'), 'Release Actor <123+release-actor@users.noreply.github.com>');
  }
  const log = readFileSync(calls, 'utf8');
  const before = runGit(remote, 'rev-parse', 'main');
  assert.equal(execute('unchanged', false).status, 0);
  assert.equal(runGit(remote, 'rev-parse', 'main'), before);
  assert.equal(readFileSync(calls, 'utf8'), log);
  let concurrent;
  const raced = execute('raced', true, () => {
    runGit(main, 'pull', '-q', '--ff-only', 'origin', 'main');
    writeFileSync(join(main, 'concurrent.txt'), 'keep concurrent work\n');
    runGit(main, 'add', 'concurrent.txt');
    runGit(main, '-c', 'user.name=Lahfir', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'test: concurrent work');
    runGit(main, 'push', '-q', 'origin', 'main');
    concurrent = runGit(remote, 'rev-parse', 'main');
  });
  assert.notEqual(raced.status, 0);
  assert.match(raced.stderr, /rejected/);
  assert.equal(runGit(remote, 'rev-parse', 'main'), concurrent);
  writeFileSync(join(remote, 'hooks/pre-receive'), '#!/bin/sh\nexit 1\n', { mode: 0o755 });
  const rejected = execute('rejected', true);
  assert.notEqual(rejected.status, 0);
  assert.match(rejected.stderr, /pre-receive hook declined/);
  assert.equal(runGit(remote, 'rev-parse', 'main'), concurrent);
  assert.equal(runGit(remote, 'for-each-ref', '--format=%(refname)', 'refs/heads'), 'refs/heads/main');
});
