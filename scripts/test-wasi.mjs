import { existsSync } from 'node:fs'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'

const require = createRequire(import.meta.url)
const loader = fileURLToPath(new URL('../tombit.wasi.cjs', import.meta.url))

if (!existsSync(loader)) {
  console.error(
    'tombit.wasi.cjs is missing; build the wasm32-wasip1-threads target first.',
  )
  process.exit(1)
}

const { formatSync } = require(loader)
const result = formatSync('a=1\n', 'a.toml', {
  // Keep schema lookup off so the smoke test never reaches the network.
  overrideConfig: { schema: { enabled: false } },
})

if (result.diagnostics.length !== 0 || result.formatted !== 'a = 1\n') {
  console.error('Unexpected WASI result:', result)
  process.exit(1)
}

console.log('WASI loader smoke test passed.')
