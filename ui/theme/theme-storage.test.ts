import { beforeEach, describe, expect, it } from 'vitest';
import { loadThemeMode, saveThemeMode } from './theme-storage';

describe('theme storage', () => {
  beforeEach(() => localStorage.clear());

  it('defaults to system and persists approved values', () => {
    expect(loadThemeMode()).toBe('system');
    saveThemeMode('dark');
    expect(loadThemeMode()).toBe('dark');
  });

  it('rejects unexpected stored values', () => {
    localStorage.setItem('voxveil.theme', 'remote-neon');
    expect(loadThemeMode()).toBe('system');
  });
});
