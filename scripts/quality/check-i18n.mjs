import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const LANGUAGES = ['en', 'vi', 'zh', 'ko', 'ja', 'es', 'fr'];

export function flattenKeys(value, prefix = '') {
  const keys = [];
  for (const key of Object.keys(value).sort()) {
    const next = prefix ? `${prefix}.${key}` : key;
    const child = value[key];
    if (child && typeof child === 'object' && !Array.isArray(child)) {
      keys.push(...flattenKeys(child, next));
    } else {
      keys.push(next);
    }
  }
  return keys;
}

export function compareLocaleKeys(expected, actual) {
  const expectedSet = new Set(expected);
  const actualSet = new Set(actual);
  return {
    missing: expected.filter((key) => !actualSet.has(key)),
    extra: actual.filter((key) => !expectedSet.has(key)),
  };
}

export function validateLocales(root) {
  const localeRoot = path.join(path.resolve(root), 'locales');
  const canonical = JSON.parse(fs.readFileSync(path.join(localeRoot, 'en/common.json'), 'utf8'));
  const expected = flattenKeys(canonical);
  const issues = [];
  for (const language of LANGUAGES.slice(1)) {
    const file = path.join(localeRoot, language, 'common.json');
    const actual = flattenKeys(JSON.parse(fs.readFileSync(file, 'utf8')));
    const diff = compareLocaleKeys(expected, actual);
    if (diff.missing.length || diff.extra.length) issues.push({ language, ...diff });
  }
  return issues;
}

function main() {
  const issues = validateLocales(process.argv[2] ?? '.');
  if (issues.length === 0) {
    console.log('i18n key parity passed.');
    return;
  }
  for (const issue of issues) console.error(JSON.stringify(issue));
  process.exitCode = 1;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();
