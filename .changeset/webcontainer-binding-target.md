---
"tombit": patch
---

Fix the WebContainer fallback throwing `ERR_NAPI_BINDING_TARGET_CONFLICT`.

The fallback now adopts the WASI binding's `__napiBindingTarget` instead of
stamping it with the root loader's default `'native'` target.
