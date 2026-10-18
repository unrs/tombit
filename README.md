# tombit

Unified toolkit for [tombi][]: TOML **format** and **lint** from Node.js and WebAssembly, with
tombi's own configuration discovery.

> **Status:** scaffold. `format`, `lint` and `resolveConfig` work on the native target and are covered
> by tests. Configuration discovery and merging live in Rust; JavaScript only supplies overrides.
> A `.wasm` build compiles and runs for WASI at the core level, but the napi `wasm32-wasip1-threads`
> link is not resolved yet.

## Why

`@tombi-toml/lib` is a native napi binding: it cannot run where native addons are disabled
(WebContainer, browsers), and it exposes only `format`/`lint`. tombit wraps the same crate that powers
tombi's own Node, Python and wasm bindings (`tombi-lib`) plus the config machinery from `serde_tombi`,
and is built for a WASI target too.

Keeping configuration in Rust is the point: `serde_tombi::config` already implements tombi's search
order (`.tombi.toml`, `tombi.toml`, `.config/tombi.toml`, `[tool.tombi]` in `pyproject.toml` from the
file's directory upward, then user and system config, then defaults), including `[[overrides]]` glob
matching. Callers don't need a TOML parser or serializer at all.

## Install

```sh
pnpm add tombit
```

Platform binaries arrive through the `@tombit/binding-*` optional packages, as with any napi-rs
module.

## Usage

```js
import { format, lint, resolveConfig } from 'tombit'

// Discovery happens in Rust, from the file's directory.
const { formatted, diagnostics } = format('a=1\n', '/project/a.toml', {
  schema: { enabled: false },
  format: { rules: { 'indent-width': 4 } },
})

const config = resolveConfig('/project/a.toml')
```

Overrides are a plain JavaScript object in tombi's own config shape, merged over the discovered
config (objects merge recursively, everything else replaces). They are deserialized into
`tombi_config::Config`, which uses `deny_unknown_fields`, so a misspelled option is an error rather
than a silently ignored setting.

## API

| Function                                       | Notes                                       |
| ---------------------------------------------- | ------------------------------------------- |
| `version()`                                    | package version                             |
| `format(source, sourcePath, overrides?, discover?)` | `{ formatted, diagnostics }`            |
| `lint(source, sourcePath, overrides?, discover?)`   | `{ diagnostics }`                       |
| `resolveConfig(sourcePath, overrides?)`        | the merged config as a plain object         |

`discover` defaults to `true`. Pass `false` to skip the filesystem search and use tombi's defaults
plus the overrides. The calls are synchronous: napi's async executor requires `Send` futures, and
tombi's wasm futures are deliberately `LocalBoxFuture`.

## Roadmap

- `parse` / `stringify`: deferred deliberately. TOML → JS needs an explicit date mapping
  (`serde_json::Value` cannot absorb tombi's datetime newtypes), and nothing in the current use cases
  needs a value-level round trip — configuration is handled in Rust.
- napi `wasm32-wasip1-threads` artifact: the core already compiles and runs for WASI; the remaining
  work is the emnapi link.
- Mounting files into the wasm virtual filesystem, so discovery can see a host-provided config in a
  browser/WebContainer build (the native build reads the real filesystem).

## Development

```sh
git clone --recurse-submodules git@github.com:unrs/tombit.git
pnpm install
pnpm build        # release; pnpm build:debug for a debug binding
pnpm test         # builds the debug binding, then runs Vitest
just check        # fmt + clippy + Rust and JS tests
```

## Vendoring

tombi lives in `vendor/tombi` as a git submodule pinned to a commit, because its crates are not
published on crates.io yet. The dependency is split by target:

- **native** — `tombi-lib` with the `native` feature: real filesystem and network, so discovery and
  schema catalogs behave like the CLI.
- **wasm** — `tombi-lib` without default features (in-memory filesystem, no HTTP client) plus
  `serde_tombi/wasm`. That feature is a cfg switch that drops `serde_tombi`'s sync `to_string`, which
  needs tokio's multi-thread runtime; it is not a request for the browser runtime.

Upgrading tombi is moving the submodule pin and rebuilding.

Publishing the crate itself is blocked while the tombi crates are unpublished, which is why
`Cargo.toml` sets `publish = false`.

## License

MIT

[tombi]: https://github.com/tombi-toml/tombi
