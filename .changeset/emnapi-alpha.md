---
"tombit": patch
---

Pin the WASI binding's emnapi dependencies to `2.0.0-alpha.6`.

`@napi-rs/wasm-runtime` 1.2.5 bundles the `2.0.0-alpha` emnapi plugin API. The
`2.0.0-beta` release no longer exposes `emnapi_shared.napiModule`, so loading the
WASI entry threw `TypeError: Cannot set properties of undefined (setting
'initWorker')`. Keep Renovate on the alpha line until the runtime supports beta.
