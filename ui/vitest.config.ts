import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./test/setup.ts'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json-summary'],
      thresholds: { lines: 85, functions: 85, branches: 85, statements: 85 },
      exclude: ['**/*.test.*', 'test/**', 'app/main.tsx'],
    },
  },
});
