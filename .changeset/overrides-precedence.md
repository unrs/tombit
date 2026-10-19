---
"tombit": patch
---

Apply `[[overrides]]` formatting and linting rules over the base rules.

Vendors tombi-toml/tombi#2330: a matching override was parsed and matched, but
its `format.rules`/`lint.rules` previously lost to the base rules.
