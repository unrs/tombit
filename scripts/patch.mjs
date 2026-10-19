import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('..', import.meta.url))
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
      require.resolve('./package.json'),
      true,
    )
    if (typeof nativeBinding.__napiBindingTarget === 'string') {
      __napiLoadedBindingTarget = nativeBinding.__napiBindingTarget
    }
  } catch (err) {
    loadErrors.push(err)
  }
}
` + anchor,
  )
  fs.writeFileSync(indexPath, code)
}
