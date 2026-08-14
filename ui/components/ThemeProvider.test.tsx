import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { ThemeProvider } from './ThemeProvider';

describe('ThemeProvider', () => {
  it('applies explicit themes to the document', () => {
    const { rerender } = render(<ThemeProvider mode="light"><div /></ThemeProvider>);
    expect(document.documentElement.dataset.theme).toBe('light');
    rerender(<ThemeProvider mode="dark"><div /></ThemeProvider>);
    expect(document.documentElement.dataset.theme).toBe('dark');
  });
});
