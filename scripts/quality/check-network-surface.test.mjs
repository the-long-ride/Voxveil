import assert from 'node:assert/strict';
import { test } from 'node:test';
import { inspectPackageJson, inspectProductionText } from './check-network-surface.mjs';

test('rejects generic tauri http plugin dependency', () => {
  const issues = inspectPackageJson({ dependencies: { '@tauri-apps/plugin-http': '2.0.0' } }, 'ui/package.json');
  assert.equal(issues.length, 1);
});

test('rejects remote dependency specifiers', () => {
  const issues = inspectPackageJson({ dependencies: { bad: 'git+https://example.invalid/x.git' } }, 'package.json');
  assert.equal(issues.length, 1);
});

test('rejects fetch in production ui source', () => {
  assert.equal(inspectProductionText('fetch("https://example.invalid")', 'ui/app/a.ts').length, 1);
});

test('rejects browser telemetry and socket primitives in production ui source', () => {
  const source = 'navigator.sendBeacon("/t", "x"); new WebSocket("wss://example.invalid");';
  const issues = inspectProductionText(source, 'ui/app/a.ts');
  assert.equal(issues.length, 2);
  assert.match(issues.join('\n'), /sendBeacon/);
  assert.match(issues.join('\n'), /WebSocket/);
});

test('rejects XMLHttpRequest and EventSource in production ui source', () => {
  const source = 'const a = new XMLHttpRequest(); const b = new EventSource("/events");';
  const issues = inspectProductionText(source, 'ui/app/a.ts');
  assert.equal(issues.length, 2);
});

test('rejects Rust socket primitives in production core code', () => {
  const source = 'use std::net::TcpStream; let socket = std::net::UdpSocket::bind("127.0.0.1:0");';
  const issues = inspectProductionText(source, 'tauri/security/network.rs');
  assert.equal(issues.length, 2);
  assert.match(issues.join('\n'), /TcpStream/);
  assert.match(issues.join('\n'), /UdpSocket/);
});


test('allows the dedicated model HTTP client only in the approved downloader', () => {
  assert.equal(inspectProductionText('minreq::get(url);', 'tauri/models/download.rs').length, 0);
  const issues = inspectProductionText('minreq::get(url);', 'tauri/security/network.rs');
  assert.equal(issues.length, 1);
  assert.match(issues[0], /HTTP client outside approved model downloader/);
});
