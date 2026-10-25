import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

export function prepareReleaseAssets(root = process.cwd()) {
  const version = JSON.parse(fs.readFileSync(resolve(root, 'package.json'), 'utf8')).version;
  assert.match(version, /^\d+\.\d+\.\d+$/, 'Release requires a stable numeric version');
  const groups = [
    ['windows-x64', [`BytePet_${version}_x64-setup.exe`, `BytePet_${version}_x64.msi`]],
    ...['x86_64', 'arm64'].map(arch => [`macos-${arch}`, [`BytePet_${version}_${arch}.dmg`, `BytePet_${version}_${arch}.app.zip`]]),
  ];
  const verified = [];
  for (const [platform, names] of groups) {
    const folder = resolve(root, 'artifacts', platform);
    const lines = fs.readFileSync(resolve(folder, 'SHA256SUMS.txt'), 'utf8').trim().split(/\r?\n/);
    assert.equal(lines.length, names.length, `Unexpected checksum count for ${platform}`);
    const checksums = new Map(lines.map(line => {
      const match = line.match(/^([a-fA-F0-9]{64})\s+\*?(?:\.\/)?([^/\\]+)$/);
      assert.ok(match, `Malformed checksum entry for ${platform}`);
      return [match[2], match[1].toLowerCase()];
    }));
    assert.equal(checksums.size, names.length, `Duplicate checksum entry for ${platform}`);
    for (const name of names) {
      const path = resolve(folder, name);
      const stat = fs.lstatSync(path);
      assert.ok(stat.isFile() && !stat.isSymbolicLink() && stat.size > 0, `Invalid asset: ${name}`);
      const hash = createHash('sha256').update(fs.readFileSync(path)).digest('hex');
      assert.equal(hash, checksums.get(name), `Checksum mismatch: ${name}`);
      verified.push({ name, path, hash });
    }
  }
  // Verify the backups as well, but expose only one recommended installer per platform.
  const published = verified.filter(({ name }) => name.endsWith('-setup.exe') || name.endsWith('.dmg'));
  assert.equal(published.length, 3, 'Expected one primary installer per platform');
  const output = resolve(root, 'artifacts', 'release');
  assert.ok(!fs.existsSync(output), 'Refusing to mix release assets with a previous output');
  fs.mkdirSync(output);
  for (const asset of published) fs.copyFileSync(asset.path, resolve(output, asset.name));
  fs.writeFileSync(resolve(output, 'SHA256SUMS.txt'), published.map(({ hash, name }) => `${hash}  ${name}\n`).join(''));
  return { version, verifiedPackages: verified.length, packages: published.length, output };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  console.log(JSON.stringify(prepareReleaseAssets()));
}
