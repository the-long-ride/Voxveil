import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { readNamedArgs } from '../lib/cli.mjs';

const PLATFORMS = new Set(['windows', 'linux', 'macos', 'android', 'ios']);
const EDITIONS = new Set(['standard', 'pro-system']);

function tauriArgs(...args) {
  return ['run', 'tauri', '--workspace', '@voxveil/tauri-shell', '--', ...args];
}

export function commandsForVariant(platform, edition) {
  if (!PLATFORMS.has(platform)) throw new Error(`Unsupported platform: ${platform}`);
  if (!EDITIONS.has(edition)) throw new Error(`Unsupported edition: ${edition}`);
  if (platform === 'android') {
    return [
      tauriArgs('android', 'init'),
      tauriArgs('android', 'build', '--config', 'tauri.android.conf.json'),
    ];
  }
  if (platform === 'ios') {
    return [
      tauriArgs('ios', 'init'),
      tauriArgs('ios', 'build', '--config', 'tauri.ios.conf.json'),
    ];
  }
  return [tauriArgs('build', '--config', `tauri.${platform}.conf.json`)];
}

function run() {
  const { platform, edition } = readNamedArgs(process.argv.slice(2), ['platform', 'edition']);
  const env = { ...process.env, VOXVEIL_EDITION: edition };
  for (const args of commandsForVariant(platform, edition)) {
    const result = spawnSync('npm', args, { stdio: 'inherit', shell: process.platform === 'win32', env });
    if (result.status !== 0) process.exit(result.status ?? 1);
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) run();
