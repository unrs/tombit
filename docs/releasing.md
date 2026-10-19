# Releasing

The versions are `0.0.0`, so the `minor` changeset produces the first release as `0.1.0`.

npm only allows a trusted publisher to be configured for a package that already exists, and the publish
job authenticates with OIDC alone. `tombit@0.0.0` exists, and the `@tombit/binding-*` placeholders are
published manually, so configure a trusted publisher for `tombit` and for each `@tombit/binding-*`
package on npmjs.com before the first automated release.

If a release stops after publishing some of the platform packages, do not re-run it for the same
version: npm rejects publishing over an existing version, so bump the version and release again.
