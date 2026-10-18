# tombit

Unified toolkit for [tombi][]: TOML **format** and **lint** from Node.js and WebAssembly, with
tombi's own configuration discovery.

> **Status:** scaffold. `format` and `lint` work on the native and WASI targets and are covered by
> tests. Configuration discovery and merging live in Rust; JavaScript only supplies overrides. WASI
> keeps schema resolution off; the `@emnapi/core` patch in `patches/` makes the generated loader work
> with emnapi v2.

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
import { format, lint } from 'tombit'

const { formatted, diagnostics } = await format('a=1\n', '/project/a.toml', {
  overrideConfig: {
    schema: { enabled: false },
    format: { rules: { 'indent-width': 4 } },
  },
})
```

The shape mirrors `tombi-lib`: `(source, sourcePath, options)`, where `options.config` is a
`string`, a `{ content, path }` file, or `false`. Supplying a config uses it instead of discovering
one; `false` uses tombi's defaults without discovering anything; leaving it out discovers from
`sourcePath`'s directory.

`overrideConfig` is a plain JavaScript object in tombi's own config shape. It is merged over the base
config (objects recursively, everything else replaces) and deserialized into `tombi_config::Config`,
whose `deny_unknown_fields` turns a misspelled option into an error rather than a silently ignored
setting.

## API

| Function                                    | Notes                                  |
| ------------------------------------------- | -------------------------------------- |
| `version()`                                 | package version                        |
| `format(source, sourcePath, options?)`      | `Promise<{ formatted, diagnostics }>`  |
| `formatSync(source, sourcePath, options?)`  | `{ formatted, diagnostics }`           |
| `lint(source, sourcePath, options?)`        | `Promise<{ diagnostics }>`             |
| `lintSync(source, sourcePath, options?)`    | `{ diagnostics }`                      |

`options`:

| Option           | Meaning                                                                                       |
| ---------------- | --------------------------------------------------------------------------------------------- |
| `config`         | `false \| string \| { content, path }`; replaces discovery (`false` keeps the defaults)        |
| `overrideConfig` | merged over the base config                                                                   |

Schema resolution is compiled in only where the target can build `ring`: linux x64/arm64 (gnu and
musl), macOS, Windows and FreeBSD. Those builds follow tombi's default (on), and a formatter that
wants to stay offline passes `overrideConfig: { schema: { enabled: false } }`. The other targets
(linux armv7/ppc64le/s390x/riscv64/loongarch64, android, ohos, wasm) ship the offline stub and force
it off, because a fetch there would fail with `No HTTP client feature enabled`.

`format`/`lint` return Promises and run the work on napi's thread pool, so they never block the
event loop; `formatSync`/`lintSync` block the calling thread instead.

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

tombi lives in `tombi/` at the repository root as a git submodule pinned to a commit, because its
crates are not published on crates.io yet. The dependency is split by target:

- **native** — `tombi-lib`, with our `schema` feature adding its `native` one. `tombi-fs` keys its real
  filesystem off the target rather than the feature, so discovery works either way. `schema` pulls in
  `tombi-schema-store/native` → `reqwest` → `rustls`/`ring`; `ring`'s C/assembly build only works on a
  target with a supported toolchain, so the release builds enable it where it does and omit it
  elsewhere (same API, offline stub there).
- **wasm** — `tombi-lib` without default features (in-memory filesystem, no HTTP client) plus
  `serde_tombi/wasm`. That feature is a cfg switch that drops `serde_tombi`'s sync `to_string`, which
  needs tokio's multi-thread runtime; it is not a request for the browser runtime.

Upgrading tombi is moving the submodule pin and rebuilding.

Publishing the crate itself is blocked while the tombi crates are unpublished, which is why
`Cargo.toml` sets `publish = false`.

## License

MIT

[tombi]: https://github.com/tombi-toml/tombi
