#![deny(clippy::all)]

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

use napi::bindgen_prelude::*;
use napi::{Env, Task};
use napi_derive::napi;
use serde_json::Value;
use tombi_config::Config;
use tombi_lib::{ConfigInput, Options as TombiOptions};

/// `serde_tombi`'s serializer formats its output through a `SchemaStore`, and an
/// unconfigured one loads the remote schemastore catalog. Rendering a merged
/// config therefore goes through this schema-disabled one.
const OFFLINE_CONFIG: &str = "[schema]\nenabled = false\n";
const OFFLINE_CONFIG_PATH: &str = "tombi.toml";

/// Whether this build must force `schema.enabled = false` because it has no
/// HTTP client to resolve a catalog with.
const FORCED_OFFLINE: bool = cfg!(any(target_family = "wasm", not(feature = "schema")));

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

/// A zero-based position in a TOML document.
#[napi(object)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}

/// A range in a TOML document, with zero-based line and column numbers.
#[napi(object)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// A diagnostic reported by tombi.
#[napi(object)]
pub struct Diagnostic {
    /// `error` or `warning`.
    #[napi(ts_type = "\"error\" | \"warning\"")]
    pub level: String,
    pub code: String,
    pub message: String,
    pub range: Range,
    /// The file the diagnostic points into, when tombi knows it.
    pub source_file: Either<String, Null>,
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
        .map(|diagnostic| {
            let range = diagnostic.range();
            Diagnostic {
                // Mirror `tombi-lib-node`: `Level` only has ERROR and WARNING,
                // and `as_str` returns the capitalised, display oriented name.
                level: if diagnostic.is_error() {
                    "error"
                } else {
                    "warning"
                }
                .to_owned(),
                code: diagnostic.code().to_owned(),
                message: diagnostic.message().to_owned(),
                range: Range {
                    start: Position {
                        line: range.start.line,
                        column: range.start.column,
                    },
                    end: Position {
                        line: range.end.line,
                        column: range.end.column,
                    },
                },
                source_file: match diagnostic.source_file() {
                    Some(path) => Either::A(path.to_string_lossy().into_owned()),
                    None => Either::B(Null),
                },
            }
        })
        .collect()
}

#[cfg(not(target_family = "wasm"))]
thread_local! {
    // One current-thread runtime per libuv worker, instead of building a
    // multi-thread runtime for every call.
    static RUNTIME: std::cell::RefCell<Option<tokio::runtime::Runtime>> =
        const { std::cell::RefCell::new(None) };
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
    RUNTIME.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            *slot = Some(
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|error| Error::from_reason(error.to_string()))?,
            );
        }

        Ok(slot.as_ref().expect("runtime installed").block_on(future))
    })
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

/// A config resolved once and reused across calls: a formatter calls
/// `format`/`lint` per file, and discovery, merging and serialization would
/// otherwise run for every one of them.
#[derive(Clone)]
struct ResolvedConfig {
    content: String,
    config_path: Option<PathBuf>,
    config_modified: Option<SystemTime>,
    dir_modified: Option<SystemTime>,
}

impl ResolvedConfig {
    fn is_valid(&self, source_dir: &Path) -> bool {
        if let (Some(path), Some(modified)) = (&self.config_path, self.config_modified) {
            match modified_at(path) {
                Some(current) if current == modified => {}
                _ => return false,
            }
        }
        if let Some(modified) = self.dir_modified {
            match modified_at(source_dir) {
                Some(current) if current == modified => {}
                _ => return false,
            }
        }
        true
    }
}

static CONFIG_CACHE: LazyLock<Mutex<HashMap<String, ResolvedConfig>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
const CONFIG_CACHE_LIMIT: usize = 256;

fn modified_at(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

fn source_dir(source_path: &str) -> &Path {
    Path::new(source_path)
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn hash_str(value: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn config_cache_key(source_path: &str, options: &Options) -> Option<String> {
    let overrides = serde_json::to_string(&options.override_config).ok()?;
    let source = match &options.config {
        None => format!("dir:{}", source_dir(source_path).display()),
        Some(Either3::A(false)) => "defaults".to_owned(),
        Some(Either3::A(true)) => return None,
        Some(Either3::B(content)) => format!("text:{}", hash_str(content)),
        Some(Either3::C(file)) => format!("file:{}:{}", file.path, hash_str(&file.content)),
    };
    Some(format!("{source}\u{1f}{overrides}"))
}

fn config_cache_get(key: &str, source_dir: &Path) -> Option<ConfigInput> {
    let entry = {
        let cache = CONFIG_CACHE
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        cache.get(key).cloned()?
    };
    if !entry.is_valid(source_dir) {
        return None;
    }
    Some(match &entry.config_path {
        Some(path) => ConfigInput::File {
            content: entry.content,
            path: path.clone(),
        },
        None => ConfigInput::Text(entry.content),
    })
}

fn config_cache_put(
    key: String,
    content: String,
    config_path: Option<PathBuf>,
    config_modified: Option<SystemTime>,
    dir_modified: Option<SystemTime>,
) {
    let mut cache = CONFIG_CACHE
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if cache.len() >= CONFIG_CACHE_LIMIT {
        cache.clear();
    }
    cache.insert(
        key,
        ResolvedConfig {
            content,
            config_path,
            config_modified,
            dir_modified,
        },
    );
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
    // Without an override there is nothing to merge, so hand tombi-lib the
    // config directly instead of parsing it into `Config` and re-serializing it.
    if options.override_config.is_none() {
        match &options.config {
            Some(Either3::A(false)) => {
                let content = if FORCED_OFFLINE { OFFLINE_CONFIG } else { "" };
                return Ok(TombiOptions {
                    config: Some(ConfigInput::Text(content.to_owned())),
                });
            }
            Some(Either3::B(content)) if !FORCED_OFFLINE => {
                return Ok(TombiOptions {
                    config: Some(ConfigInput::Text(content.clone())),
                });
            }
            Some(Either3::C(file)) if !FORCED_OFFLINE => {
                return Ok(TombiOptions {
                    config: Some(ConfigInput::File {
                        content: file.content.clone(),
                        path: file.path.clone().into(),
                    }),
                });
            }
            _ => {}
        }
    }

    let dir = source_dir(source_path);
    let cache_key = config_cache_key(source_path, options);
    if let Some(key) = &cache_key {
        if let Some(config) = config_cache_get(key, dir) {
            return Ok(TombiOptions {
                config: Some(config),
            });
        }
    }

    let (base, config_path) = base_config(source_path, options)?;
    let config = merge_override(base, options.override_config.clone())?;
    // WASI never resolves schemas, and a native build without the schema client
    // cannot resolve a catalog — either way a caller must not be able to switch
    // it back on. Enforce this on the final merged config. A native build with
    // the client keeps tombi's default and leaves the decision to the caller,
    // which passes `schema: { enabled: false }` to stay offline.
    #[cfg(any(target_family = "wasm", not(feature = "schema")))]
    let config = merge_override(
        config,
        Some(serde_json::json!({ "schema": { "enabled": false } })),
    )?;
    let input = config_input(&config, config_path)?;

    if let Some(key) = cache_key {
        let discovered = options.config.is_none();
        let (content, cached_path) = match &input {
            ConfigInput::Text(content) => (content.clone(), None),
            ConfigInput::File { content, path } => (content.clone(), Some(path.clone())),
        };
        let config_modified = discovered
            .then(|| cached_path.as_deref().and_then(modified_at))
            .flatten();
        let dir_modified = discovered.then(|| modified_at(dir)).flatten();
        config_cache_put(key, content, cached_path, config_modified, dir_modified);
    }

    Ok(TombiOptions {
        config: Some(input),
    })
}

/// Format a TOML document asynchronously, using tombi's own configuration
/// discovery and the given override config.
#[napi(ts_return_type = "Promise<FormatResult>")]
pub fn format(
    source: String,
    source_path: String,
    options: Option<Options>,
) -> AsyncTask<FormatTask> {
    AsyncTask::new(FormatTask {
        source,
        source_path,
        options: options.unwrap_or_default(),
    })
}

/// Format a TOML document synchronously, blocking the calling thread.
#[napi]
pub fn format_sync(
    source: String,
    source_path: String,
    options: Option<Options>,
) -> Result<FormatResult> {
    format_impl(&source, &source_path, &options.unwrap_or_default())
}

pub struct FormatTask {
    source: String,
    source_path: String,
    options: Options,
}

impl Task for FormatTask {
    type Output = FormatResult;
    type JsValue = FormatResult;

    fn compute(&mut self) -> Result<Self::Output> {
        format_impl(&self.source, &self.source_path, &self.options)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

fn format_impl(source: &str, source_path: &str, options: &Options) -> Result<FormatResult> {
    let tombi_options = tombi_options(source_path, options)?;
    let result = block_on(tombi_lib::format_async(
        source.to_owned(),
        source_path.to_owned(),
        tombi_options,
    ))?
    .map_err(|error| Error::from_reason(error.to_string()))?;

    Ok(FormatResult {
        formatted: result.formatted,
        diagnostics: to_diagnostics(&result.diagnostics),
    })
}

/// Lint a TOML document asynchronously, using tombi's own configuration discovery
/// and the given override config.
#[napi(ts_return_type = "Promise<LintResult>")]
pub fn lint(source: String, source_path: String, options: Option<Options>) -> AsyncTask<LintTask> {
    AsyncTask::new(LintTask {
        source,
        source_path,
        options: options.unwrap_or_default(),
    })
}

/// Lint a TOML document synchronously, blocking the calling thread.
#[napi]
pub fn lint_sync(
    source: String,
    source_path: String,
    options: Option<Options>,
) -> Result<LintResult> {
    lint_impl(&source, &source_path, &options.unwrap_or_default())
}

pub struct LintTask {
    source: String,
    source_path: String,
    options: Options,
}

impl Task for LintTask {
    type Output = LintResult;
    type JsValue = LintResult;

    fn compute(&mut self) -> Result<Self::Output> {
        lint_impl(&self.source, &self.source_path, &self.options)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

fn lint_impl(source: &str, source_path: &str, options: &Options) -> Result<LintResult> {
    let tombi_options = tombi_options(source_path, options)?;
    let result = block_on(tombi_lib::lint_async(
        source.to_owned(),
        source_path.to_owned(),
        tombi_options,
    ))?
    .map_err(|error| Error::from_reason(error.to_string()))?;

    Ok(LintResult {
        diagnostics: to_diagnostics(&result.diagnostics),
    })
}

// Roadmap: `parse`/`stringify`. TOML -> JS needs an explicit date mapping
// (`serde_json::Value` cannot absorb tombi's datetime newtypes), and nothing in
// the current use cases needs it: config lives here now, and callers that only
// format never round-trip through a value.
