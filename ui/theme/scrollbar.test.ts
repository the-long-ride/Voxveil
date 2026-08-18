import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';

describe('scrollbar styling', () => {
  it('uses a thin transparent track and rounded hoverable thumb', () => {
    const css = fs.readFileSync(path.resolve(process.cwd(), 'theme/base.css'), 'utf8');
    expect(css).toMatch(/scrollbar-width:\s*thin/);
    expect(css).toMatch(/::-webkit-scrollbar\s*\{/);
    expect(css).toMatch(/::-webkit-scrollbar-track\s*\{[^}]*background:\s*transparent/s);
    expect(css).toMatch(/::-webkit-scrollbar-thumb\s*\{[^}]*border-radius:/s);
    expect(css).toMatch(/::-webkit-scrollbar-thumb:hover\s*\{/);
  });
});
