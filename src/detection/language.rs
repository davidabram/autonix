pub use std::path::Path;
use std::path::PathBuf;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub enum Language {
    Go,
    Rust,
    Python,
    JavaScript,
}

#[derive(Debug, Serialize)]
pub enum LanguageDetectionSource {
    //Go
    GoMod,
    GoWork,
    GoSum,
    GoVersion,
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

pub struct GoDetector;

impl LanguageDetector for GoDetector {
    fn detect(&self, path: &Path) -> Option<LanguageDetection> {
        let mut detected_from = Vec::new();
        let mut has_go_file = false;

        for entry in walkdir::WalkDir::new(path).into_iter().filter_map(Result::ok) {
            let entry_path = entry.path();

            if entry.file_type().is_file() && let Some(filename) = entry_path.file_name().and_then(|n| n.to_str()) {
                match filename {
                    "go.mod" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::GoMod,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "go.work" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::GoWork,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "go.sum" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::GoSum,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    ".go-version" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::GoVersion,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    _ => {
                        if !has_go_file && entry_path.extension().and_then(|ext| ext.to_str()) == Some("go") {
                            has_go_file = true;
                            detected_from.push(DetectedSource {
                                source: LanguageDetectionSource::GoFile,
                                path: None,
                            });
                        }
                    }
                }
            }
        }

        if detected_from.is_empty() {
            None
        } else {
            Some(LanguageDetection {
                language: Language::Go,
                detected_from,
            })
        }
    }
}

pub struct RustDetector;

impl LanguageDetector for RustDetector {
    fn detect(&self, path: &Path) -> Option<LanguageDetection> {
        let mut detected_from = Vec::new();
        let mut has_rs_file = false;

        for entry in walkdir::WalkDir::new(path).into_iter().filter_map(Result::ok) {
            let entry_path = entry.path();

            if entry.file_type().is_file() && let Some(filename) = entry_path.file_name().and_then(|n| n.to_str()) {
                match filename {
                    "Cargo.toml" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::CargoToml,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "Cargo.lock" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::CargoLock,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "rust-toolchain" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::RustToolchain,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "rust-toolchain.toml" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::RustToolchainToml,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    _ => {
                        if !has_rs_file && entry_path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                            has_rs_file = true;
                            detected_from.push(DetectedSource {
                                source: LanguageDetectionSource::RsFile,
                                path: None,
                            });
                        }
                    }
                }
            }
        }

        if detected_from.is_empty() {
            None
        } else {
            Some(LanguageDetection {
                language: Language::Rust,
                detected_from,
            })
        }
    }
}

pub struct PythonDetector;

impl LanguageDetector for PythonDetector {
    fn detect(&self, path: &Path) -> Option<LanguageDetection> {
        let mut detected_from = Vec::new();
        let mut has_py_file = false;

        for entry in walkdir::WalkDir::new(path).into_iter().filter_map(Result::ok) {
            let entry_path = entry.path();

            if entry.file_type().is_file() && let Some(filename) = entry_path.file_name().and_then(|n| n.to_str()) {
                match filename {
                    "requirements.txt" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::RequirementsTxt,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "pyproject.toml" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::PyprojectToml,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "Pipfile" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::Pipfile,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "Pipfile.lock" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::PipfileLock,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "poetry.lock" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::PoetryLock,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "setup.py" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::SetupPy,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "setup.cfg" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::SetupCfg,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "environment.yml" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::EnvironmentYml,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    _ => {
                        if !has_py_file && entry_path.extension().and_then(|ext| ext.to_str()) == Some("py") {
                            has_py_file = true;
                            detected_from.push(DetectedSource {
                                source: LanguageDetectionSource::PyFile,
                                path: None,
                            });
                        }
                    }
                }
            }
        }

        if detected_from.is_empty() {
            None
        } else {
            Some(LanguageDetection {
                language: Language::Python,
                detected_from,
            })
        }
    }
}

pub struct JavaScriptDetector;

impl LanguageDetector for JavaScriptDetector {
    fn detect(&self, path: &Path) -> Option<LanguageDetection> {
        let mut detected_from = Vec::new();
        let mut has_js_file = false;
        let mut has_mjs_file = false;
        let mut has_cjs_file = false;
        let mut has_ts_file = false;
        let mut has_jsx_file = false;
        let mut has_tsx_file = false;

        for entry in walkdir::WalkDir::new(path).into_iter().filter_map(Result::ok) {
            let entry_path = entry.path();

            if entry.file_type().is_file() && let Some(filename) = entry_path.file_name().and_then(|n| n.to_str()) {
                match filename {
                    "package.json" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::PackageJson,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "package-lock.json" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::PackageLockJson,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "yarn.lock" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::YarnLock,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "pnpm-lock.yaml" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::PnpmLockYaml,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "bun.lock" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::BunLock,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "bun.lockb" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::BunLockb,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "lock.json" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::LockJson,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "deno.lock" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::DenoLock,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "deno.json" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::DenoJson,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "deno.jsonc" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::DenoJsonc,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "tsconfig.json" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::TsConfig,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    "jsconfig.json" => {
                        detected_from.push(DetectedSource {
                            source: LanguageDetectionSource::JsConfig,
                            path: Some(entry_path.to_path_buf()),
                        });
                    }
                    _ => {
                        match entry_path.extension().and_then(|ext| ext.to_str()) {
                            Some("js") if !has_js_file => {
                                has_js_file = true;
                                detected_from.push(DetectedSource {
                                    source: LanguageDetectionSource::JsFile,
                                    path: None,
                                });
                            }
                            Some("mjs") if !has_mjs_file => {
                                has_mjs_file = true;
                                detected_from.push(DetectedSource {
                                    source: LanguageDetectionSource::MjsFile,
                                    path: None,
                                });
                            }
                            Some("cjs") if !has_cjs_file => {
                                has_cjs_file = true;
                                detected_from.push(DetectedSource {
                                    source: LanguageDetectionSource::CjsFile,
                                    path: None,
                                });
                            }
                            Some("ts") if !has_ts_file => {
                                has_ts_file = true;
                                detected_from.push(DetectedSource {
                                    source: LanguageDetectionSource::TsFile,
                                    path: None,
                                });
                            }
                            Some("jsx") if !has_jsx_file => {
                                has_jsx_file = true;
                                detected_from.push(DetectedSource {
                                    source: LanguageDetectionSource::JsxFile,
                                    path: None,
                                });
                            }
                            Some("tsx") if !has_tsx_file => {
                                has_tsx_file = true;
                                detected_from.push(DetectedSource {
                                    source: LanguageDetectionSource::TsxFile,
                                    path: None,
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if detected_from.is_empty() {
            None
        } else {
            Some(LanguageDetection {
                language: Language::JavaScript,
                detected_from,
            })
        }
    }
}
