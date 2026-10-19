# tombit

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
