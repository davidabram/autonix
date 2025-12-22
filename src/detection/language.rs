use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
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

impl From<&LanguageDetectionSource> for Language {
    fn from(source: &LanguageDetectionSource) -> Self {
        match source {
            // Go
            LanguageDetectionSource::GoMod
            | LanguageDetectionSource::GoWork
            | LanguageDetectionSource::GoSum
            | LanguageDetectionSource::GoVersion
            | LanguageDetectionSource::GoFile => Language::Go,

            // Rust
            LanguageDetectionSource::CargoToml
            | LanguageDetectionSource::CargoLock
            | LanguageDetectionSource::RustToolchain
            | LanguageDetectionSource::RustToolchainToml
            | LanguageDetectionSource::RsFile => Language::Rust,

            // Python
            LanguageDetectionSource::RequirementsTxt
            | LanguageDetectionSource::PyprojectToml
            | LanguageDetectionSource::Pipfile
            | LanguageDetectionSource::PipfileLock
            | LanguageDetectionSource::PoetryLock
            | LanguageDetectionSource::SetupPy
            | LanguageDetectionSource::SetupCfg
            | LanguageDetectionSource::EnvironmentYml
            | LanguageDetectionSource::PyFile => Language::Python,

            // JavaScript/Node
            LanguageDetectionSource::PackageJson
            | LanguageDetectionSource::PackageLockJson
            | LanguageDetectionSource::YarnLock
            | LanguageDetectionSource::PnpmLockYaml
            | LanguageDetectionSource::BunLock
            | LanguageDetectionSource::BunLockb
            | LanguageDetectionSource::DenoLock
            | LanguageDetectionSource::DenoJson
            | LanguageDetectionSource::DenoJsonc
            | LanguageDetectionSource::TsConfig
            | LanguageDetectionSource::JsConfig
            | LanguageDetectionSource::JsFile
            | LanguageDetectionSource::MjsFile
            | LanguageDetectionSource::CjsFile
            | LanguageDetectionSource::TsFile
            | LanguageDetectionSource::JsxFile
            | LanguageDetectionSource::TsxFile => Language::JavaScript,
        }
    }
}

impl From<&LanguageDetectionSignal> for Language {
    fn from(signal: &LanguageDetectionSignal) -> Self {
        match signal {
            LanguageDetectionSignal::Strong { source, .. } => source.into(),
            LanguageDetectionSignal::Weak(source) => source.into(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LanguageDetection {
    language: Language,
    sources: Vec<LanguageDetectionSignal>,
}

impl LanguageDetection {
    pub fn new(language: Language, sources: Vec<LanguageDetectionSignal>) -> Self {
        Self { language, sources }
    }
}

#[derive(Debug, Serialize)]
pub enum LanguageDetectionSignal {
    Strong {
        path: PathBuf,
        source: LanguageDetectionSource,
    },
    Weak(LanguageDetectionSource),
}

impl TryFrom<PathBuf> for LanguageDetectionSignal {
    type Error = ();

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let filename = path.file_name().ok_or(())?.to_str().ok_or(())?;
        let extension = path.extension().ok_or(())?.to_str().ok_or(())?;

        let source = match filename {
            // Go
            "go.mod" => Ok(LanguageDetectionSource::GoMod),
            "go.work" => Ok(LanguageDetectionSource::GoWork),
            "go.sum" => Ok(LanguageDetectionSource::GoSum),
            ".go-version" => Ok(LanguageDetectionSource::GoVersion),

            // Rust
            "Cargo.toml" => Ok(LanguageDetectionSource::CargoToml),
            "Cargo.lock" => Ok(LanguageDetectionSource::CargoLock),
            "rust-toolchain" => Ok(LanguageDetectionSource::RustToolchain),
            "rust-toolchain.toml" => Ok(LanguageDetectionSource::RustToolchainToml),

            // Python
            "requirements.txt" => Ok(LanguageDetectionSource::RequirementsTxt),
            "pyproject.toml" => Ok(LanguageDetectionSource::PyprojectToml),
            "Pipfile" => Ok(LanguageDetectionSource::Pipfile),
            "Pipfile.lock" => Ok(LanguageDetectionSource::PipfileLock),
            "poetry.lock" => Ok(LanguageDetectionSource::PoetryLock),
            "setup.py" => Ok(LanguageDetectionSource::SetupPy),
            "setup.cfg" => Ok(LanguageDetectionSource::SetupCfg),
            "environment.yml" => Ok(LanguageDetectionSource::EnvironmentYml),

            // JavaScript/Node
            "package.json" => Ok(LanguageDetectionSource::PackageJson),
            "package-lock.json" => Ok(LanguageDetectionSource::PackageLockJson),
            "yarn.lock" => Ok(LanguageDetectionSource::YarnLock),
            "pnpm-lock.yaml" => Ok(LanguageDetectionSource::PnpmLockYaml),
            "bun.lock" => Ok(LanguageDetectionSource::BunLock),
            "bun.lockb" => Ok(LanguageDetectionSource::BunLockb),
            "deno.lock" => Ok(LanguageDetectionSource::DenoLock),
            "deno.json" => Ok(LanguageDetectionSource::DenoJson),
            "deno.jsonc" => Ok(LanguageDetectionSource::DenoJsonc),
            "tsconfig.json" => Ok(LanguageDetectionSource::TsConfig),
            "jsconfig.json" => Ok(LanguageDetectionSource::JsConfig),

            _ => Err(()),
        };

        if let Ok(source) = source {
            return Ok(LanguageDetectionSignal::Strong { path, source });
        }

        match extension {
            //Extensions
            "go" => Ok(LanguageDetectionSource::GoFile),
            "rs" => Ok(LanguageDetectionSource::RsFile),
            "py" => Ok(LanguageDetectionSource::PyFile),
            "js" => Ok(LanguageDetectionSource::JsFile),
            "mjs" => Ok(LanguageDetectionSource::MjsFile),
            "cjs" => Ok(LanguageDetectionSource::CjsFile),
            "ts" => Ok(LanguageDetectionSource::TsFile),
            "jsx" => Ok(LanguageDetectionSource::JsxFile),
            "tsx" => Ok(LanguageDetectionSource::TsxFile),
            _ => Err(()),
        }
        .map(LanguageDetectionSignal::Weak)
    }
}
