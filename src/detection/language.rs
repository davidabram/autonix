use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
pub enum Language {
    Go,
    Rust,
    Python,
    JavaScript,
}

impl Language {
    pub fn dir_name(&self) -> &'static str {
        match self {
            Language::Go => "golang",
            Language::Python => "python",
            Language::JavaScript => "nodejs",
            Language::Rust => "rust",
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_go_sources() {
        assert_eq!(
            Language::from(&LanguageDetectionSource::GoMod),
            Language::Go
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::GoWork),
            Language::Go
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::GoSum),
            Language::Go
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::GoVersionFile),
            Language::Go
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::GoFile),
            Language::Go
        );
    }

    #[test]
    fn test_language_from_rust_sources() {
        assert_eq!(
            Language::from(&LanguageDetectionSource::CargoToml),
            Language::Rust
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::CargoLock),
            Language::Rust
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::RustToolchain),
            Language::Rust
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::RustToolchainToml),
            Language::Rust
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::RsFile),
            Language::Rust
        );
    }

    #[test]
    fn test_language_from_python_sources() {
        assert_eq!(
            Language::from(&LanguageDetectionSource::RequirementsTxt),
            Language::Python
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::PyprojectToml),
            Language::Python
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::Pipfile),
            Language::Python
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::PipfileLock),
            Language::Python
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::PoetryLock),
            Language::Python
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::SetupPy),
            Language::Python
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::SetupCfg),
            Language::Python
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::EnvironmentYml),
            Language::Python
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::PythonVersionFile),
            Language::Python
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::PyFile),
            Language::Python
        );
    }

    #[test]
    fn test_language_from_javascript_sources() {
        assert_eq!(
            Language::from(&LanguageDetectionSource::PackageJson),
            Language::JavaScript
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::PackageLockJson),
            Language::JavaScript
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::YarnLock),
            Language::JavaScript
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::PnpmLockYaml),
            Language::JavaScript
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::BunLock),
            Language::JavaScript
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::BunLockb),
            Language::JavaScript
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::DenoLock),
            Language::JavaScript
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::LockJson),
            Language::JavaScript
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::DenoJson),
            Language::JavaScript
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::DenoJsonc),
            Language::JavaScript
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::TsConfig),
            Language::JavaScript
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::JsConfig),
            Language::JavaScript
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::NvmrcFile),
            Language::JavaScript
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::NodeVersionFile),
            Language::JavaScript
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::BunVersionFile),
            Language::JavaScript
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::JsFile),
            Language::JavaScript
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::MjsFile),
            Language::JavaScript
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::CjsFile),
            Language::JavaScript
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::TsFile),
            Language::JavaScript
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::JsxFile),
            Language::JavaScript
        );
        assert_eq!(
            Language::from(&LanguageDetectionSource::TsxFile),
            Language::JavaScript
        );
    }

    #[test]
    fn test_language_from_strong_signal() {
        let signal = LanguageDetectionSignal::Strong {
            path: PathBuf::from("go.mod"),
            source: LanguageDetectionSource::GoMod,
        };
        assert_eq!(Language::from(&signal), Language::Go);
    }

    #[test]
    fn test_language_from_weak_signal() {
        let signal = LanguageDetectionSignal::Weak(LanguageDetectionSource::PyFile);
        assert_eq!(Language::from(&signal), Language::Python);
    }

    #[test]
    fn test_try_from_pathbuf_go_files() {
        let path = PathBuf::from("go.mod");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Strong { source, .. } => {
                assert!(matches!(source, LanguageDetectionSource::GoMod));
            }
            _ => panic!("Expected Strong signal"),
        }

        let path = PathBuf::from("go.work");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Strong { source, .. } => {
                assert!(matches!(source, LanguageDetectionSource::GoWork));
            }
            _ => panic!("Expected Strong signal"),
        }

        let path = PathBuf::from("go.sum");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Strong { source, .. } => {
                assert!(matches!(source, LanguageDetectionSource::GoSum));
            }
            _ => panic!("Expected Strong signal"),
        }

        let path = PathBuf::from(".go-version");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Strong { source, .. } => {
                assert!(matches!(source, LanguageDetectionSource::GoVersionFile));
            }
            _ => panic!("Expected Strong signal"),
        }
    }

    #[test]
    fn test_try_from_pathbuf_rust_files() {
        let path = PathBuf::from("Cargo.toml");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Strong { source, .. } => {
                assert!(matches!(source, LanguageDetectionSource::CargoToml));
            }
            _ => panic!("Expected Strong signal"),
        }

        let path = PathBuf::from("Cargo.lock");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Strong { source, .. } => {
                assert!(matches!(source, LanguageDetectionSource::CargoLock));
            }
            _ => panic!("Expected Strong signal"),
        }

        let path = PathBuf::from("rust-toolchain");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Strong { source, .. } => {
                assert!(matches!(source, LanguageDetectionSource::RustToolchain));
            }
            _ => panic!("Expected Strong signal"),
        }

        let path = PathBuf::from("rust-toolchain.toml");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Strong { source, .. } => {
                assert!(matches!(source, LanguageDetectionSource::RustToolchainToml));
            }
            _ => panic!("Expected Strong signal"),
        }
    }

    #[test]
    fn test_try_from_pathbuf_python_files() {
        let path = PathBuf::from("requirements.txt");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Strong { source, .. } => {
                assert!(matches!(source, LanguageDetectionSource::RequirementsTxt));
            }
            _ => panic!("Expected Strong signal"),
        }

        let path = PathBuf::from("pyproject.toml");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Strong { source, .. } => {
                assert!(matches!(source, LanguageDetectionSource::PyprojectToml));
            }
            _ => panic!("Expected Strong signal"),
        }

        let path = PathBuf::from("Pipfile");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Strong { source, .. } => {
                assert!(matches!(source, LanguageDetectionSource::Pipfile));
            }
            _ => panic!("Expected Strong signal"),
        }

        let path = PathBuf::from(".python-version");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Strong { source, .. } => {
                assert!(matches!(source, LanguageDetectionSource::PythonVersionFile));
            }
            _ => panic!("Expected Strong signal"),
        }
    }

    #[test]
    fn test_try_from_pathbuf_javascript_files() {
        let path = PathBuf::from("package.json");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Strong { source, .. } => {
                assert!(matches!(source, LanguageDetectionSource::PackageJson));
            }
            _ => panic!("Expected Strong signal"),
        }

        let path = PathBuf::from("package-lock.json");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Strong { source, .. } => {
                assert!(matches!(source, LanguageDetectionSource::PackageLockJson));
            }
            _ => panic!("Expected Strong signal"),
        }

        let path = PathBuf::from("yarn.lock");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Strong { source, .. } => {
                assert!(matches!(source, LanguageDetectionSource::YarnLock));
            }
            _ => panic!("Expected Strong signal"),
        }

        let path = PathBuf::from("tsconfig.json");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Strong { source, .. } => {
                assert!(matches!(source, LanguageDetectionSource::TsConfig));
            }
            _ => panic!("Expected Strong signal"),
        }

        let path = PathBuf::from(".nvmrc");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Strong { source, .. } => {
                assert!(matches!(source, LanguageDetectionSource::NvmrcFile));
            }
            _ => panic!("Expected Strong signal"),
        }
    }

    #[test]
    fn test_try_from_pathbuf_weak_signals() {
        let path = PathBuf::from("main.go");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Weak(source) => {
                assert!(matches!(source, LanguageDetectionSource::GoFile));
            }
            _ => panic!("Expected Weak signal"),
        }

        let path = PathBuf::from("lib.rs");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Weak(source) => {
                assert!(matches!(source, LanguageDetectionSource::RsFile));
            }
            _ => panic!("Expected Weak signal"),
        }

        let path = PathBuf::from("app.py");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Weak(source) => {
                assert!(matches!(source, LanguageDetectionSource::PyFile));
            }
            _ => panic!("Expected Weak signal"),
        }

        let path = PathBuf::from("index.js");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Weak(source) => {
                assert!(matches!(source, LanguageDetectionSource::JsFile));
            }
            _ => panic!("Expected Weak signal"),
        }

        let path = PathBuf::from("module.mjs");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Weak(source) => {
                assert!(matches!(source, LanguageDetectionSource::MjsFile));
            }
            _ => panic!("Expected Weak signal"),
        }

        let path = PathBuf::from("common.cjs");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Weak(source) => {
                assert!(matches!(source, LanguageDetectionSource::CjsFile));
            }
            _ => panic!("Expected Weak signal"),
        }

        let path = PathBuf::from("types.ts");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Weak(source) => {
                assert!(matches!(source, LanguageDetectionSource::TsFile));
            }
            _ => panic!("Expected Weak signal"),
        }

        let path = PathBuf::from("Component.jsx");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Weak(source) => {
                assert!(matches!(source, LanguageDetectionSource::JsxFile));
            }
            _ => panic!("Expected Weak signal"),
        }

        let path = PathBuf::from("App.tsx");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Weak(source) => {
                assert!(matches!(source, LanguageDetectionSource::TsxFile));
            }
            _ => panic!("Expected Weak signal"),
        }
    }

    #[test]
    fn test_try_from_pathbuf_invalid_files() {
        let path = PathBuf::from("README.md");
        assert!(LanguageDetectionSignal::try_from(path).is_err());

        let path = PathBuf::from("unknown.txt");
        assert!(LanguageDetectionSignal::try_from(path).is_err());

        let path = PathBuf::from(".gitignore");
        assert!(LanguageDetectionSignal::try_from(path).is_err());
    }

    #[test]
    fn test_try_from_pathbuf_with_directory_path() {
        let path = PathBuf::from("src/main.rs");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Weak(source) => {
                assert!(matches!(source, LanguageDetectionSource::RsFile));
            }
            _ => panic!("Expected Weak signal"),
        }

        let path = PathBuf::from("/home/user/project/package.json");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Strong { source, .. } => {
                assert!(matches!(source, LanguageDetectionSource::PackageJson));
            }
            _ => panic!("Expected Strong signal"),
        }
    }

    #[test]
    fn test_language_detection_new() {
        let sources = vec![
            LanguageDetectionSignal::Strong {
                path: PathBuf::from("go.mod"),
                source: LanguageDetectionSource::GoMod,
            },
            LanguageDetectionSignal::Weak(LanguageDetectionSource::GoFile),
        ];

        let detection = LanguageDetection::new(Language::Go, sources);
        assert_eq!(detection.language, Language::Go);
        assert_eq!(detection.sources.len(), 2);
    }

    #[test]
    fn test_language_detection_deduplicates_weak_signals() {
        let sources = vec![
            LanguageDetectionSignal::Weak(LanguageDetectionSource::PyFile),
            LanguageDetectionSignal::Weak(LanguageDetectionSource::PyFile),
            LanguageDetectionSignal::Weak(LanguageDetectionSource::PyFile),
        ];

        let detection = LanguageDetection::new(Language::Python, sources);
        assert_eq!(detection.language, Language::Python);
        assert_eq!(detection.sources.len(), 1);
    }

    #[test]
    fn test_language_detection_preserves_strong_signals() {
        let sources = vec![
            LanguageDetectionSignal::Strong {
                path: PathBuf::from("requirements.txt"),
                source: LanguageDetectionSource::RequirementsTxt,
            },
            LanguageDetectionSignal::Strong {
                path: PathBuf::from("setup.py"),
                source: LanguageDetectionSource::SetupPy,
            },
            LanguageDetectionSignal::Strong {
                path: PathBuf::from("pyproject.toml"),
                source: LanguageDetectionSource::PyprojectToml,
            },
        ];

        let detection = LanguageDetection::new(Language::Python, sources);
        assert_eq!(detection.language, Language::Python);
        assert_eq!(detection.sources.len(), 3);
    }

    #[test]
    fn test_language_detection_mixed_signals() {
        let sources = vec![
            LanguageDetectionSignal::Strong {
                path: PathBuf::from("package.json"),
                source: LanguageDetectionSource::PackageJson,
            },
            LanguageDetectionSignal::Weak(LanguageDetectionSource::JsFile),
            LanguageDetectionSignal::Weak(LanguageDetectionSource::JsFile),
            LanguageDetectionSignal::Weak(LanguageDetectionSource::TsFile),
            LanguageDetectionSignal::Strong {
                path: PathBuf::from("tsconfig.json"),
                source: LanguageDetectionSource::TsConfig,
            },
        ];

        let detection = LanguageDetection::new(Language::JavaScript, sources);
        assert_eq!(detection.language, Language::JavaScript);
        assert_eq!(detection.sources.len(), 4);
    }

    #[test]
    fn test_try_from_pathbuf_python_additional_files() {
        let path = PathBuf::from("Pipfile.lock");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        assert!(matches!(
            signal,
            LanguageDetectionSignal::Strong {
                source: LanguageDetectionSource::PipfileLock,
                ..
            }
        ));

        let path = PathBuf::from("poetry.lock");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        assert!(matches!(
            signal,
            LanguageDetectionSignal::Strong {
                source: LanguageDetectionSource::PoetryLock,
                ..
            }
        ));

        let path = PathBuf::from("setup.cfg");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        assert!(matches!(
            signal,
            LanguageDetectionSignal::Strong {
                source: LanguageDetectionSource::SetupCfg,
                ..
            }
        ));

        let path = PathBuf::from("environment.yml");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        assert!(matches!(
            signal,
            LanguageDetectionSignal::Strong {
                source: LanguageDetectionSource::EnvironmentYml,
                ..
            }
        ));
    }

    #[test]
    fn test_try_from_pathbuf_javascript_additional_files() {
        let path = PathBuf::from("bun.lock");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        assert!(matches!(
            signal,
            LanguageDetectionSignal::Strong {
                source: LanguageDetectionSource::BunLock,
                ..
            }
        ));

        let path = PathBuf::from("bun.lockb");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        assert!(matches!(
            signal,
            LanguageDetectionSignal::Strong {
                source: LanguageDetectionSource::BunLockb,
                ..
            }
        ));

        let path = PathBuf::from("deno.lock");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        assert!(matches!(
            signal,
            LanguageDetectionSignal::Strong {
                source: LanguageDetectionSource::DenoLock,
                ..
            }
        ));

        let path = PathBuf::from("lock.json");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        assert!(matches!(
            signal,
            LanguageDetectionSignal::Strong {
                source: LanguageDetectionSource::LockJson,
                ..
            }
        ));

        let path = PathBuf::from("deno.jsonc");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        assert!(matches!(
            signal,
            LanguageDetectionSignal::Strong {
                source: LanguageDetectionSource::DenoJsonc,
                ..
            }
        ));

        let path = PathBuf::from("jsconfig.json");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        assert!(matches!(
            signal,
            LanguageDetectionSignal::Strong {
                source: LanguageDetectionSource::JsConfig,
                ..
            }
        ));

        let path = PathBuf::from(".node-version");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        assert!(matches!(
            signal,
            LanguageDetectionSignal::Strong {
                source: LanguageDetectionSource::NodeVersionFile,
                ..
            }
        ));

        let path = PathBuf::from(".bun-version");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        assert!(matches!(
            signal,
            LanguageDetectionSignal::Strong {
                source: LanguageDetectionSource::BunVersionFile,
                ..
            }
        ));

        let path = PathBuf::from("pnpm-lock.yaml");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        assert!(matches!(
            signal,
            LanguageDetectionSignal::Strong {
                source: LanguageDetectionSource::PnpmLockYaml,
                ..
            }
        ));
    }

    #[test]
    fn test_try_from_pathbuf_go_additional_files() {
        let path = PathBuf::from("go.work");
        let signal = LanguageDetectionSignal::try_from(path).unwrap();
        match signal {
            LanguageDetectionSignal::Strong { source, .. } => {
                assert!(matches!(source, LanguageDetectionSource::GoWork));
            }
            _ => panic!("Expected Strong signal"),
        }
    }

    #[test]
    fn test_try_from_pathbuf_edge_cases() {
        let path = PathBuf::from("Makefile");
        assert!(LanguageDetectionSignal::try_from(path).is_err());

        let path = PathBuf::from(".gitignore");
        assert!(LanguageDetectionSignal::try_from(path).is_err());

        let path = PathBuf::from("script.PY");
        assert!(LanguageDetectionSignal::try_from(path).is_err());
    }

    #[test]
    fn test_try_from_pathbuf_path_without_filename() {
        let path = PathBuf::from("/");
        assert!(LanguageDetectionSignal::try_from(path).is_err());
    }

    #[test]
    fn test_language_detection_empty_sources() {
        let detection = LanguageDetection::new(Language::Go, vec![]);
        assert_eq!(detection.language, Language::Go);
        assert_eq!(detection.sources.len(), 0);
    }

    #[test]
    fn test_language_detection_only_weak_signals() {
        let sources = vec![
            LanguageDetectionSignal::Weak(LanguageDetectionSource::PyFile),
            LanguageDetectionSignal::Weak(LanguageDetectionSource::PyFile),
            LanguageDetectionSignal::Weak(LanguageDetectionSource::PyFile),
            LanguageDetectionSignal::Weak(LanguageDetectionSource::PyFile),
        ];

        let detection = LanguageDetection::new(Language::Python, sources);
        assert_eq!(detection.sources.len(), 1);
    }

    #[test]
    fn test_language_detection_multiple_different_weak_signals() {
        let sources = vec![
            LanguageDetectionSignal::Weak(LanguageDetectionSource::JsFile),
            LanguageDetectionSignal::Weak(LanguageDetectionSource::TsFile),
            LanguageDetectionSignal::Weak(LanguageDetectionSource::JsxFile),
            LanguageDetectionSignal::Weak(LanguageDetectionSource::TsxFile),
            LanguageDetectionSignal::Weak(LanguageDetectionSource::MjsFile),
            LanguageDetectionSignal::Weak(LanguageDetectionSource::CjsFile),
        ];

        let detection = LanguageDetection::new(Language::JavaScript, sources);
        assert_eq!(detection.sources.len(), 6);
    }
}
