# Releasing

The versions are `0.0.0`, so the `minor` changeset produces the first release as `0.1.0`.

npm only allows a trusted publisher to be configured for a package that already exists, and the publish
job authenticates with OIDC alone. `tombit@0.0.0` exists, and the `@tombit/binding-*` placeholders are
published manually, so configure a trusted publisher for `tombit` and for each `@tombit/binding-*`
package on npmjs.com before the first automated release.

If a release stops after publishing some of the platform packages, do not re-run it for the same
version: npm rejects publishing over an existing version, so bump the version and release again.

## WASI dependency compatibility

`@napi-rs/wasm-runtime` bundles the emnapi async-work and TSFN plugins into its own `dist` at build
time. `1.2.5` bundles the `2.0.0-alpha` plugin, which writes `emnapi_shared.napiModule`; emnapi
`2.0.0-beta` renamed that internal property to `addonModule`, so the two generations cannot be mixed.
Upstream is tracking the fallback in [napi-rs/napi-rs#3621](https://github.com/napi-rs/napi-rs/issues/3621).
Until a runtime built against beta ships:

- keep `@emnapi/core`, `@emnapi/runtime`, and `emnapi` on the `alpha` npm dist-tag
  (`followTag: "alpha"`); a tag move is surfaced for approval instead of being silently filtered by a
  version cap;
- hold `@napi-rs/wasm-runtime` updates for approval in the Dependency Dashboard; it follows its own
  release channel, and the WASI smoke test checks the pairing with the alpha line.

`pnpm test:wasi` loads the generated WASI loader and formats a small document. CI runs it in the
`wasi` job, and the release workflow runs it for the `wasm32-wasip1-threads` build before uploading
the artifact. To move to beta, bump the four packages together, approve the runtime update from the
Dependency Dashboard, and rerun the smoke test.
