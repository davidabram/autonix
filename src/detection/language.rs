pub use std::path::Path;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub enum Language {
    Php,
    Go,
    Java,
    Rust,
    Ruby,
    Elixir,
    Python,
    DotNet,
    JavaScript,
    Gleam,
    Cpp,
    Staticfile,
    Shell,
}

#[derive(Debug, Serialize)]
pub enum LanguageDetectionSource {
    //Go
    GoMod,
    GoWork,
    GoFile,

    //Rust
    CargoToml,
    RsFile,

    //Python,
    RequirementsTxt,
    PyprojectToml,
    Pipfile,
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
    JsFile,
    MjsFile,
    CjsFile,
    TsFile,
    JsxFile,
    TsxFile,
}

#[derive(Debug, Serialize)]
pub struct LanguageDetection {
    pub language: Language,
    pub detected_from: Vec<LanguageDetectionSource>,
}

pub trait LanguageDetector {
    fn detect(&self, path: &Path) -> Option<LanguageDetection>;
}

pub struct GoDetector;

impl LanguageDetector for GoDetector {
    fn detect(&self, path: &Path) -> Option<LanguageDetection> {
        let mut detected_from = Vec::new();

        if path.join("go.mod").exists() {
            detected_from.push(LanguageDetectionSource::GoMod);
        }

        if path.join("go.work").exists() {
            detected_from.push(LanguageDetectionSource::GoWork);
        }

        if walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(Result::ok)
            .any(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("go"))
        {
            detected_from.push(LanguageDetectionSource::GoFile);
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

        if path.join("Cargo.toml").exists() {
            detected_from.push(LanguageDetectionSource::CargoToml);
        }

        if walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(Result::ok)
            .any(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("rs"))
        {
            detected_from.push(LanguageDetectionSource::RsFile);
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

        if path.join("requirements.txt").exists() {
            detected_from.push(LanguageDetectionSource::RequirementsTxt);
        }

        if path.join("pyproject.toml").exists() {
            detected_from.push(LanguageDetectionSource::PyprojectToml);
        }

        if path.join("Pipfile").exists() {
            detected_from.push(LanguageDetectionSource::Pipfile);
        }

        if walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(Result::ok)
            .any(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("py"))
        {
            detected_from.push(LanguageDetectionSource::PyFile);
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

        if path.join("package.json").exists() {
            detected_from.push(LanguageDetectionSource::PackageJson);
        }

        if path.join("package-lock.json").exists() {
            detected_from.push(LanguageDetectionSource::PackageLockJson);
        }

        if path.join("yarn.lock").exists() {
            detected_from.push(LanguageDetectionSource::YarnLock);
        }

        if path.join("pnpm-lock.yaml").exists() {
            detected_from.push(LanguageDetectionSource::PnpmLockYaml);
        }

        if path.join("bun.lock").exists() {
            detected_from.push(LanguageDetectionSource::BunLock);
        }

        if path.join("bun.lockb").exists() {
            detected_from.push(LanguageDetectionSource::BunLockb);
        }

        if path.join("lock.json").exists() {
            detected_from.push(LanguageDetectionSource::LockJson);
        }

        if path.join("deno.lock").exists() {
            detected_from.push(LanguageDetectionSource::DenoLock);
        }

        let mut has_js = false;
        let mut has_mjs = false;
        let mut has_cjs = false;
        let mut has_ts = false;
        let mut has_jsx = false;
        let mut has_tsx = false;

        for entry in walkdir::WalkDir::new(path).into_iter().filter_map(Result::ok) {
            if let Some(ext) = entry.path().extension().and_then(|ext| ext.to_str()) {
                match ext {
                    "js" => has_js = true,
                    "mjs" => has_mjs = true,
                    "cjs" => has_cjs = true,
                    "ts" => has_ts = true,
                    "jsx" => has_jsx = true,
                    "tsx" => has_tsx = true,
                    _ => {}
                }
            }
        }

        if has_js {
            detected_from.push(LanguageDetectionSource::JsFile);
        }
        if has_mjs {
            detected_from.push(LanguageDetectionSource::MjsFile);
        }
        if has_cjs {
            detected_from.push(LanguageDetectionSource::CjsFile);
        }
        if has_ts {
            detected_from.push(LanguageDetectionSource::TsFile);
        }
        if has_jsx {
            detected_from.push(LanguageDetectionSource::JsxFile);
        }
        if has_tsx {
            detected_from.push(LanguageDetectionSource::TsxFile);
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
