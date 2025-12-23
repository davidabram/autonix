use super::{Language, LanguageDetection, LanguageDetectionSignal, LanguageDetectionSource};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, Serialize)]
pub enum VersionSource {
    GoModDirective,
    GoVersionFile,

    RustToolchainFile,
    RustToolchainToml,
    CargoTomlRustVersion,

    PyprojectRequiresPython,
    PythonVersionFile,
    PipfilePythonVersion,
    SetupPyPythonRequires,

    PackageJsonEnginesNode,
    NvmrcFile,
    NodeVersionFile,

    BunVersionFile,
    PackageJsonEnginesBun,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum VersionConstraint {
    Exact,
    GreaterOrEqual,
    LessOrEqual,
    GreaterThan,
    LessThan,
    Caret,
    Tilde,
    Wildcard,
}

#[derive(Debug, Clone, Serialize)]
pub struct SemanticVersion {
    pub major: Option<u32>,
    pub minor: Option<u32>,
    pub patch: Option<u32>,
    pub pre_release: Option<String>,
    pub build: Option<String>,
    pub constraint: VersionConstraint,
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionInfo {
    pub raw: String,
    pub parsed: Option<SemanticVersion>,
    pub source: VersionSource,
    pub path: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct VersionDetection {
    pub language: Language,
    pub versions: Vec<VersionInfo>,
}

impl TryFrom<&LanguageDetection> for VersionDetection {
    type Error = ();

    fn try_from(lang_detection: &LanguageDetection) -> Result<Self, Self::Error> {
        let versions: Vec<VersionInfo> = lang_detection
            .sources
            .iter()
            .filter_map(|signal| Vec::<VersionInfo>::try_from(signal).ok())
            .flatten()
            .collect();

        if versions.is_empty() {
            Err(())
        } else {
            Ok(VersionDetection {
                language: lang_detection.language.clone(),
                versions,
            })
        }
    }
}

fn parse_constraint(s: &str) -> (VersionConstraint, &str) {
    const CONSTRAINTS: &[(&str, VersionConstraint)] = &[
        (">=", VersionConstraint::GreaterOrEqual),
        ("<=", VersionConstraint::LessOrEqual),
        (">", VersionConstraint::GreaterThan),
        ("<", VersionConstraint::LessThan),
        ("^", VersionConstraint::Caret),
        ("~", VersionConstraint::Tilde),
        ("=", VersionConstraint::Exact),
    ];

    for (prefix, constraint) in CONSTRAINTS {
        if let Some(rest) = s.strip_prefix(prefix) {
            return (*constraint, rest.trim());
        }
    }

    (VersionConstraint::Exact, s)
}

fn parse_semantic_version(raw: &str) -> Option<SemanticVersion> {
    let trimmed = raw
        .trim()
        .trim_start_matches('v')
        .trim_start_matches("python-")
        .trim_start_matches("node-");

    if trimmed == "*" {
        return Some(SemanticVersion {
            major: None,
            minor: None,
            patch: None,
            pre_release: None,
            build: None,
            constraint: VersionConstraint::Wildcard,
        });
    }

    let (constraint, version_str) = parse_constraint(trimmed);

    if version_str.is_empty() {
        return None;
    }

    let parts: Vec<&str> = version_str.split('.').collect();
    let major = parts.first().and_then(|p| p.parse::<u32>().ok())?;
    let minor = parts.get(1).and_then(|p| p.parse::<u32>().ok());
    let patch = parts.get(2).and_then(|p| {
        p.split(&['-', '+'][..])
            .next()
            .and_then(|n| n.parse::<u32>().ok())
    });

    let pre_release = version_str
        .split('-')
        .nth(1)
        .map(|p| p.split('+').next().unwrap_or(p).to_string());

    let build = version_str.split('+').nth(1).map(String::from);

    Some(SemanticVersion {
        major: Some(major),
        minor,
        patch,
        pre_release,
        build,
        constraint,
    })
}

fn parse_version_or_expression(raw: &str) -> Option<SemanticVersion> {
    if !raw.contains("||") {
        return parse_semantic_version(raw);
    }

    raw.split("||")
        .filter_map(|v| parse_semantic_version(v.trim()))
        .max_by_key(|v| (v.major, v.minor, v.patch))
}

fn parse_go_mod(path: &PathBuf) -> Vec<VersionInfo> {
    let Ok(content) = fs::read_to_string(path) else {
        return vec![];
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("go ") {
            let version = trimmed.trim_start_matches("go ").trim();
            return vec![VersionInfo {
                raw: version.to_string(),
                parsed: parse_semantic_version(version),
                source: VersionSource::GoModDirective,
                path: path.clone(),
            }];
        }
    }
    vec![]
}

fn parse_simple_version_file(path: &PathBuf, source: VersionSource) -> Vec<VersionInfo> {
    let Ok(content) = fs::read_to_string(path) else {
        return vec![];
    };
    let version = content.trim();

    if version.is_empty() {
        return vec![];
    }

    vec![VersionInfo {
        raw: version.to_string(),
        parsed: parse_semantic_version(version),
        source,
        path: path.clone(),
    }]
}

fn parse_go_version_file(path: &PathBuf) -> Vec<VersionInfo> {
    parse_simple_version_file(path, VersionSource::GoVersionFile)
}

fn parse_rust_toolchain(path: &PathBuf) -> Vec<VersionInfo> {
    parse_simple_version_file(path, VersionSource::RustToolchainFile)
}

fn parse_toml_field(
    path: &PathBuf,
    field_path: &[&str],
    source: VersionSource,
) -> Vec<VersionInfo> {
    let Ok(content) = fs::read_to_string(path) else {
        return vec![];
    };
    let Ok(parsed) = toml::from_str::<toml::Value>(&content) else {
        return vec![];
    };

    let mut current = &parsed;
    for field in field_path {
        let Some(next) = current.get(field) else {
            return vec![];
        };
        current = next;
    }

    let Some(version_str) = current.as_str() else {
        return vec![];
    };

    vec![VersionInfo {
        raw: version_str.to_string(),
        parsed: parse_semantic_version(version_str),
        source,
        path: path.clone(),
    }]
}

fn parse_rust_toolchain_toml(path: &PathBuf) -> Vec<VersionInfo> {
    let Ok(content) = fs::read_to_string(path) else {
        return vec![];
    };
    let Ok(parsed) = toml::from_str::<toml::Value>(&content) else {
        return vec![];
    };

    let Some(channel) = parsed
        .get("toolchain")
        .and_then(|t| t.get("channel"))
        .or_else(|| parsed.get("channel"))
        .and_then(|c| c.as_str())
    else {
        return vec![];
    };

    vec![VersionInfo {
        raw: channel.to_string(),
        parsed: parse_semantic_version(channel),
        source: VersionSource::RustToolchainToml,
        path: path.clone(),
    }]
}

fn parse_cargo_toml_rust_version(path: &PathBuf) -> Vec<VersionInfo> {
    parse_toml_field(
        path,
        &["package", "rust-version"],
        VersionSource::CargoTomlRustVersion,
    )
}

fn parse_pyproject_toml(path: &PathBuf) -> Vec<VersionInfo> {
    parse_toml_field(
        path,
        &["project", "requires-python"],
        VersionSource::PyprojectRequiresPython,
    )
}

fn parse_python_version_file(path: &PathBuf) -> Vec<VersionInfo> {
    parse_simple_version_file(path, VersionSource::PythonVersionFile)
}

fn parse_pipfile(path: &PathBuf) -> Vec<VersionInfo> {
    parse_toml_field(
        path,
        &["requires", "python_version"],
        VersionSource::PipfilePythonVersion,
    )
}

fn parse_setup_py(path: &PathBuf) -> Vec<VersionInfo> {
    static PYTHON_REQUIRES_RE: OnceLock<regex::Regex> = OnceLock::new();

    let Ok(content) = fs::read_to_string(path) else {
        return vec![];
    };
    let re = PYTHON_REQUIRES_RE.get_or_init(|| {
        regex::Regex::new(r#"python_requires\s*=\s*["']([^"']+)["']"#)
            .expect("invalid regex pattern")
    });

    if let Some(captures) = re.captures(&content)
        && let Some(version_match) = captures.get(1)
    {
        let version = version_match.as_str();
        return vec![VersionInfo {
            raw: version.to_string(),
            parsed: parse_semantic_version(version),
            source: VersionSource::SetupPyPythonRequires,
            path: path.clone(),
        }];
    }

    vec![]
}

fn parse_package_json(path: &PathBuf) -> Vec<VersionInfo> {
    let mut versions = Vec::new();

    let Ok(content) = fs::read_to_string(path) else {
        return versions;
    };

    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) else {
        return versions;
    };

    if let Some(engines) = parsed.get("engines").and_then(|e| e.as_object()) {
        if let Some(node_version) = engines.get("node").and_then(|v| v.as_str()) {
            let parsed_version = parse_version_or_expression(node_version);

            versions.push(VersionInfo {
                raw: node_version.to_string(),
                parsed: parsed_version,
                source: VersionSource::PackageJsonEnginesNode,
                path: path.clone(),
            });
        }

        if let Some(bun_version) = engines.get("bun").and_then(|v| v.as_str()) {
            let parsed_version = parse_version_or_expression(bun_version);

            versions.push(VersionInfo {
                raw: bun_version.to_string(),
                parsed: parsed_version,
                source: VersionSource::PackageJsonEnginesBun,
                path: path.clone(),
            });
        }
    }

    versions
}

fn parse_nvmrc(path: &PathBuf) -> Vec<VersionInfo> {
    parse_simple_version_file(path, VersionSource::NvmrcFile)
}

fn parse_node_version_file(path: &PathBuf) -> Vec<VersionInfo> {
    parse_simple_version_file(path, VersionSource::NodeVersionFile)
}

fn parse_bun_version_file(path: &PathBuf) -> Vec<VersionInfo> {
    parse_simple_version_file(path, VersionSource::BunVersionFile)
}

impl TryFrom<&LanguageDetectionSignal> for Vec<VersionInfo> {
    type Error = ();

    fn try_from(signal: &LanguageDetectionSignal) -> Result<Self, Self::Error> {
        match signal {
            LanguageDetectionSignal::Strong { path, source } => {
                let versions = match source {
                    // Go
                    LanguageDetectionSource::GoMod => parse_go_mod(path),
                    LanguageDetectionSource::GoVersionFile => parse_go_version_file(path),

                    // Rust
                    LanguageDetectionSource::CargoToml => parse_cargo_toml_rust_version(path),
                    LanguageDetectionSource::RustToolchain => parse_rust_toolchain(path),
                    LanguageDetectionSource::RustToolchainToml => parse_rust_toolchain_toml(path),

                    // Python
                    LanguageDetectionSource::PyprojectToml => parse_pyproject_toml(path),
                    LanguageDetectionSource::PythonVersionFile => parse_python_version_file(path),
                    LanguageDetectionSource::Pipfile => parse_pipfile(path),
                    LanguageDetectionSource::SetupPy => parse_setup_py(path),

                    // JavaScript/Node
                    LanguageDetectionSource::PackageJson => parse_package_json(path),
                    LanguageDetectionSource::NvmrcFile => parse_nvmrc(path),
                    LanguageDetectionSource::NodeVersionFile => parse_node_version_file(path),
                    LanguageDetectionSource::BunVersionFile => parse_bun_version_file(path),

                    _ => vec![],
                };

                if versions.is_empty() {
                    Err(())
                } else {
                    Ok(versions)
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

    mod parse_constraint {
        use super::*;

        #[test]
        fn test_exact_constraint() {
            assert!(matches!(
                parse_constraint("1.2.3"),
                (VersionConstraint::Exact, "1.2.3")
            ));
            assert!(matches!(
                parse_constraint("=1.2.3"),
                (VersionConstraint::Exact, "1.2.3")
            ));
        }

        #[test]
        fn test_greater_or_equal() {
            assert!(matches!(
                parse_constraint(">=1.2.3"),
                (VersionConstraint::GreaterOrEqual, "1.2.3")
            ));
        }

        #[test]
        fn test_less_or_equal() {
            assert!(matches!(
                parse_constraint("<=1.2.3"),
                (VersionConstraint::LessOrEqual, "1.2.3")
            ));
        }

        #[test]
        fn test_greater_than() {
            assert!(matches!(
                parse_constraint(">1.2.3"),
                (VersionConstraint::GreaterThan, "1.2.3")
            ));
        }

        #[test]
        fn test_less_than() {
            assert!(matches!(
                parse_constraint("<1.2.3"),
                (VersionConstraint::LessThan, "1.2.3")
            ));
        }

        #[test]
        fn test_caret_constraint() {
            assert!(matches!(
                parse_constraint("^1.2.3"),
                (VersionConstraint::Caret, "1.2.3")
            ));
        }

        #[test]
        fn test_tilde_constraint() {
            assert!(matches!(
                parse_constraint("~1.2.3"),
                (VersionConstraint::Tilde, "1.2.3")
            ));
        }

        #[test]
        fn test_whitespace_handling() {
            let (constraint, version) = parse_constraint(">=  1.2.3  ");
            assert!(matches!(constraint, VersionConstraint::GreaterOrEqual));
            assert_eq!(version, "1.2.3");
        }

        #[test]
        fn test_empty_string() {
            assert!(matches!(
                parse_constraint(""),
                (VersionConstraint::Exact, "")
            ));
        }
    }

    mod parse_semantic_version {
        use super::*;

        #[test]
        fn test_full_version() {
            let result = parse_semantic_version("1.2.3").unwrap();
            assert_eq!(result.major, Some(1));
            assert_eq!(result.minor, Some(2));
            assert_eq!(result.patch, Some(3));
            assert_eq!(result.pre_release, None);
            assert_eq!(result.build, None);
            assert!(matches!(result.constraint, VersionConstraint::Exact));
        }

        #[test]
        fn test_version_with_v_prefix() {
            let result = parse_semantic_version("v1.2.3").unwrap();
            assert_eq!(result.major, Some(1));
            assert_eq!(result.minor, Some(2));
            assert_eq!(result.patch, Some(3));
        }

        #[test]
        fn test_python_prefix() {
            let result = parse_semantic_version("python-3.9.0").unwrap();
            assert_eq!(result.major, Some(3));
            assert_eq!(result.minor, Some(9));
            assert_eq!(result.patch, Some(0));
        }

        #[test]
        fn test_node_prefix() {
            let result = parse_semantic_version("node-18.0.0").unwrap();
            assert_eq!(result.major, Some(18));
            assert_eq!(result.minor, Some(0));
            assert_eq!(result.patch, Some(0));
        }

        #[test]
        fn test_partial_version_major_minor() {
            let result = parse_semantic_version("1.2").unwrap();
            assert_eq!(result.major, Some(1));
            assert_eq!(result.minor, Some(2));
            assert_eq!(result.patch, None);
        }

        #[test]
        fn test_partial_version_major_only() {
            let result = parse_semantic_version("1").unwrap();
            assert_eq!(result.major, Some(1));
            assert_eq!(result.minor, None);
            assert_eq!(result.patch, None);
        }

        #[test]
        fn test_pre_release_version() {
            let result = parse_semantic_version("1.2.3-alpha").unwrap();
            assert_eq!(result.major, Some(1));
            assert_eq!(result.minor, Some(2));
            assert_eq!(result.patch, Some(3));
            assert_eq!(result.pre_release, Some("alpha".to_string()));
            assert_eq!(result.build, None);
        }

        #[test]
        fn test_pre_release_with_number() {
            let result = parse_semantic_version("1.2.3-beta.1").unwrap();
            assert_eq!(result.pre_release, Some("beta.1".to_string()));
        }

        #[test]
        fn test_build_metadata() {
            let result = parse_semantic_version("1.2.3+build123").unwrap();
            assert_eq!(result.major, Some(1));
            assert_eq!(result.minor, Some(2));
            assert_eq!(result.patch, Some(3));
            assert_eq!(result.pre_release, None);
            assert_eq!(result.build, Some("build123".to_string()));
        }

        #[test]
        fn test_pre_release_and_build() {
            let result = parse_semantic_version("1.2.3-alpha+build").unwrap();
            assert_eq!(result.pre_release, Some("alpha".to_string()));
            assert_eq!(result.build, Some("build".to_string()));
        }

        #[test]
        fn test_wildcard() {
            let result = parse_semantic_version("*").unwrap();
            assert_eq!(result.major, None);
            assert_eq!(result.minor, None);
            assert_eq!(result.patch, None);
            assert!(matches!(result.constraint, VersionConstraint::Wildcard));
        }

        #[test]
        fn test_greater_or_equal_constraint() {
            let result = parse_semantic_version(">=1.2.3").unwrap();
            assert_eq!(result.major, Some(1));
            assert!(matches!(
                result.constraint,
                VersionConstraint::GreaterOrEqual
            ));
        }

        #[test]
        fn test_caret_constraint() {
            let result = parse_semantic_version("^1.2.3").unwrap();
            assert!(matches!(result.constraint, VersionConstraint::Caret));
        }

        #[test]
        fn test_tilde_constraint() {
            let result = parse_semantic_version("~1.2.3").unwrap();
            assert!(matches!(result.constraint, VersionConstraint::Tilde));
        }

        #[test]
        fn test_whitespace() {
            let result = parse_semantic_version("  1.2.3  ").unwrap();
            assert_eq!(result.major, Some(1));
            assert_eq!(result.minor, Some(2));
            assert_eq!(result.patch, Some(3));
        }

        #[test]
        fn test_empty_string() {
            assert!(parse_semantic_version("").is_none());
        }

        #[test]
        fn test_invalid_format() {
            assert!(parse_semantic_version("invalid").is_none());
            assert!(parse_semantic_version("a.b.c").is_none());
        }

        #[test]
        fn test_constraint_only() {
            assert!(parse_semantic_version(">=").is_none());
            assert!(parse_semantic_version("^").is_none());
        }
    }

    mod parse_version_or_expression {
        use super::*;

        #[test]
        fn test_single_version() {
            let result = parse_version_or_expression("1.2.3").unwrap();
            assert_eq!(result.major, Some(1));
            assert_eq!(result.minor, Some(2));
            assert_eq!(result.patch, Some(3));
        }

        #[test]
        fn test_or_expression_picks_max() {
            let result = parse_version_or_expression(">=3.8 || >=3.9").unwrap();
            assert_eq!(result.major, Some(3));
            assert_eq!(result.minor, Some(9));
        }

        #[test]
        fn test_or_expression_multiple_versions() {
            let result = parse_version_or_expression("1.0.0 || 2.0.0 || 1.5.0").unwrap();
            assert_eq!(result.major, Some(2));
            assert_eq!(result.minor, Some(0));
            assert_eq!(result.patch, Some(0));
        }

        #[test]
        fn test_or_expression_with_whitespace() {
            let result = parse_version_or_expression("  1.0.0  ||  2.0.0  ").unwrap();
            assert_eq!(result.major, Some(2));
        }

        #[test]
        fn test_or_expression_with_invalid_parts() {
            let result = parse_version_or_expression("invalid || 1.2.3");
            assert!(result.is_some());
        }

        #[test]
        fn test_all_invalid_or_expression() {
            assert!(parse_version_or_expression("invalid || bad").is_none());
        }
    }

    mod file_parsers {
        use super::*;
        use std::fs;
        use tempfile::TempDir;

        fn create_temp_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
            let path = dir.path().join(name);
            let mut file = fs::File::create(&path).unwrap();
            file.write_all(content.as_bytes()).unwrap();
            path
        }

        mod go_mod {
            use super::*;

            #[test]
            fn test_valid_go_mod() {
                let dir = TempDir::new().unwrap();
                let path = create_temp_file(&dir, "go.mod", "module example.com\n\ngo 1.21\n");

                let versions = parse_go_mod(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, "1.21");
                assert!(matches!(versions[0].source, VersionSource::GoModDirective));
                assert_eq!(versions[0].parsed.as_ref().unwrap().major, Some(1));
                assert_eq!(versions[0].parsed.as_ref().unwrap().minor, Some(21));
            }

            #[test]
            fn test_go_mod_with_extra_whitespace() {
                let dir = TempDir::new().unwrap();
                let path =
                    create_temp_file(&dir, "go.mod", "module example.com\n\n  go   1.20  \n");

                let versions = parse_go_mod(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, "1.20");
            }

            #[test]
            fn test_go_mod_multiple_lines() {
                let dir = TempDir::new().unwrap();
                let content = r#"
module example.com

go 1.21

require (
    github.com/foo/bar v1.2.3
)
"#;
                let path = create_temp_file(&dir, "go.mod", content);

                let versions = parse_go_mod(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, "1.21");
            }

            #[test]
            fn test_go_mod_missing_directive() {
                let dir = TempDir::new().unwrap();
                let path = create_temp_file(&dir, "go.mod", "module example.com\n");

                let versions = parse_go_mod(&path);
                assert!(versions.is_empty());
            }

            #[test]
            fn test_go_mod_nonexistent_file() {
                let path = PathBuf::from("/nonexistent/go.mod");
                let versions = parse_go_mod(&path);
                assert!(versions.is_empty());
            }
        }

        mod simple_version_files {
            use super::*;

            #[test]
            fn test_go_version_file() {
                let dir = TempDir::new().unwrap();
                let path = create_temp_file(&dir, ".go-version", "1.21.0\n");

                let versions = parse_go_version_file(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, "1.21.0");
                assert!(matches!(versions[0].source, VersionSource::GoVersionFile));
            }

            #[test]
            fn test_python_version_file() {
                let dir = TempDir::new().unwrap();
                let path = create_temp_file(&dir, ".python-version", "3.11.0\n");

                let versions = parse_python_version_file(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, "3.11.0");
                assert!(matches!(
                    versions[0].source,
                    VersionSource::PythonVersionFile
                ));
            }

            #[test]
            fn test_node_version_file() {
                let dir = TempDir::new().unwrap();
                let path = create_temp_file(&dir, ".node-version", "18.0.0\n");

                let versions = parse_node_version_file(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, "18.0.0");
                assert!(matches!(versions[0].source, VersionSource::NodeVersionFile));
            }

            #[test]
            fn test_nvmrc_file() {
                let dir = TempDir::new().unwrap();
                let path = create_temp_file(&dir, ".nvmrc", "v18.12.0\n");

                let versions = parse_nvmrc(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, "v18.12.0");
                assert!(matches!(versions[0].source, VersionSource::NvmrcFile));
            }

            #[test]
            fn test_bun_version_file() {
                let dir = TempDir::new().unwrap();
                let path = create_temp_file(&dir, ".bun-version", "1.0.0\n");

                let versions = parse_bun_version_file(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, "1.0.0");
                assert!(matches!(versions[0].source, VersionSource::BunVersionFile));
            }

            #[test]
            fn test_rust_toolchain_file() {
                let dir = TempDir::new().unwrap();
                let path = create_temp_file(&dir, "rust-toolchain", "1.70.0\n");

                let versions = parse_rust_toolchain(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, "1.70.0");
                assert!(matches!(
                    versions[0].source,
                    VersionSource::RustToolchainFile
                ));
            }

            #[test]
            fn test_empty_file() {
                let dir = TempDir::new().unwrap();
                let path = create_temp_file(&dir, ".node-version", "");

                let versions = parse_node_version_file(&path);
                assert!(versions.is_empty());
            }

            #[test]
            fn test_whitespace_only_file() {
                let dir = TempDir::new().unwrap();
                let path = create_temp_file(&dir, ".node-version", "   \n  \n");

                let versions = parse_node_version_file(&path);
                assert!(versions.is_empty());
            }

            #[test]
            fn test_version_with_whitespace() {
                let dir = TempDir::new().unwrap();
                let path = create_temp_file(&dir, ".node-version", "  18.0.0  \n");

                let versions = parse_node_version_file(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, "18.0.0");
            }
        }

        mod toml_files {
            use super::*;

            #[test]
            fn test_cargo_toml_rust_version() {
                let dir = TempDir::new().unwrap();
                let content = r#"
[package]
name = "example"
rust-version = "1.70.0"
"#;
                let path = create_temp_file(&dir, "Cargo.toml", content);

                let versions = parse_cargo_toml_rust_version(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, "1.70.0");
                assert!(matches!(
                    versions[0].source,
                    VersionSource::CargoTomlRustVersion
                ));
            }

            #[test]
            fn test_cargo_toml_missing_rust_version() {
                let dir = TempDir::new().unwrap();
                let content = r#"
[package]
name = "example"
"#;
                let path = create_temp_file(&dir, "Cargo.toml", content);

                let versions = parse_cargo_toml_rust_version(&path);
                assert!(versions.is_empty());
            }

            #[test]
            fn test_rust_toolchain_toml_with_toolchain_channel() {
                let dir = TempDir::new().unwrap();
                let content = r#"
[toolchain]
channel = "1.70.0"
"#;
                let path = create_temp_file(&dir, "rust-toolchain.toml", content);

                let versions = parse_rust_toolchain_toml(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, "1.70.0");
                assert!(matches!(
                    versions[0].source,
                    VersionSource::RustToolchainToml
                ));
            }

            #[test]
            fn test_rust_toolchain_toml_with_root_channel() {
                let dir = TempDir::new().unwrap();
                let content = r#"
channel = "stable"
"#;
                let path = create_temp_file(&dir, "rust-toolchain.toml", content);

                let versions = parse_rust_toolchain_toml(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, "stable");
            }

            #[test]
            fn test_rust_toolchain_toml_missing_channel() {
                let dir = TempDir::new().unwrap();
                let content = r#"
[toolchain]
components = ["rustfmt"]
"#;
                let path = create_temp_file(&dir, "rust-toolchain.toml", content);

                let versions = parse_rust_toolchain_toml(&path);
                assert!(versions.is_empty());
            }

            #[test]
            fn test_pyproject_toml() {
                let dir = TempDir::new().unwrap();
                let content = r#"
[project]
name = "example"
requires-python = ">=3.8"
"#;
                let path = create_temp_file(&dir, "pyproject.toml", content);

                let versions = parse_pyproject_toml(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, ">=3.8");
                assert!(matches!(
                    versions[0].source,
                    VersionSource::PyprojectRequiresPython
                ));
            }

            #[test]
            fn test_pipfile() {
                let dir = TempDir::new().unwrap();
                let content = r#"
[requires]
python_version = "3.9"
"#;
                let path = create_temp_file(&dir, "Pipfile", content);

                let versions = parse_pipfile(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, "3.9");
                assert!(matches!(
                    versions[0].source,
                    VersionSource::PipfilePythonVersion
                ));
            }

            #[test]
            fn test_invalid_toml() {
                let dir = TempDir::new().unwrap();
                let path = create_temp_file(&dir, "Cargo.toml", "invalid toml content {{");

                let versions = parse_cargo_toml_rust_version(&path);
                assert!(versions.is_empty());
            }

            #[test]
            fn test_toml_non_string_value() {
                let dir = TempDir::new().unwrap();
                let content = r#"
[package]
rust-version = 123
"#;
                let path = create_temp_file(&dir, "Cargo.toml", content);

                let versions = parse_cargo_toml_rust_version(&path);
                assert!(versions.is_empty());
            }
        }

        mod setup_py {
            use super::*;

            #[test]
            fn test_setup_py_with_double_quotes() {
                let dir = TempDir::new().unwrap();
                let content = r#"
from setuptools import setup

setup(
    name="example",
    python_requires=">=3.8",
)
"#;
                let path = create_temp_file(&dir, "setup.py", content);

                let versions = parse_setup_py(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, ">=3.8");
                assert!(matches!(
                    versions[0].source,
                    VersionSource::SetupPyPythonRequires
                ));
            }

            #[test]
            fn test_setup_py_with_single_quotes() {
                let dir = TempDir::new().unwrap();
                let content = r#"
setup(
    python_requires='>=3.9',
)
"#;
                let path = create_temp_file(&dir, "setup.py", content);

                let versions = parse_setup_py(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, ">=3.9");
            }

            #[test]
            fn test_setup_py_with_whitespace() {
                let dir = TempDir::new().unwrap();
                let content = r#"
setup(
    python_requires  =  ">=3.10"  ,
)
"#;
                let path = create_temp_file(&dir, "setup.py", content);

                let versions = parse_setup_py(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, ">=3.10");
            }

            #[test]
            fn test_setup_py_missing_python_requires() {
                let dir = TempDir::new().unwrap();
                let content = r#"
setup(
    name="example",
)
"#;
                let path = create_temp_file(&dir, "setup.py", content);

                let versions = parse_setup_py(&path);
                assert!(versions.is_empty());
            }

            #[test]
            fn test_setup_py_complex_version() {
                let dir = TempDir::new().unwrap();
                let content = r#"
setup(
    python_requires=">=3.8,<4.0",
)
"#;
                let path = create_temp_file(&dir, "setup.py", content);

                let versions = parse_setup_py(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, ">=3.8,<4.0");
            }
        }

        mod package_json {
            use super::*;

            #[test]
            fn test_package_json_with_node_engine() {
                let dir = TempDir::new().unwrap();
                let content = r#"
{
  "name": "example",
  "engines": {
    "node": ">=18.0.0"
  }
}
"#;
                let path = create_temp_file(&dir, "package.json", content);

                let versions = parse_package_json(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, ">=18.0.0");
                assert!(matches!(
                    versions[0].source,
                    VersionSource::PackageJsonEnginesNode
                ));
            }

            #[test]
            fn test_package_json_with_bun_engine() {
                let dir = TempDir::new().unwrap();
                let content = r#"
{
  "engines": {
    "bun": "^1.0.0"
  }
}
"#;
                let path = create_temp_file(&dir, "package.json", content);

                let versions = parse_package_json(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, "^1.0.0");
                assert!(matches!(
                    versions[0].source,
                    VersionSource::PackageJsonEnginesBun
                ));
            }

            #[test]
            fn test_package_json_with_both_engines() {
                let dir = TempDir::new().unwrap();
                let content = r#"
{
  "engines": {
    "node": ">=18.0.0",
    "bun": "^1.0.0"
  }
}
"#;
                let path = create_temp_file(&dir, "package.json", content);

                let versions = parse_package_json(&path);
                assert_eq!(versions.len(), 2);

                let node_version = versions
                    .iter()
                    .find(|v| matches!(v.source, VersionSource::PackageJsonEnginesNode))
                    .unwrap();
                assert_eq!(node_version.raw, ">=18.0.0");

                let bun_version = versions
                    .iter()
                    .find(|v| matches!(v.source, VersionSource::PackageJsonEnginesBun))
                    .unwrap();
                assert_eq!(bun_version.raw, "^1.0.0");
            }

            #[test]
            fn test_package_json_with_or_expression() {
                let dir = TempDir::new().unwrap();
                let content = r#"
{
  "engines": {
    "node": ">=16.0.0 || >=18.0.0"
  }
}
"#;
                let path = create_temp_file(&dir, "package.json", content);

                let versions = parse_package_json(&path);
                assert_eq!(versions.len(), 1);
                assert_eq!(versions[0].raw, ">=16.0.0 || >=18.0.0");
                assert_eq!(versions[0].parsed.as_ref().unwrap().major, Some(18));
            }

            #[test]
            fn test_package_json_no_engines() {
                let dir = TempDir::new().unwrap();
                let content = r#"
{
  "name": "example"
}
"#;
                let path = create_temp_file(&dir, "package.json", content);

                let versions = parse_package_json(&path);
                assert!(versions.is_empty());
            }

            #[test]
            fn test_package_json_invalid_json() {
                let dir = TempDir::new().unwrap();
                let path = create_temp_file(&dir, "package.json", "{ invalid json");

                let versions = parse_package_json(&path);
                assert!(versions.is_empty());
            }

            #[test]
            fn test_package_json_empty_engines() {
                let dir = TempDir::new().unwrap();
                let content = r#"
{
  "engines": {}
}
"#;
                let path = create_temp_file(&dir, "package.json", content);

                let versions = parse_package_json(&path);
                assert!(versions.is_empty());
            }
        }
    }

    mod integration {
        use super::*;
        use std::fs;
        use tempfile::TempDir;

        fn create_temp_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
            let path = dir.path().join(name);
            let mut file = fs::File::create(&path).unwrap();
            file.write_all(content.as_bytes()).unwrap();
            path
        }

        #[test]
        fn test_signal_conversion_go_mod() {
            let dir = TempDir::new().unwrap();
            let path = create_temp_file(&dir, "go.mod", "module test\n\ngo 1.21\n");

            let signal = LanguageDetectionSignal::Strong {
                path,
                source: LanguageDetectionSource::GoMod,
            };

            let result = Vec::<VersionInfo>::try_from(&signal);
            assert!(result.is_ok());
            let versions = result.unwrap();
            assert_eq!(versions.len(), 1);
            assert_eq!(versions[0].raw, "1.21");
        }

        #[test]
        fn test_signal_conversion_weak_signal() {
            let signal = LanguageDetectionSignal::Weak(LanguageDetectionSource::GoFile);
            let result = Vec::<VersionInfo>::try_from(&signal);
            assert!(result.is_err());
        }

        #[test]
        fn test_signal_conversion_empty_file() {
            let dir = TempDir::new().unwrap();
            let path = create_temp_file(&dir, "go.mod", "module test\n");

            let signal = LanguageDetectionSignal::Strong {
                path,
                source: LanguageDetectionSource::GoMod,
            };

            let result = Vec::<VersionInfo>::try_from(&signal);
            assert!(result.is_err());
        }

        #[test]
        fn test_signal_conversion_unsupported_source() {
            let dir = TempDir::new().unwrap();
            let path = create_temp_file(&dir, "go.sum", "");

            let signal = LanguageDetectionSignal::Strong {
                path,
                source: LanguageDetectionSource::GoSum,
            };

            let result = Vec::<VersionInfo>::try_from(&signal);
            assert!(result.is_err());
        }

        #[test]
        fn test_version_detection_from_language_detection() {
            let dir = TempDir::new().unwrap();
            let go_mod_path = create_temp_file(&dir, "go.mod", "module test\n\ngo 1.21\n");
            let go_version_path = create_temp_file(&dir, ".go-version", "1.20.0\n");

            let lang_detection = LanguageDetection::new(
                Language::Go,
                vec![
                    LanguageDetectionSignal::Strong {
                        path: go_mod_path,
                        source: LanguageDetectionSource::GoMod,
                    },
                    LanguageDetectionSignal::Strong {
                        path: go_version_path,
                        source: LanguageDetectionSource::GoVersionFile,
                    },
                ],
            );

            let version_detection = VersionDetection::try_from(&lang_detection);
            assert!(version_detection.is_ok());

            let detection = version_detection.unwrap();
            assert!(matches!(detection.language, Language::Go));
            assert_eq!(detection.versions.len(), 2);

            let go_mod_version = detection
                .versions
                .iter()
                .find(|v| matches!(v.source, VersionSource::GoModDirective))
                .unwrap();
            assert_eq!(go_mod_version.raw, "1.21");

            let go_version = detection
                .versions
                .iter()
                .find(|v| matches!(v.source, VersionSource::GoVersionFile))
                .unwrap();
            assert_eq!(go_version.raw, "1.20.0");
        }

        #[test]
        fn test_version_detection_no_versions() {
            let lang_detection = LanguageDetection::new(
                Language::Go,
                vec![LanguageDetectionSignal::Weak(
                    LanguageDetectionSource::GoFile,
                )],
            );

            let version_detection = VersionDetection::try_from(&lang_detection);
            assert!(version_detection.is_err());
        }

        #[test]
        fn test_version_detection_mixed_valid_invalid() {
            let dir = TempDir::new().unwrap();
            let valid_path = create_temp_file(&dir, "go.mod", "module test\n\ngo 1.21\n");
            let invalid_path = create_temp_file(&dir, ".go-version", "");

            let lang_detection = LanguageDetection::new(
                Language::Go,
                vec![
                    LanguageDetectionSignal::Strong {
                        path: valid_path,
                        source: LanguageDetectionSource::GoMod,
                    },
                    LanguageDetectionSignal::Strong {
                        path: invalid_path,
                        source: LanguageDetectionSource::GoVersionFile,
                    },
                ],
            );

            let version_detection = VersionDetection::try_from(&lang_detection);
            assert!(version_detection.is_ok());

            let detection = version_detection.unwrap();
            assert_eq!(detection.versions.len(), 1);
            assert_eq!(detection.versions[0].raw, "1.21");
        }

        #[test]
        fn test_package_json_multiple_versions() {
            let dir = TempDir::new().unwrap();
            let content = r#"
{
  "engines": {
    "node": ">=18.0.0",
    "bun": "^1.0.0"
  }
}
"#;
            let path = create_temp_file(&dir, "package.json", content);

            let signal = LanguageDetectionSignal::Strong {
                path,
                source: LanguageDetectionSource::PackageJson,
            };

            let result = Vec::<VersionInfo>::try_from(&signal);
            assert!(result.is_ok());
            let versions = result.unwrap();
            assert_eq!(versions.len(), 2);
        }
    }
}
