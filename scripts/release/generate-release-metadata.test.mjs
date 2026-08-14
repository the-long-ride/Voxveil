import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  cargoMetadataToCycloneDx,
  packageLockToCycloneDx,
} from './generate-release-metadata.mjs';

test('packageLockToCycloneDx emits registry packages and skips workspace roots', () => {
  const lock = {
    name: 'voxveil',
    version: '0.1.0',
    lockfileVersion: 3,
    packages: {
      '': { name: 'voxveil', version: '0.1.0' },
      'node_modules/react': {
        version: '19.2.8',
        resolved: 'https://registry.npmjs.org/react/-/react-19.2.8.tgz',
        integrity: 'sha512-test',
        license: 'MIT',
      },
      'ui': { name: '@voxveil/ui', version: '0.1.0' },
    },
  };

  const sbom = packageLockToCycloneDx(lock, '2026-08-14T00:00:00.000Z');

  assert.equal(sbom.bomFormat, 'CycloneDX');
  assert.equal(sbom.components.length, 1);
  assert.equal(sbom.components[0].name, 'react');
  assert.equal(sbom.components[0].version, '19.2.8');
  assert.equal(sbom.components[0].purl, 'pkg:npm/react@19.2.8');
  assert.deepEqual(sbom.components[0].licenses, [{ license: { id: 'MIT' } }]);
});

test('cargoMetadataToCycloneDx includes crates.io packages and license expressions', () => {
  const metadata = {
    packages: [
      {
        name: 'voxveil-dsp',
        version: '0.1.0',
        source: null,
        license: 'MIT OR Apache-2.0',
      },
      {
        name: 'serde',
        version: '1.0.229',
        source: 'registry+https://github.com/rust-lang/crates.io-index',
        license: 'MIT OR Apache-2.0',
      },
    ],
  };

  const sbom = cargoMetadataToCycloneDx(metadata, '2026-08-14T00:00:00.000Z');

  assert.equal(sbom.components.length, 1);
  assert.equal(sbom.components[0].name, 'serde');
  assert.equal(sbom.components[0].purl, 'pkg:cargo/serde@1.0.229');
  assert.deepEqual(sbom.components[0].licenses, [{ expression: 'MIT OR Apache-2.0' }]);
});

test('packageLockToCycloneDx encodes scoped npm package purls', () => {
  const sbom = packageLockToCycloneDx({
    packages: {
      'node_modules/@tauri-apps/api': { version: '2.11.1', license: 'MIT OR Apache-2.0' },
    },
  }, '2026-08-14T00:00:00.000Z');
  assert.equal(sbom.components[0].purl, 'pkg:npm/%40tauri-apps/api@2.11.1');
});
