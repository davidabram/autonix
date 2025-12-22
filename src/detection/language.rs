use serde::Serialize;
pub use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub enum Language {
    Go,
    Rust,
    Python,
    JavaScript,
}

#[derive(Debug, Clone, Serialize)]
pub enum LanguageDetectionSource {
    //Go
    GoMod,
    GoWork,
    GoSum,
    GoVersionFile,
    GoFile,

    //Rust
    CargoToml,
    CargoLock,
    RustToolchain,
    RustToolchainToml,
    RsFile,

    //Python,
    RequirementsTxt,
    PyprojectToml,
    Pipfile,
    PipfileLock,
    PoetryLock,
    SetupPy,
    SetupCfg,
    EnvironmentYml,
    PythonVersionFile,
    PyFile,

    //NodeJS
    PackageJson,
    PackageLockJson,
    YarnLock,
    PnpmLockYaml,
    BunLock,
    BunLockb,
    LockJson,
    DenoLock,
    DenoJson,
    DenoJsonc,
    TsConfig,
    JsConfig,
    NvmrcFile,
    NodeVersionFile,
    BunVersionFile,
    JsFile,
    MjsFile,
    CjsFile,
    TsFile,
    JsxFile,
    TsxFile,
}

#[derive(Debug, Serialize)]
pub struct DetectedSource {
    pub source: LanguageDetectionSource,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct LanguageDetection {
    pub language: Language,
    pub detected_from: Vec<DetectedSource>,
}

pub trait LanguageDetector {
    fn detect(&self, path: &Path) -> Option<LanguageDetection>;
}

struct DetectorConfig {
    language: Language,
    file_patterns: &'static [(&'static str, LanguageDetectionSource)],
    extension_patterns: &'static [(&'static str, LanguageDetectionSource)],
}

fn detect_language(config: &DetectorConfig, path: &Path) -> Option<LanguageDetection> {
    let mut detected_from = Vec::new();
    let mut found_extensions = std::collections::HashSet::new();

    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
    {
        let entry_path = entry.path();

        if entry.file_type().is_file() {
            if let Some(filename) = entry_path.file_name().and_then(|n| n.to_str()) {
                for (pattern, source) in config.file_patterns {
                    if filename == *pattern {
                        detected_from.push(DetectedSource {
                            source: source.clone(),
                            path: Some(entry_path.to_path_buf()),
                        });
                        break;
                    }
                }
            }

            if let Some(ext) = entry_path.extension().and_then(|ext| ext.to_str()) {
                for (pattern, source) in config.extension_patterns {
                    if ext == *pattern && !found_extensions.contains(*pattern) {
                        found_extensions.insert(*pattern);
                        detected_from.push(DetectedSource {
                            source: source.clone(),
                            path: None,
                        });
                        break;
                    }
                }
            }
        }
    }

    if detected_from.is_empty() {
        None
    } else {
        Some(LanguageDetection {
            language: config.language.clone(),
            detected_from,
        })
    }
}

const GO_CONFIG: DetectorConfig = DetectorConfig {
    language: Language::Go,
    file_patterns: &[
        ("go.mod", LanguageDetectionSource::GoMod),
        ("go.work", LanguageDetectionSource::GoWork),
        ("go.sum", LanguageDetectionSource::GoSum),
        (".go-version", LanguageDetectionSource::GoVersionFile),
    ],
    extension_patterns: &[("go", LanguageDetectionSource::GoFile)],
};

const RUST_CONFIG: DetectorConfig = DetectorConfig {
    language: Language::Rust,
    file_patterns: &[
        ("Cargo.toml", LanguageDetectionSource::CargoToml),
        ("Cargo.lock", LanguageDetectionSource::CargoLock),
        ("rust-toolchain", LanguageDetectionSource::RustToolchain),
        (
            "rust-toolchain.toml",
            LanguageDetectionSource::RustToolchainToml,
        ),
    ],
    extension_patterns: &[("rs", LanguageDetectionSource::RsFile)],
};

const PYTHON_CONFIG: DetectorConfig = DetectorConfig {
    language: Language::Python,
    file_patterns: &[
        ("requirements.txt", LanguageDetectionSource::RequirementsTxt),
        ("pyproject.toml", LanguageDetectionSource::PyprojectToml),
        ("Pipfile", LanguageDetectionSource::Pipfile),
        ("Pipfile.lock", LanguageDetectionSource::PipfileLock),
        ("poetry.lock", LanguageDetectionSource::PoetryLock),
        ("setup.py", LanguageDetectionSource::SetupPy),
        ("setup.cfg", LanguageDetectionSource::SetupCfg),
        ("environment.yml", LanguageDetectionSource::EnvironmentYml),
        (".python-version", LanguageDetectionSource::PythonVersionFile),
    ],
    extension_patterns: &[("py", LanguageDetectionSource::PyFile)],
};

const JAVASCRIPT_CONFIG: DetectorConfig = DetectorConfig {
    language: Language::JavaScript,
    file_patterns: &[
        ("package.json", LanguageDetectionSource::PackageJson),
        (
            "package-lock.json",
            LanguageDetectionSource::PackageLockJson,
        ),
        ("yarn.lock", LanguageDetectionSource::YarnLock),
        ("pnpm-lock.yaml", LanguageDetectionSource::PnpmLockYaml),
        ("bun.lock", LanguageDetectionSource::BunLock),
        ("bun.lockb", LanguageDetectionSource::BunLockb),
        ("lock.json", LanguageDetectionSource::LockJson),
        ("deno.lock", LanguageDetectionSource::DenoLock),
        ("deno.json", LanguageDetectionSource::DenoJson),
        ("deno.jsonc", LanguageDetectionSource::DenoJsonc),
        ("tsconfig.json", LanguageDetectionSource::TsConfig),
        ("jsconfig.json", LanguageDetectionSource::JsConfig),
        (".nvmrc", LanguageDetectionSource::NvmrcFile),
        (".node-version", LanguageDetectionSource::NodeVersionFile),
        (".bun-version", LanguageDetectionSource::BunVersionFile),
    ],
    extension_patterns: &[
        ("js", LanguageDetectionSource::JsFile),
        ("mjs", LanguageDetectionSource::MjsFile),
        ("cjs", LanguageDetectionSource::CjsFile),
        ("ts", LanguageDetectionSource::TsFile),
        ("jsx", LanguageDetectionSource::JsxFile),
        ("tsx", LanguageDetectionSource::TsxFile),
    ],
};

pub struct GoDetector;

impl LanguageDetector for GoDetector {
    fn detect(&self, path: &Path) -> Option<LanguageDetection> {
        detect_language(&GO_CONFIG, path)
    }
}

pub struct RustDetector;

impl LanguageDetector for RustDetector {
    fn detect(&self, path: &Path) -> Option<LanguageDetection> {
        detect_language(&RUST_CONFIG, path)
    }
}

pub struct PythonDetector;

impl LanguageDetector for PythonDetector {
    fn detect(&self, path: &Path) -> Option<LanguageDetection> {
        detect_language(&PYTHON_CONFIG, path)
    }
}

pub struct JavaScriptDetector;

impl LanguageDetector for JavaScriptDetector {
    fn detect(&self, path: &Path) -> Option<LanguageDetection> {
        detect_language(&JAVASCRIPT_CONFIG, path)
    }
}
