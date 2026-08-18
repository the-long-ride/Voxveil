import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';

const css = fs.readFileSync(path.join(process.cwd(), 'ui', 'theme', 'base.css'), 'utf8');

test('uses a thin transparent track and rounded hoverable thumb', () => {
  assert.match(css, /scrollbar-width:\s*thin/);
  assert.match(css, /::-webkit-scrollbar\s*\{/);
  assert.match(css, /::-webkit-scrollbar-track\s*\{[^}]*background:\s*transparent/s);
  assert.match(css, /::-webkit-scrollbar-thumb\s*\{[^}]*border-radius:/s);
  assert.match(css, /::-webkit-scrollbar-thumb:hover\s*\{/);
});
