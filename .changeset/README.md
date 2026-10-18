# Changesets

Run `pnpm changeset` to record a change. The release workflow turns these into a version pull request,
then publishes the package and its platform binaries.

## Releasing

The versions are `0.0.0`, so the `minor` changeset produces the first release as `0.1.0`.

npm only allows a trusted publisher to be configured for a package that already exists, and the publish
job authenticates with OIDC alone. `tombit@0.0.0` is published, but the four `@tombit/binding-*`
packages are not, so before the first release:

1. Publish a `0.0.0` placeholder for each platform package, using a granular token that can publish
   under `@tombit`. A minimal `package.json` with `name` and `version` is enough.
2. Configure a trusted publisher for `tombit` and for each `@tombit/binding-*` package on npmjs.com.
3. Remove the token.

If a release stops after publishing some of the platform packages, do not re-run it for the same
version: npm rejects publishing over an existing version, so bump the version and release again.
