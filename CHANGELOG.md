# tombit

## 0.1.5

### Patch Changes

- [#13](https://github.com/unrs/tombit/pull/13) [`f1a8a97`](https://github.com/unrs/tombit/commit/f1a8a9782ebadd81be07081b396006c062bdcd6d) Thanks [@JounQin](https://github.com/JounQin)! - Fix the WebContainer fallback throwing `ERR_NAPI_BINDING_TARGET_CONFLICT`.
  
  The fallback now adopts the WASI binding's `__napiBindingTarget` instead of
  stamping it with the root loader's default `'native'` target.

## 0.1.4

### Patch Changes

- [#11](https://github.com/unrs/tombit/pull/11) [`4ce2e04`](https://github.com/unrs/tombit/commit/4ce2e047ca42e287ebbab7358e5e2aa8a819b097) Thanks [@JounQin](https://github.com/JounQin)! - Publish the WASI build as the `@tombit/binding-wasm32-wasi` optional dependency
  instead of embedding it in the main package.
  
  Only `wasm32-wasip1-threads` is built now, which is the target napi-rs
  generates. The main package also gains a `browser.js` entry that re-exports the
  WASI package for browser bundlers, and a WebContainer fallback: when
  `process.versions.webcontainer` is set and no native binding loaded,
  `napi-postinstall/fallback` downloads the WASI binding on demand based on
  `napi.targets`. The non-threaded `wasm32-wasip1` build made the loader resolve a local
  wasm copy, so every `tombit` install carried about 6 MB of WebAssembly.

## 0.1.3

### Patch Changes

- [#9](https://github.com/unrs/tombit/pull/9) [`c1bce0b`](https://github.com/unrs/tombit/commit/c1bce0bd26d83389df43b3d23bba8b78757d864f) Thanks [@JounQin](https://github.com/JounQin)! - Apply `[[overrides]]` formatting and linting rules over the base rules.
  
  Vendors tombi-toml/tombi#2330: a matching override was parsed and matched, but
  its `format.rules`/`lint.rules` previously lost to the base rules.

## 0.1.2

### Patch Changes

- [#6](https://github.com/unrs/tombit/pull/6) [`2c674e6`](https://github.com/unrs/tombit/commit/2c674e65d3ed1f70afd537187b57196cb52ee26d) Thanks [@JounQin](https://github.com/JounQin)! - Serialize the resolved configuration with `toml_edit` instead of `serde_tombi`'s writer.
  
  `serde_tombi`'s writer emits invalid TOML for configurations with `[[overrides]]`
  (nested tables inside an array-of-tables element become `key = [table]`), so
  discovery with such a `tombi.toml` panicked. The mirror issue is
  [unrs/tombit#5](https://github.com/unrs/tombit/issues/5); this avoids the broken
  writer while keeping the configuration semantics.

## 0.1.1

### Patch Changes

- [#3](https://github.com/unrs/tombit/pull/3) [`a12c974`](https://github.com/unrs/tombit/commit/a12c9748cf67ff0926933456828e6a459a5b542b) Thanks [@JounQin](https://github.com/JounQin)! - Expose `range` and `sourceFile` on diagnostics, matching `@tombi-toml/lib` so callers can render code frames and point at the offending source.

## 0.1.0

### Minor Changes

- [#1](https://github.com/unrs/tombit/pull/1) [`733aef2`](https://github.com/unrs/tombit/commit/733aef2abd5436ee99dc39fd4943944e78cbbcb5) Thanks [@JounQin](https://github.com/JounQin)! - Initial release: async and sync `format` and `lint` over tombi's own configuration discovery, with JavaScript overrides merged and validated in Rust, on native and WASI.
