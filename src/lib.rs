#![deny(clippy::all)]

use std::path::{Path, PathBuf};

use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::Value;
use tombi_config::Config;
use tombi_lib::{ConfigInput, Options as TombiOptions};

/// `serde_tombi`'s serializer formats its output through a `SchemaStore`, and an
/// unconfigured one loads the remote schemastore catalog. Rendering a merged
/// config therefore goes through this schema-disabled one.
const OFFLINE_CONFIG: &str = "[schema]\nenabled = false\n";
const OFFLINE_CONFIG_PATH: &str = "tombi.toml";

/// The content of a `tombi.toml` config file at a given path.
#[napi(object)]
pub struct ConfigFile {
    pub content: String,
    pub path: String,
}

/// Options shared by [`format`] and [`lint`], mirroring `tombi-lib`'s.
#[napi(object)]
#[derive(Default)]
pub struct Options {
    /// A config to use instead of discovering one, or `false` to use tombi's
    /// defaults without discovering anything.
    #[napi(ts_type = "false | string | ConfigFile")]
    pub config: Option<Either3<bool, String, ConfigFile>>,
    /// Config merged over the base — the discovered config, or `config` when it
    /// is given. Objects merge recursively, everything else is replaced.
    pub override_config: Option<Value>,
}

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

/// Merge the JavaScript overrides into the base config. Objects merge
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
fn merge_override(base: Config, overrides: Option<Value>) -> Result<Config> {
    let Some(overrides) = overrides else {
        return Ok(base);
    };

    let mut base =
        serde_json::to_value(&base).map_err(|error| Error::from_reason(error.to_string()))?;
    merge(&mut base, &overrides);

    serde_json::from_value(base)
        .map_err(|error| Error::from_reason(format!("invalid tombit config: {error}")))
}

/// The base config: the one the caller supplied, or tombi's discovery.
fn base_config(source_path: &str, options: &Options) -> Result<(Config, Option<PathBuf>)> {
    match &options.config {
        Some(Either3::A(false)) => Ok((Config::default(), None)),
        Some(Either3::A(true)) => Err(Error::from_reason(
            "`config: true` is not meaningful: pass a config string or object, or `false` to use the defaults",
        )),
        Some(Either3::B(content)) => {
            let path = PathBuf::from(OFFLINE_CONFIG_PATH);
            let config = serde_tombi::config::from_str(content, &path)
                .map_err(|error| Error::from_reason(error.to_string()))?;
            Ok((config, Some(path)))
        }
        Some(Either3::C(file)) => {
            let path = PathBuf::from(&file.path);
            let config = serde_tombi::config::from_str(&file.content, &path)
                .map_err(|error| Error::from_reason(error.to_string()))?;
            Ok((config, Some(path)))
        }
        None => discover(source_path),
    }
}

/// `tombi-lib` accepts config as TOML text, so the merged config is rendered
/// back to text. Keeping the config's path matters: `format_async` hands it to
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

fn tombi_options(source_path: &str, options: &Options) -> Result<TombiOptions> {
    let (base, config_path) = base_config(source_path, options)?;
    // The binding is built without an HTTP client (see `Cargo.toml`), so schema
    // resolution would fail on the first catalog fetch. Disable it in the base
    // config; `enabled = true` survives the round-trip as serde's default, so a
    // caller can still opt in (and gets a clear fetch error if it does).
    let base = merge_override(
        base,
        Some(serde_json::json!({ "schema": { "enabled": false } })),
    )?;
    let config = merge_override(base, options.override_config.clone())?;

    Ok(TombiOptions {
        config: Some(config_input(&config, config_path)?),
    })
}

/// Format a TOML document, using tombi's own configuration discovery and the
/// given override config.
#[napi]
pub fn format(
    source: String,
    source_path: String,
    options: Option<Options>,
) -> Result<FormatResult> {
    let options = options.unwrap_or_default();
    let tombi_options = tombi_options(&source_path, &options)?;

    let result = block_on(tombi_lib::format_async(source, source_path, tombi_options))?
        .map_err(|error| Error::from_reason(error.to_string()))?;

    Ok(FormatResult {
        formatted: result.formatted,
        diagnostics: to_diagnostics(&result.diagnostics),
    })
}

/// Lint a TOML document, using tombi's own configuration discovery and the given
/// override config.
#[napi]
pub fn lint(source: String, source_path: String, options: Option<Options>) -> Result<LintResult> {
    let options = options.unwrap_or_default();
    let tombi_options = tombi_options(&source_path, &options)?;

    let result = block_on(tombi_lib::lint_async(source, source_path, tombi_options))?
        .map_err(|error| Error::from_reason(error.to_string()))?;

    Ok(LintResult {
        diagnostics: to_diagnostics(&result.diagnostics),
    })
}

// Roadmap: `parse`/`stringify`. TOML -> JS needs an explicit date mapping
// (`serde_json::Value` cannot absorb tombi's datetime newtypes), and nothing in
// the current use cases needs it: config lives here now, and callers that only
// format never round-trip through a value.
