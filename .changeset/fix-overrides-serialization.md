---
"tombit": patch
---

Serialize the resolved configuration with `toml_edit` instead of `serde_tombi`'s writer.

`serde_tombi`'s writer emits invalid TOML for configurations with `[[overrides]]`
(nested tables inside an array-of-tables element become `key = [table]`), so
discovery with such a `tombi.toml` panicked. The mirror issue is
[unrs/tombit#5](https://github.com/unrs/tombit/issues/5); this avoids the broken
writer while keeping the configuration semantics.
