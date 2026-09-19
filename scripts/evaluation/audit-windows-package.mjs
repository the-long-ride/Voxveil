import { createHash } from 'node:crypto';
import { readdir, readFile } from 'node:fs/promises';
import path from 'node:path';

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function validHash(value) {
  return typeof value === 'string' && /^[a-f0-9]{64}$/i.test(value);
}

async function listFiles(root, relative = '') {
  const directory = path.join(root, relative);
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const child = relative ? path.posix.join(relative.replaceAll('\\', '/'), entry.name) : entry.name;
    if (entry.isDirectory()) files.push(...await listFiles(root, child));
    else if (entry.isFile()) files.push(child.replaceAll('\\', '/'));
  }
  return files.sort();
}

function parseChecksums(text) {
  const entries = new Map();
  const issues = [];
  for (const raw of text.split(/\r?\n/)) {
    if (!raw.trim()) continue;
    const match = /^([a-f0-9]{64})  ([^\\].*)$/i.exec(raw);
    if (!match) {
      issues.push(`invalid SHA256SUMS line: ${raw}`);
      continue;
    }
    const relative = match[2].replaceAll('\\', '/');
    if (path.posix.isAbsolute(relative) || relative === '..' || relative.startsWith('../') || relative.includes('/../')) {
      issues.push(`checksum path escapes package root: ${relative}`);
      continue;
    }
    if (relative === 'SHA256SUMS.txt') {
      issues.push('SHA256SUMS.txt must not checksum itself');
      continue;
    }
    if (entries.has(relative)) {
      issues.push(`duplicate checksum entry: ${relative}`);
      continue;
    }
    entries.set(relative, match[1].toLowerCase());
  }
  return { entries, issues };
}

async function readJson(file, label, issues) {
  try {
    return JSON.parse(await readFile(file, 'utf8'));
  } catch (error) {
    issues.push(`${label}: missing or invalid JSON (${error.code === 'ENOENT' ? 'not found' : error.message})`);
    return null;
  }
}

export async function auditWindowsPackage(args) {
  const root = path.resolve(args.packageRoot);
  const issues = [];
  const manifestPath = path.join(root, 'release-manifest.json');
  const sumsPath = path.join(root, 'SHA256SUMS.txt');
  const manifest = await readJson(manifestPath, 'release-manifest.json', issues);

  if (manifest) {
    if (manifest.schemaVersion !== 1) issues.push('release-manifest.json: schemaVersion must be 1');
    if (manifest.voxveilCommit !== args.commit) issues.push('release-manifest.json: exact commit mismatch');
    if (manifest.architecture !== args.architecture) issues.push('release-manifest.json: architecture mismatch');
    if (manifest.packageFilesHashedBy !== 'SHA256SUMS.txt') issues.push('release-manifest.json: packageFilesHashedBy must be SHA256SUMS.txt');
  }

  let checksumEntries = new Map();
  try {
    const parsed = parseChecksums(await readFile(sumsPath, 'ascii'));
    checksumEntries = parsed.entries;
    issues.push(...parsed.issues);
  } catch (error) {
    issues.push(`SHA256SUMS.txt: missing or unreadable (${error.code === 'ENOENT' ? 'not found' : error.message})`);
  }

  let packageFiles = [];
  try {
    packageFiles = (await listFiles(root)).filter((file) => file !== 'SHA256SUMS.txt');
  } catch (error) {
    issues.push(`package root: unreadable (${error.code === 'ENOENT' ? 'not found' : error.message})`);
  }

  const fileSet = new Set(packageFiles);
  for (const file of packageFiles) {
    const expected = checksumEntries.get(file);
    if (!expected) {
      issues.push(`SHA256SUMS.txt: missing entry for ${file}`);
      continue;
    }
    const actual = sha256(await readFile(path.join(root, ...file.split('/'))));
    if (actual !== expected) issues.push(`SHA256SUMS.txt: hash mismatch for ${file}`);
  }
  for (const file of checksumEntries.keys()) {
    if (!fileSet.has(file)) issues.push(`SHA256SUMS.txt: entry points to missing file ${file}`);
  }

  if (manifest) {
    const apoPath = path.join(root, 'system-audio', 'apo-verification.json');
    const driverPath = path.join(root, 'system-audio', 'virtual-driver', 'verification.json');

    if (manifest.signedApo?.present === true) {
      if (!validHash(manifest.signedApo.verificationSha256)) {
        issues.push('release-manifest.json: signed APO verification SHA-256 is invalid');
      } else {
        try {
          const bytes = await readFile(apoPath);
          if (sha256(bytes) !== manifest.signedApo.verificationSha256) {
            issues.push('release-manifest.json: signed APO verification hash mismatch');
          }
          const apo = JSON.parse(bytes.toString('utf8'));
          if (apo.voxveilCommit !== args.commit) issues.push('apo-verification.json: exact commit mismatch');
        } catch (error) {
          issues.push(`apo-verification.json: missing or invalid (${error.code === 'ENOENT' ? 'not found' : error.message})`);
        }
      }
    } else if (fileSet.has('system-audio/apo-verification.json')) {
      issues.push('release-manifest.json: signedApo.present is false but apo-verification.json is packaged');
    }

    if (manifest.signedVirtualDriver?.present !== true) {
      issues.push('release-manifest.json: signed virtual driver must be present for the unified driver release gate');
    } else {
      if (manifest.signedVirtualDriver.releaseChannel !== args.releaseChannel) {
        issues.push('release-manifest.json: signed virtual-driver release channel mismatch');
      }
      if (!validHash(manifest.signedVirtualDriver.verificationSha256)) {
        issues.push('release-manifest.json: signed virtual-driver verification SHA-256 is invalid');
      } else {
        try {
          const bytes = await readFile(driverPath);
          if (sha256(bytes) !== manifest.signedVirtualDriver.verificationSha256) {
            issues.push('release-manifest.json: signed virtual-driver verification hash mismatch');
          }
          const driver = JSON.parse(bytes.toString('utf8'));
          if (driver.voxveilCommit !== args.commit) issues.push('virtual-driver/verification.json: exact commit mismatch');
          if (driver.releaseChannel !== args.releaseChannel) issues.push('virtual-driver/verification.json: release channel mismatch');
        } catch (error) {
          issues.push(`virtual-driver/verification.json: missing or invalid (${error.code === 'ENOENT' ? 'not found' : error.message})`);
        }
      }
    }
  }

  return {
    status: issues.length === 0 ? 'complete' : 'incomplete',
    packageRoot: root,
    commit: args.commit,
    architecture: args.architecture,
    releaseChannel: args.releaseChannel,
    filesChecked: packageFiles.length,
    issues,
  };
}
