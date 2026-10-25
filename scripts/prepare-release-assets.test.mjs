import { afterEach, describe, expect, it } from 'vitest';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import { resolve } from 'node:path';
import { prepareReleaseAssets } from './prepare-release-assets.mjs';

const roots = [];
function fixture() {
  // Tests stay on the project's drive, never a user's app-data or keychain.
  const base = resolve('.codex', 'release-tests');
  fs.mkdirSync(base, { recursive: true });
  const root = fs.mkdtempSync(resolve(base, 'case-'));
  roots.push(root);
  fs.writeFileSync(resolve(root, 'package.json'), JSON.stringify({ version: '0.2.0' }));
  for (const [platform, names] of [
    ['windows-x64', ['BytePet_0.2.0_x64-setup.exe', 'BytePet_0.2.0_x64.msi']],
    ...['arm64', 'x86_64'].map(arch => [`macos-${arch}`, [`BytePet_0.2.0_${arch}.dmg`, `BytePet_0.2.0_${arch}.app.zip`]]),
  ]) {
    const dir = resolve(root, 'artifacts', platform);
    fs.mkdirSync(dir, { recursive: true });
    const lines = names.map(name => {
      fs.writeFileSync(resolve(dir, name), name);
      const hash = createHash('sha256').update(name).digest('hex');
      return `${hash}  ./${name}`;
    });
    fs.writeFileSync(resolve(dir, 'SHA256SUMS.txt'), lines.join('\n'));
  }
  return root;
}
afterEach(() => {
  for (const root of roots.splice(0)) {
    // Check the resolved boundary before recursive deletion on Windows.
    const base = resolve('.codex', 'release-tests');
    if (!root.startsWith(`${base}/`) && !root.startsWith(`${base}\\`)) throw Error('Unexpected fixture path');
    fs.rmSync(root, { recursive: true });
  }
});
describe('release packages', () => {
  it('requires all platforms and produces six packages plus one combined checksum file', () => {
    const root = fixture();
    const result = prepareReleaseAssets(root);
    expect(result.packages).toBe(6);
    expect(fs.readdirSync(result.output)).toHaveLength(7);
    expect(fs.readFileSync(resolve(result.output, 'SHA256SUMS.txt'), 'utf8').trim().split('\n')).toHaveLength(6);
    expect(() => prepareReleaseAssets(root)).toThrow('previous output');
  });
  it('refuses a modified package before creating any release output', () => {
    const root = fixture();
    fs.appendFileSync(resolve(root, 'artifacts/windows-x64/BytePet_0.2.0_x64.msi'), 'changed');
    expect(() => prepareReleaseAssets(root)).toThrow('Checksum mismatch');
    expect(fs.existsSync(resolve(root, 'artifacts/release'))).toBe(false);
  });
  it('refuses duplicate or missing checksums', () => {
    const root = fixture();
    const checksums = resolve(root, 'artifacts/macos-arm64/SHA256SUMS.txt');
    const line = fs.readFileSync(checksums, 'utf8').split('\n')[0];
    fs.writeFileSync(checksums, `${line}\n${line}`);
    expect(() => prepareReleaseAssets(root)).toThrow('Duplicate checksum');
  });
  it('refuses path traversal in checksum entries', () => {
    const root = fixture();
    const checksums = resolve(root, 'artifacts/windows-x64/SHA256SUMS.txt');
    fs.writeFileSync(checksums, fs.readFileSync(checksums, 'utf8').replace('./BytePet_', '../BytePet_'));
    expect(() => prepareReleaseAssets(root)).toThrow('Malformed checksum');
  });
});
