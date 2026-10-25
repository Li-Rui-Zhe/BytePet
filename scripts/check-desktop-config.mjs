import assert from 'node:assert/strict';
import fs from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

export function checkDesktopConfig(root = process.cwd()) {
  const read = file => fs.readFileSync(resolve(root, file), 'utf8');
  const packageJson = JSON.parse(read('package.json'));
  const base = JSON.parse(read('src-tauri/tauri.conf.json'));
  const mac = JSON.parse(read('src-tauri/tauri.macos.conf.json'));
  const windows = JSON.parse(read('src-tauri/tauri.windows.conf.json'));
  const cargo = read('src-tauri/Cargo.toml');
  assert.equal(base.version, packageJson.version, 'package.json and Tauri version differ');
  assert.equal(cargo.match(/^version\s*=\s*"([^"]+)"/m)?.[1], base.version, 'Cargo package version differs');
  assert.equal(mac.bundle.macOS.minimumSystemVersion, '13.3');
  assert.equal(mac.bundle.macOS.signingIdentity, '-', 'Default CI builds must remain explicitly ad-hoc signed');
  assert.deepEqual(mac.bundle.targets, ['app', 'dmg']);
  assert.equal(mac.app.windows.length, 1);
  const window = mac.app.windows[0];
  assert.equal(window.label, 'main');
  assert.equal(window.transparent, true);
  assert.equal(window.decorations, false);
  assert.equal(window.visibleOnAllWorkspaces, true);
  assert.equal(window.alwaysOnTop, true);
  const plist = read('src-tauri/Info.macos.plist');
  assert.match(plist, /<key>LSUIElement<\/key>\s*<true\s*\/>/);
  assert.match(plist, /<key>NSHighResolutionCapable<\/key>\s*<true\s*\/>/);
  const icons = mac.bundle.icon;
  assert.ok(icons.includes('icons/icon.icns'));
  for (const icon of icons) assert.ok(fs.existsSync(resolve(root, 'src-tauri', icon)), `Missing ${icon}`);
  const icns = fs.readFileSync(resolve(root, 'src-tauri/icons/icon.icns'));
  assert.equal(icns.toString('ascii', 0, 4), 'icns');
  assert.equal(icns.readUInt32BE(4), icns.length, 'Invalid ICNS container length');
  assert.deepEqual(windows.bundle.targets, ['nsis', 'msi']);
  assert.equal(windows.bundle.useLocalToolsDir, true);
  assert.equal(windows.bundle.windows.webviewInstallMode.type, 'downloadBootstrapper');
  assert.equal(windows.bundle.windows.nsis.installMode, 'currentUser');
  for (const icon of base.bundle.icon) assert.ok(fs.existsSync(resolve(root, 'src-tauri', icon)), `Missing ${icon}`);
  return { version: base.version, minimumMacOS: mac.bundle.macOS.minimumSystemVersion, icons: icons.length, windowsBundles: windows.bundle.targets };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  console.log(JSON.stringify(checkDesktopConfig()));
}
