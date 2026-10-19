---
"tombit": patch
---

Publish the WASI build as the `@tombit/binding-wasm32-wasi` optional dependency
instead of embedding it in the main package.

Only `wasm32-wasip1-threads` is built now, which is the target napi-rs
generates. The main package also gains a `browser.js` entry that re-exports the
WASI package for browser bundlers, and a WebContainer fallback: when
`process.versions.webcontainer` is set and no native binding loaded,
`napi-postinstall/fallback` downloads the WASI binding on demand. The non-threaded `wasm32-wasip1` build made the loader resolve a local
wasm copy, so every `tombit` install carried about 6 MB of WebAssembly.
