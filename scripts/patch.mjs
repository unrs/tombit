import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('..', import.meta.url))
const packageJson = JSON.parse(
  fs.readFileSync(path.join(root, 'package.json'), 'utf8'),
)

// The WebContainer fallback validates against this manifest. The real
// package.json intentionally does not list the WASI package (napi keeps it out
// of `optionalDependencies` so native installs do not download the wasm);
// `napi-postinstall/fallback` downloads it on demand in WebContainer instead.
fs.writeFileSync(
  path.join(root, 'fallback-package.json'),
  `${JSON.stringify(
    {
      name: packageJson.name,
      version: packageJson.version,
      napi: packageJson.napi,
      optionalDependencies: {
        [`${packageJson.napi.packageName}-wasm32-wasi`]: packageJson.version,
      },
    },
    null,
    2,
  )}\n`,
)

const indexPath = path.join(root, 'index.js')
let code = fs.readFileSync(indexPath, 'utf8')
if (!code.includes('napi-postinstall/fallback')) {
  const anchor = '\nif (!nativeBinding) {'
  if (!code.includes(anchor)) {
    throw new Error(`cannot find ${JSON.stringify(anchor)} in index.js`)
  }
  code = code.replace(
    anchor,
    `
if (!nativeBinding && globalThis.process?.versions?.["webcontainer"]) {
  try {
    nativeBinding = require('napi-postinstall/fallback')(
      require.resolve('./fallback-package.json'),
      true,
    )
  } catch (err) {
    loadErrors.push(err)
  }
}
` + anchor,
  )
  fs.writeFileSync(indexPath, code)
}
