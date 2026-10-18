#![deny(clippy::all)]

use std::path::{Path, PathBuf};

use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::Value;
use tombi_config::Config;
use tombi_lib::{ConfigInput, Options};

/// `serde_tombi`'s serializer formats its output through a `SchemaStore`, and an
/// unconfigured one loads the remote schemastore catalog. Rendering a merged
/// config therefore goes through this schema-disabled one.
const OFFLINE_CONFIG: &str = "[schema]\nenabled = false\n";
const OFFLINE_CONFIG_PATH: &str = "tombi.toml";

#[napi]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// A diagnostic reported by tombi.
#[napi(object)]
pub struct Diagnostic {
    /// `error` or `warning`.
    pub level: String,
    pub code: String,
    pub message: String,
}

/// The result of formatting a TOML document.
#[napi(object)]
pub struct FormatResult {
    /// The formatted source, or `null` when formatting failed.
    pub formatted: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
}

/// The result of linting a TOML document.
#[napi(object)]
pub struct LintResult {
    pub diagnostics: Vec<Diagnostic>,
}

fn to_diagnostics(diagnostics: &[tombi_lib::Diagnostic]) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .map(|diagnostic| Diagnostic {
            // `Level`'s `Display` adds ANSI colour; `as_str` is the plain name.
            level: diagnostic.level().as_str().to_lowercase(),
            code: diagnostic.code().to_owned(),
            message: diagnostic.message().to_owned(),
        })
        .collect()
}

/// Run a future to completion.
///
/// napi's async executor requires `Send` futures, and tombi's wasm futures are
/// deliberately `LocalBoxFuture`, so the exports are blocking: tokio drives them
/// natively, and the single-threaded `futures` executor drives them in a WASI
/// module. Neither awaits a timer -- schema lookup is disabled and the wasm
/// filesystem is in-memory.
#[cfg(not(target_family = "wasm"))]
fn block_on<F: std::future::Future>(future: F) -> Result<F::Output> {
    let runtime =
        tokio::runtime::Runtime::new().map_err(|error| Error::from_reason(error.to_string()))?;

    Ok(runtime.block_on(future))
}

#[cfg(target_family = "wasm")]
fn block_on<F: std::future::Future>(future: F) -> Result<F::Output> {
    Ok(futures::executor::block_on(future))
}

/// Discover tombi's configuration for a file, using tombi's own search order:
/// `.tombi.toml`, `tombi.toml`, `.config/tombi.toml` and `[tool.tombi]` in
/// `pyproject.toml` from the file's directory upward, then the user and system
/// config, then the defaults.
fn discover(source_path: &str) -> Result<(Config, Option<PathBuf>)> {
    let search_dir = Path::new(source_path)
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .or_else(|| std::env::current_dir().ok());

    serde_tombi::config::load_with_path(search_dir)
        .map_err(|error| Error::from_reason(error.to_string()))
}

/// Merge the JavaScript overrides into the discovered config. Objects merge
/// recursively, everything else is replaced.
fn merge(base: &mut Value, overrides: &Value) {
    match (base, overrides) {
        (Value::Object(base), Value::Object(overrides)) => {
            for (key, value) in overrides {
                match base.get_mut(key) {
                    Some(existing) => merge(existing, value),
                    None => {
                        base.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (base, overrides) => *base = overrides.clone(),
    }
}

/// Deserializing the merged value back into `Config` validates it:
/// `deny_unknown_fields` turns a misspelled option into an error instead of
/// silently dropping it.
fn merge_override(discovered: Config, overrides: Option<Value>) -> Result<Config> {
    let Some(overrides) = overrides else {
        return Ok(discovered);
    };

    let mut base =
        serde_json::to_value(&discovered).map_err(|error| Error::from_reason(error.to_string()))?;
    merge(&mut base, &overrides);

    serde_json::from_value(base)
        .map_err(|error| Error::from_reason(format!("invalid tombit config: {error}")))
}

/// `tombi-lib` accepts config as TOML text, so the merged config is rendered
/// back to text. Keeping the discovered path matters: `format_async` hands it to
/// `tombi_glob`, so the config's own `[[overrides]]` still match the file.
fn config_input(config: &Config, config_path: Option<PathBuf>) -> Result<ConfigInput> {
    let offline = serde_tombi::config::from_str(OFFLINE_CONFIG, Path::new(OFFLINE_CONFIG_PATH))
        .map_err(|error| Error::from_reason(error.to_string()))?;

    let serializer = serde_tombi::Serializer::builder().config(&offline).build();
    let content = block_on(serializer.to_string_async(config))?
        .map_err(|error| Error::from_reason(error.to_string()))?;

    Ok(match config_path {
        Some(path) => ConfigInput::File { content, path },
        None => ConfigInput::Text(content),
    })
}

fn options(source_path: &str, overrides: Option<Value>, discover_config: bool) -> Result<Options> {
    let (discovered, config_path) = if discover_config {
        discover(source_path)?
    } else {
        (Config::default(), None)
    };

    let config = merge_override(discovered, overrides)?;

    Ok(Options {
        config: Some(config_input(&config, config_path)?),
    })
}

/// Format a TOML document, using tombi's own configuration discovery and the
/// given overrides.
#[napi]
pub fn format(
    source: String,
    source_path: String,
    overrides: Option<Value>,
    discover: Option<bool>,
) -> Result<FormatResult> {
    let options = options(&source_path, overrides, discover.unwrap_or(true))?;

    let result = block_on(tombi_lib::format_async(source, source_path, options))?
        .map_err(|error| Error::from_reason(error.to_string()))?;

    Ok(FormatResult {
        formatted: result.formatted,
        diagnostics: to_diagnostics(&result.diagnostics),
    })
}

/// Lint a TOML document, using tombi's own configuration discovery and the given
/// overrides.
#[napi]
pub fn lint(
    source: String,
    source_path: String,
    overrides: Option<Value>,
    discover: Option<bool>,
) -> Result<LintResult> {
    let options = options(&source_path, overrides, discover.unwrap_or(true))?;

    let result = block_on(tombi_lib::lint_async(source, source_path, options))?
        .map_err(|error| Error::from_reason(error.to_string()))?;

    Ok(LintResult {
        diagnostics: to_diagnostics(&result.diagnostics),
    })
}

/// The configuration tombi would use for a file, after applying the overrides.
#[napi]
pub fn resolve_config(source_path: String, overrides: Option<Value>) -> Result<Value> {
    let (discovered, _) = discover(&source_path)?;
    let config = merge_override(discovered, overrides)?;

    serde_json::to_value(&config).map_err(|error| Error::from_reason(error.to_string()))
}

// Roadmap: `parse`/`stringify`. TOML -> JS needs an explicit date mapping
// (`serde_json::Value` cannot absorb tombi's datetime newtypes), and nothing in
// the current use cases needs it: config lives here now, and callers that only
// format never round-trip through a value.
