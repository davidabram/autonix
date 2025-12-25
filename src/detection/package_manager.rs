use super::{Language, LanguageDetection, LanguageDetectionSignal, LanguageDetectionSource};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum PackageManager {
    // JavaScript
    Npm,
    Pnpm,
    Yarn,
    Bun,
    Deno,
    // Python
    Pip,
    Uv,
    Poetry,
    Pdm,
    Pipenv,
    // Rust
    Cargo,
    // Golang
    Go,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum PackageManagerSource {
    // JavaScript
    PackageJson,
    PackageLockJson,
    YarnLock,
    PnpmLockYaml,
    BunLockb,
    BunLock,
    DenoJson,
    DenoJsonc,
    DenoLock,
    LockJson,

    // Python
    RequirementsTxt,
    PyprojectToml,
    PoetryLock,
    Pipfile,
    PipfileLock,

    // Rust
    CargoToml,
    CargoLock,

    // Go
    GoMod,
    GoSum,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageManagerInfo {
    pub package_manager: PackageManager,
    pub source: PackageManagerSource,
    pub path: PathBuf,
    pub version: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PackageManagerDetection {
    pub language: Language,
    pub package_managers: Vec<PackageManagerInfo>,
}

impl TryFrom<&LanguageDetection> for PackageManagerDetection {
    type Error = ();

    fn try_from(lang_detection: &LanguageDetection) -> Result<Self, Self::Error> {
        let package_managers: Vec<PackageManagerInfo> = lang_detection
            .sources
            .iter()
            .filter_map(|signal| Vec::<PackageManagerInfo>::try_from(signal).ok())
            .flatten()
            .collect();

        if package_managers.is_empty() {
            Err(())
        } else {
            Ok(PackageManagerDetection {
                language: lang_detection.language.clone(),
                package_managers,
            })
        }
    }
}

fn detect_npm(path: &Path) -> Vec<PackageManagerInfo> {
    if path.exists() {
        vec![PackageManagerInfo {
            package_manager: PackageManager::Npm,
            source: PackageManagerSource::PackageLockJson,
            path: path.to_path_buf(),
            version: None,
        }]
    } else {
        vec![]
    }
}

fn detect_yarn(path: &Path) -> Vec<PackageManagerInfo> {
    if path.exists() {
        vec![PackageManagerInfo {
            package_manager: PackageManager::Yarn,
            source: PackageManagerSource::YarnLock,
            path: path.to_path_buf(),
            version: None,
        }]
    } else {
        vec![]
    }
}

fn detect_pnpm(path: &Path) -> Vec<PackageManagerInfo> {
    if path.exists() {
        vec![PackageManagerInfo {
            package_manager: PackageManager::Pnpm,
            source: PackageManagerSource::PnpmLockYaml,
            path: path.to_path_buf(),
            version: None,
        }]
    } else {
        vec![]
    }
}

fn detect_bun_lockb(path: &Path) -> Vec<PackageManagerInfo> {
    if path.exists() {
        vec![PackageManagerInfo {
            package_manager: PackageManager::Bun,
            source: PackageManagerSource::BunLockb,
            path: path.to_path_buf(),
            version: None,
        }]
    } else {
        vec![]
    }
}

fn detect_bun_lock(path: &Path) -> Vec<PackageManagerInfo> {
    if path.exists() {
        vec![PackageManagerInfo {
            package_manager: PackageManager::Bun,
            source: PackageManagerSource::BunLock,
            path: path.to_path_buf(),
            version: None,
        }]
    } else {
        vec![]
    }
}

fn detect_deno_json(path: &Path) -> Vec<PackageManagerInfo> {
    if path.exists() {
        vec![PackageManagerInfo {
            package_manager: PackageManager::Deno,
            source: PackageManagerSource::DenoJson,
            path: path.to_path_buf(),
            version: None,
        }]
    } else {
        vec![]
    }
}

fn detect_deno_jsonc(path: &Path) -> Vec<PackageManagerInfo> {
    if path.exists() {
        vec![PackageManagerInfo {
            package_manager: PackageManager::Deno,
            source: PackageManagerSource::DenoJsonc,
            path: path.to_path_buf(),
            version: None,
        }]
    } else {
        vec![]
    }
}

fn detect_deno_lock(path: &Path) -> Vec<PackageManagerInfo> {
    if path.exists() {
        vec![PackageManagerInfo {
            package_manager: PackageManager::Deno,
            source: PackageManagerSource::DenoLock,
            path: path.to_path_buf(),
            version: None,
        }]
    } else {
        vec![]
    }
}

fn detect_lock_json(path: &Path) -> Vec<PackageManagerInfo> {
    if path.exists() {
        vec![PackageManagerInfo {
            package_manager: PackageManager::Deno,
            source: PackageManagerSource::LockJson,
            path: path.to_path_buf(),
            version: None,
        }]
    } else {
        vec![]
    }
}

fn detect_from_package_json(path: &Path) -> Vec<PackageManagerInfo> {
    let Ok(content) = fs::read_to_string(path) else {
        return vec![];
    };

    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) else {
        return vec![];
    };

    if let Some(package_manager_str) = parsed.get("packageManager").and_then(|v| v.as_str()) {
        let (name, version) = if let Some(at_pos) = package_manager_str.find('@') {
            let (name_part, version_part) = package_manager_str.split_at(at_pos);
            (name_part, Some(version_part[1..].to_string()))
        } else {
            (package_manager_str, None)
        };

        let package_manager = match name {
            "npm" => Some(PackageManager::Npm),
            "pnpm" => Some(PackageManager::Pnpm),
            "yarn" => Some(PackageManager::Yarn),
            "bun" => Some(PackageManager::Bun),
            _ => None,
        };

        if let Some(pm) = package_manager {
            return vec![PackageManagerInfo {
                package_manager: pm,
                source: PackageManagerSource::PackageJson,
                path: path.to_path_buf(),
                version,
            }];
        }
    }

    vec![]
}

fn detect_pip(path: &Path) -> Vec<PackageManagerInfo> {
    if path.exists() {
        vec![PackageManagerInfo {
            package_manager: PackageManager::Pip,
            source: PackageManagerSource::RequirementsTxt,
            path: path.to_path_buf(),
            version: None,
        }]
    } else {
        vec![]
    }
}

fn detect_poetry_from_lock(path: &Path) -> Vec<PackageManagerInfo> {
    if path.exists() {
        vec![PackageManagerInfo {
            package_manager: PackageManager::Poetry,
            source: PackageManagerSource::PoetryLock,
            path: path.to_path_buf(),
            version: None,
        }]
    } else {
        vec![]
    }
}

fn detect_from_pyproject_toml(path: &Path) -> Vec<PackageManagerInfo> {
    let Ok(content) = fs::read_to_string(path) else {
        return vec![];
    };

    let Ok(parsed) = toml::from_str::<toml::Value>(&content) else {
        return vec![];
    };

    let mut results = Vec::new();

    if parsed.get("tool").and_then(|t| t.get("poetry")).is_some() {
        results.push(PackageManagerInfo {
            package_manager: PackageManager::Poetry,
            source: PackageManagerSource::PyprojectToml,
            path: path.to_path_buf(),
            version: None,
        });
    }

    if parsed.get("tool").and_then(|t| t.get("pdm")).is_some() {
        results.push(PackageManagerInfo {
            package_manager: PackageManager::Pdm,
            source: PackageManagerSource::PyprojectToml,
            path: path.to_path_buf(),
            version: None,
        });
    }

    if parsed.get("tool").and_then(|t| t.get("uv")).is_some()
        || parsed.get("dependency-groups").is_some()
    {
        results.push(PackageManagerInfo {
            package_manager: PackageManager::Uv,
            source: PackageManagerSource::PyprojectToml,
            path: path.to_path_buf(),
            version: None,
        });
    }

    if results.is_empty() {
        results.push(PackageManagerInfo {
            package_manager: PackageManager::Pip,
            source: PackageManagerSource::PyprojectToml,
            path: path.to_path_buf(),
            version: None,
        });
    }

    results
}

fn detect_pipenv(path: &Path) -> Vec<PackageManagerInfo> {
    if path.exists() {
        vec![PackageManagerInfo {
            package_manager: PackageManager::Pipenv,
            source: PackageManagerSource::Pipfile,
            path: path.to_path_buf(),
            version: None,
        }]
    } else {
        vec![]
    }
}

fn detect_pipenv_lock(path: &Path) -> Vec<PackageManagerInfo> {
    if path.exists() {
        vec![PackageManagerInfo {
            package_manager: PackageManager::Pipenv,
            source: PackageManagerSource::PipfileLock,
            path: path.to_path_buf(),
            version: None,
        }]
    } else {
        vec![]
    }
}

fn detect_cargo_toml(path: &Path) -> Vec<PackageManagerInfo> {
    if path.exists() {
        vec![PackageManagerInfo {
            package_manager: PackageManager::Cargo,
            source: PackageManagerSource::CargoToml,
            path: path.to_path_buf(),
            version: None,
        }]
    } else {
        vec![]
    }
}

fn detect_cargo_lock(path: &Path) -> Vec<PackageManagerInfo> {
    if path.exists() {
        vec![PackageManagerInfo {
            package_manager: PackageManager::Cargo,
            source: PackageManagerSource::CargoLock,
            path: path.to_path_buf(),
            version: None,
        }]
    } else {
        vec![]
    }
}

fn detect_go_mod(path: &Path) -> Vec<PackageManagerInfo> {
    if path.exists() {
        vec![PackageManagerInfo {
            package_manager: PackageManager::Go,
            source: PackageManagerSource::GoMod,
            path: path.to_path_buf(),
            version: None,
        }]
    } else {
        vec![]
    }
}

fn detect_go_sum(path: &Path) -> Vec<PackageManagerInfo> {
    if path.exists() {
        vec![PackageManagerInfo {
            package_manager: PackageManager::Go,
            source: PackageManagerSource::GoSum,
            path: path.to_path_buf(),
            version: None,
        }]
    } else {
        vec![]
    }
}

impl TryFrom<&LanguageDetectionSignal> for Vec<PackageManagerInfo> {
    type Error = ();

    fn try_from(signal: &LanguageDetectionSignal) -> Result<Self, Self::Error> {
        match signal {
            LanguageDetectionSignal::Strong { path, source } => {
                let package_managers = match source {
                    // JavaScript
                    LanguageDetectionSource::PackageJson => detect_from_package_json(path),
                    LanguageDetectionSource::PackageLockJson => detect_npm(path),
                    LanguageDetectionSource::YarnLock => detect_yarn(path),
                    LanguageDetectionSource::PnpmLockYaml => detect_pnpm(path),
                    LanguageDetectionSource::BunLockb => detect_bun_lockb(path),
                    LanguageDetectionSource::BunLock => detect_bun_lock(path),
                    LanguageDetectionSource::DenoJson => detect_deno_json(path),
                    LanguageDetectionSource::DenoJsonc => detect_deno_jsonc(path),
                    LanguageDetectionSource::DenoLock => detect_deno_lock(path),
                    LanguageDetectionSource::LockJson => detect_lock_json(path),

                    // Python
                    LanguageDetectionSource::RequirementsTxt => detect_pip(path),
                    LanguageDetectionSource::PyprojectToml => detect_from_pyproject_toml(path),
                    LanguageDetectionSource::PoetryLock => detect_poetry_from_lock(path),
                    LanguageDetectionSource::Pipfile => detect_pipenv(path),
                    LanguageDetectionSource::PipfileLock => detect_pipenv_lock(path),

                    // Rust
                    LanguageDetectionSource::CargoToml => detect_cargo_toml(path),
                    LanguageDetectionSource::CargoLock => detect_cargo_lock(path),

                    // Go
                    LanguageDetectionSource::GoMod => detect_go_mod(path),
                    LanguageDetectionSource::GoSum => detect_go_sum(path),

                    _ => vec![],
                };

                if package_managers.is_empty() {
                    Err(())
                } else {
                    Ok(package_managers)
                }
            }
            LanguageDetectionSignal::Weak(_) => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_temp_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let path = dir.path().join(name);
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    mod javascript {
        use super::*;

        #[test]
        fn test_detect_npm() {
            let dir = TempDir::new().unwrap();
            let path = create_temp_file(&dir, "package-lock.json", "{}");

            let pms = detect_npm(&path);
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Npm);
            assert!(matches!(
                pms[0].source,
                PackageManagerSource::PackageLockJson
            ));
        }

        #[test]
        fn test_detect_yarn() {
            let dir = TempDir::new().unwrap();
            let path = create_temp_file(&dir, "yarn.lock", "");

            let pms = detect_yarn(&path);
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Yarn);
            assert!(matches!(pms[0].source, PackageManagerSource::YarnLock));
        }

        #[test]
        fn test_detect_pnpm() {
            let dir = TempDir::new().unwrap();
            let path = create_temp_file(&dir, "pnpm-lock.yaml", "");

            let pms = detect_pnpm(&path);
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Pnpm);
            assert!(matches!(pms[0].source, PackageManagerSource::PnpmLockYaml));
        }

        #[test]
        fn test_detect_bun_lockb() {
            let dir = TempDir::new().unwrap();
            let path = create_temp_file(&dir, "bun.lockb", "");

            let pms = detect_bun_lockb(&path);
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Bun);
            assert!(matches!(pms[0].source, PackageManagerSource::BunLockb));
        }

        #[test]
        fn test_detect_deno_json() {
            let dir = TempDir::new().unwrap();
            let path = create_temp_file(&dir, "deno.json", "{}");

            let pms = detect_deno_json(&path);
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Deno);
            assert!(matches!(pms[0].source, PackageManagerSource::DenoJson));
        }

        #[test]
        fn test_detect_lock_json() {
            let dir = TempDir::new().unwrap();
            let path = create_temp_file(&dir, "lock.json", "{}");

            let pms = detect_lock_json(&path);
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Deno);
            assert!(matches!(pms[0].source, PackageManagerSource::LockJson));
        }

        #[test]
        fn test_detect_from_package_json_with_version() {
            let dir = TempDir::new().unwrap();
            let content = r#"{"packageManager": "pnpm@9.0.0"}"#;
            let path = create_temp_file(&dir, "package.json", content);

            let pms = detect_from_package_json(&path);
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Pnpm);
            assert!(matches!(pms[0].source, PackageManagerSource::PackageJson));
            assert_eq!(pms[0].version, Some("9.0.0".to_string()));
        }

        #[test]
        fn test_detect_from_package_json_without_version() {
            let dir = TempDir::new().unwrap();
            let content = r#"{"packageManager": "pnpm"}"#;
            let path = create_temp_file(&dir, "package.json", content);

            let pms = detect_from_package_json(&path);
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Pnpm);
            assert!(matches!(pms[0].source, PackageManagerSource::PackageJson));
            assert_eq!(pms[0].version, None);
        }

        #[test]
        fn test_detect_from_package_json_missing_field() {
            let dir = TempDir::new().unwrap();
            let content = r#"{"name": "test"}"#;
            let path = create_temp_file(&dir, "package.json", content);

            let pms = detect_from_package_json(&path);
            assert_eq!(pms.len(), 0);
        }

        #[test]
        fn test_detect_from_package_json_unknown_manager() {
            let dir = TempDir::new().unwrap();
            let content = r#"{"packageManager": "unknown@1.0.0"}"#;
            let path = create_temp_file(&dir, "package.json", content);

            let pms = detect_from_package_json(&path);
            assert_eq!(pms.len(), 0);
        }

        #[test]
        fn test_detect_from_package_json_all_managers() {
            let dir = TempDir::new().unwrap();

            for (pm_str, expected_pm, expected_version) in [
                ("npm@10.0.0", PackageManager::Npm, "10.0.0"),
                ("pnpm@9.0.0", PackageManager::Pnpm, "9.0.0"),
                ("yarn@4.0.0", PackageManager::Yarn, "4.0.0"),
                ("bun@1.0.0", PackageManager::Bun, "1.0.0"),
            ] {
                let content = format!(r#"{{"packageManager": "{}"}}"#, pm_str);
                let path = create_temp_file(&dir, &format!("package-{}.json", pm_str), &content);

                let pms = detect_from_package_json(&path);
                assert_eq!(pms.len(), 1);
                assert_eq!(pms[0].package_manager, expected_pm);
                assert_eq!(pms[0].version, Some(expected_version.to_string()));
            }
        }
    }

    mod python {
        use super::*;

        #[test]
        fn test_detect_pip() {
            let dir = TempDir::new().unwrap();
            let path = create_temp_file(&dir, "requirements.txt", "requests==2.28.0");

            let pms = detect_pip(&path);
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Pip);
            assert!(matches!(
                pms[0].source,
                PackageManagerSource::RequirementsTxt
            ));
        }

        #[test]
        fn test_detect_poetry_from_pyproject() {
            let dir = TempDir::new().unwrap();
            let content = r#"
[tool.poetry]
name = "test"
version = "0.1.0"
"#;
            let path = create_temp_file(&dir, "pyproject.toml", content);

            let pms = detect_from_pyproject_toml(&path);
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Poetry);
            assert!(matches!(pms[0].source, PackageManagerSource::PyprojectToml));
        }

        #[test]
        fn test_detect_pdm_from_pyproject() {
            let dir = TempDir::new().unwrap();
            let content = r#"
[tool.pdm]
"#;
            let path = create_temp_file(&dir, "pyproject.toml", content);

            let pms = detect_from_pyproject_toml(&path);
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Pdm);
        }

        #[test]
        fn test_detect_uv_from_pyproject() {
            let dir = TempDir::new().unwrap();
            let content = r#"
[tool.uv]
"#;
            let path = create_temp_file(&dir, "pyproject.toml", content);

            let pms = detect_from_pyproject_toml(&path);
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Uv);
        }

        #[test]
        fn test_detect_pip_from_plain_pyproject() {
            let dir = TempDir::new().unwrap();
            let content = r#"
[project]
name = "test"
"#;
            let path = create_temp_file(&dir, "pyproject.toml", content);

            let pms = detect_from_pyproject_toml(&path);
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Pip);
        }

        #[test]
        fn test_detect_pipenv() {
            let dir = TempDir::new().unwrap();
            let path = create_temp_file(&dir, "Pipfile", "");

            let pms = detect_pipenv(&path);
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Pipenv);
            assert!(matches!(pms[0].source, PackageManagerSource::Pipfile));
        }

        #[test]
        fn test_detect_poetry_from_lock() {
            let dir = TempDir::new().unwrap();
            let path = create_temp_file(&dir, "poetry.lock", "");

            let pms = detect_poetry_from_lock(&path);
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Poetry);
            assert!(matches!(pms[0].source, PackageManagerSource::PoetryLock));
        }
    }

    mod rust {
        use super::*;

        #[test]
        fn test_detect_cargo_toml() {
            let dir = TempDir::new().unwrap();
            let content = r#"
[package]
name = "test"
"#;
            let path = create_temp_file(&dir, "Cargo.toml", content);

            let pms = detect_cargo_toml(&path);
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Cargo);
            assert!(matches!(pms[0].source, PackageManagerSource::CargoToml));
        }

        #[test]
        fn test_detect_cargo_lock() {
            let dir = TempDir::new().unwrap();
            let path = create_temp_file(&dir, "Cargo.lock", "");

            let pms = detect_cargo_lock(&path);
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Cargo);
            assert!(matches!(pms[0].source, PackageManagerSource::CargoLock));
        }
    }

    mod go {
        use super::*;

        #[test]
        fn test_detect_go_mod() {
            let dir = TempDir::new().unwrap();
            let content = "module test\n\ngo 1.21\n";
            let path = create_temp_file(&dir, "go.mod", content);

            let pms = detect_go_mod(&path);
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Go);
            assert!(matches!(pms[0].source, PackageManagerSource::GoMod));
        }

        #[test]
        fn test_detect_go_sum() {
            let dir = TempDir::new().unwrap();
            let path = create_temp_file(&dir, "go.sum", "");

            let pms = detect_go_sum(&path);
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Go);
            assert!(matches!(pms[0].source, PackageManagerSource::GoSum));
        }
    }

    mod integration {
        use super::*;

        #[test]
        fn test_signal_conversion_npm() {
            let dir = TempDir::new().unwrap();
            let path = create_temp_file(&dir, "package-lock.json", "{}");

            let signal = LanguageDetectionSignal::Strong {
                path,
                source: LanguageDetectionSource::PackageLockJson,
            };

            let result = Vec::<PackageManagerInfo>::try_from(&signal);
            assert!(result.is_ok());
            let pms = result.unwrap();
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Npm);
        }

        #[test]
        fn test_signal_conversion_weak_signal() {
            let signal = LanguageDetectionSignal::Weak(LanguageDetectionSource::JsFile);
            let result = Vec::<PackageManagerInfo>::try_from(&signal);
            assert!(result.is_err());
        }

        #[test]
        fn test_package_manager_detection_from_language_detection() {
            let dir = TempDir::new().unwrap();
            let npm_path = create_temp_file(&dir, "package-lock.json", "{}");
            let package_json_path = create_temp_file(&dir, "package.json", "{}");

            let lang_detection = LanguageDetection::new(
                Language::JavaScript,
                vec![
                    LanguageDetectionSignal::Strong {
                        path: npm_path,
                        source: LanguageDetectionSource::PackageLockJson,
                    },
                    LanguageDetectionSignal::Strong {
                        path: package_json_path,
                        source: LanguageDetectionSource::PackageJson,
                    },
                ],
            );

            let pm_detection = PackageManagerDetection::try_from(&lang_detection);
            assert!(pm_detection.is_ok());

            let detection = pm_detection.unwrap();
            assert!(matches!(detection.language, Language::JavaScript));
            assert_eq!(detection.package_managers.len(), 1);
            assert_eq!(
                detection.package_managers[0].package_manager,
                PackageManager::Npm
            );
        }

        #[test]
        fn test_package_manager_detection_no_package_managers() {
            let lang_detection = LanguageDetection::new(
                Language::JavaScript,
                vec![LanguageDetectionSignal::Weak(
                    LanguageDetectionSource::JsFile,
                )],
            );

            let pm_detection = PackageManagerDetection::try_from(&lang_detection);
            assert!(pm_detection.is_err());
        }

        #[test]
        fn test_pipfile_detection_via_signal() {
            let dir = TempDir::new().unwrap();
            let pipfile_path = create_temp_file(&dir, "Pipfile", "");

            let signal = LanguageDetectionSignal::Strong {
                path: pipfile_path,
                source: LanguageDetectionSource::Pipfile,
            };

            let result = Vec::<PackageManagerInfo>::try_from(&signal);
            assert!(result.is_ok());
            let pms = result.unwrap();
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Pipenv);
        }

        #[test]
        fn test_lock_json_detection_via_signal() {
            let dir = TempDir::new().unwrap();
            let lock_json_path = create_temp_file(&dir, "lock.json", "{}");

            let signal = LanguageDetectionSignal::Strong {
                path: lock_json_path,
                source: LanguageDetectionSource::LockJson,
            };

            let result = Vec::<PackageManagerInfo>::try_from(&signal);
            assert!(result.is_ok());
            let pms = result.unwrap();
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Deno);
            assert!(matches!(pms[0].source, PackageManagerSource::LockJson));
        }

        #[test]
        fn test_package_json_with_packagemanager_via_signal() {
            let dir = TempDir::new().unwrap();
            let content = r#"{"packageManager": "yarn@3.5.0"}"#;
            let package_json_path = create_temp_file(&dir, "package.json", content);

            let signal = LanguageDetectionSignal::Strong {
                path: package_json_path,
                source: LanguageDetectionSource::PackageJson,
            };

            let result = Vec::<PackageManagerInfo>::try_from(&signal);
            assert!(result.is_ok());
            let pms = result.unwrap();
            assert_eq!(pms.len(), 1);
            assert_eq!(pms[0].package_manager, PackageManager::Yarn);
            assert_eq!(pms[0].version, Some("3.5.0".to_string()));
        }

        #[test]
        fn test_multiple_python_package_managers() {
            let dir = TempDir::new().unwrap();
            let content = r#"
[tool.poetry]
name = "test"

[tool.pdm]
"#;
            let pyproject_path = create_temp_file(&dir, "pyproject.toml", content);

            let signal = LanguageDetectionSignal::Strong {
                path: pyproject_path,
                source: LanguageDetectionSource::PyprojectToml,
            };

            let result = Vec::<PackageManagerInfo>::try_from(&signal);
            assert!(result.is_ok());
            let pms = result.unwrap();
            assert_eq!(pms.len(), 2);

            let has_poetry = pms
                .iter()
                .any(|pm| pm.package_manager == PackageManager::Poetry);
            let has_pdm = pms
                .iter()
                .any(|pm| pm.package_manager == PackageManager::Pdm);
            assert!(has_poetry);
            assert!(has_pdm);
        }
    }
}
