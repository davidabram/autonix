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
    DenoLock,
    LockJson,
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

impl From<&LanguageDetectionSource> for Language {
    fn from(source: &LanguageDetectionSource) -> Self {
        match source {
            // Go
            LanguageDetectionSource::GoMod
            | LanguageDetectionSource::GoWork
            | LanguageDetectionSource::GoSum
            | LanguageDetectionSource::GoVersionFile
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
            | LanguageDetectionSource::PythonVersionFile
            | LanguageDetectionSource::PyFile => Language::Python,

            // JavaScript/Node
            LanguageDetectionSource::PackageJson
            | LanguageDetectionSource::PackageLockJson
            | LanguageDetectionSource::YarnLock
            | LanguageDetectionSource::PnpmLockYaml
            | LanguageDetectionSource::BunLock
            | LanguageDetectionSource::BunLockb
            | LanguageDetectionSource::DenoLock
            | LanguageDetectionSource::LockJson
            | LanguageDetectionSource::DenoJson
            | LanguageDetectionSource::DenoJsonc
            | LanguageDetectionSource::TsConfig
            | LanguageDetectionSource::JsConfig
            | LanguageDetectionSource::NvmrcFile
            | LanguageDetectionSource::NodeVersionFile
            | LanguageDetectionSource::BunVersionFile
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
    pub language: Language,
    pub sources: Vec<LanguageDetectionSignal>,
}

impl LanguageDetection {
    pub fn new(language: Language, mut sources: Vec<LanguageDetectionSignal>) -> Self {
        use std::collections::HashSet;
        let mut seen_weak: HashSet<String> = HashSet::new();
        sources.retain(|signal| match signal {
            LanguageDetectionSignal::Strong { .. } => true,
            LanguageDetectionSignal::Weak(source) => {
                let source_str = format!("{:?}", source);
                seen_weak.insert(source_str)
            }
        });

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

        let source = match filename {
            // Go
            "go.mod" => Ok(LanguageDetectionSource::GoMod),
            "go.work" => Ok(LanguageDetectionSource::GoWork),
            "go.sum" => Ok(LanguageDetectionSource::GoSum),
            ".go-version" => Ok(LanguageDetectionSource::GoVersionFile),

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
            ".python-version" => Ok(LanguageDetectionSource::PythonVersionFile),

            // JavaScript/Node
            "package.json" => Ok(LanguageDetectionSource::PackageJson),
            "package-lock.json" => Ok(LanguageDetectionSource::PackageLockJson),
            "yarn.lock" => Ok(LanguageDetectionSource::YarnLock),
            "pnpm-lock.yaml" => Ok(LanguageDetectionSource::PnpmLockYaml),
            "bun.lock" => Ok(LanguageDetectionSource::BunLock),
            "bun.lockb" => Ok(LanguageDetectionSource::BunLockb),
            "deno.lock" => Ok(LanguageDetectionSource::DenoLock),
            "lock.json" => Ok(LanguageDetectionSource::LockJson),
            "deno.json" => Ok(LanguageDetectionSource::DenoJson),
            "deno.jsonc" => Ok(LanguageDetectionSource::DenoJsonc),
            "tsconfig.json" => Ok(LanguageDetectionSource::TsConfig),
            "jsconfig.json" => Ok(LanguageDetectionSource::JsConfig),
            ".nvmrc" => Ok(LanguageDetectionSource::NvmrcFile),
            ".node-version" => Ok(LanguageDetectionSource::NodeVersionFile),
            ".bun-version" => Ok(LanguageDetectionSource::BunVersionFile),

            _ => Err(()),
        };

        if let Ok(source) = source {
            return Ok(LanguageDetectionSignal::Strong { path, source });
        }

        let extension = path.extension().and_then(|e| e.to_str());
        match extension {
            Some("go") => Ok(LanguageDetectionSource::GoFile),
            Some("rs") => Ok(LanguageDetectionSource::RsFile),
            Some("py") => Ok(LanguageDetectionSource::PyFile),
            Some("js") => Ok(LanguageDetectionSource::JsFile),
            Some("mjs") => Ok(LanguageDetectionSource::MjsFile),
            Some("cjs") => Ok(LanguageDetectionSource::CjsFile),
            Some("ts") => Ok(LanguageDetectionSource::TsFile),
            Some("jsx") => Ok(LanguageDetectionSource::JsxFile),
            Some("tsx") => Ok(LanguageDetectionSource::TsxFile),
            _ => Err(()),
        }
        .map(LanguageDetectionSignal::Weak)
    }
}
