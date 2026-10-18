import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    include: ['__test__/**/*.test.mjs'],
    // `tombi/` is a full checkout with its own test suites; only ours run here.
    exclude: ['node_modules/**', 'tombi/**', 'target/**'],
  },
})
