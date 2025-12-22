use serde::Serialize;
use std::path::PathBuf;
use std::fs;
use std::sync::OnceLock;
use super::{Language, LanguageDetection, LanguageDetectionSource};

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

pub trait VersionDetector {
    fn detect(&self, lang_detection: &LanguageDetection) -> Option<VersionDetection>;
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
    let trimmed = raw.trim()
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

    let pre_release = version_str.split('-').nth(1).map(|p| {
        p.split('+').next().unwrap_or(p).to_string()
    });

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

fn parse_go_mod(path: &PathBuf) -> Option<VersionInfo> {
    let content = fs::read_to_string(path).ok()?;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("go ") {
            let version = trimmed.trim_start_matches("go ").trim();
            return Some(VersionInfo {
                raw: version.to_string(),
                parsed: parse_semantic_version(version),
                source: VersionSource::GoModDirective,
                path: path.clone(),
            });
        }
    }
    None
}

fn parse_simple_version_file(path: &PathBuf, source: VersionSource) -> Option<VersionInfo> {
    let content = fs::read_to_string(path).ok()?;
    let version = content.trim();

    if version.is_empty() {
        return None;
    }

    Some(VersionInfo {
        raw: version.to_string(),
        parsed: parse_semantic_version(version),
        source,
        path: path.clone(),
    })
}

fn parse_go_version_file(path: &PathBuf) -> Option<VersionInfo> {
    parse_simple_version_file(path, VersionSource::GoVersionFile)
}

fn parse_rust_toolchain(path: &PathBuf) -> Option<VersionInfo> {
    parse_simple_version_file(path, VersionSource::RustToolchainFile)
}

fn parse_toml_field(path: &PathBuf, field_path: &[&str], source: VersionSource) -> Option<VersionInfo> {
    let content = fs::read_to_string(path).ok()?;
    let parsed: toml::Value = toml::from_str(&content).ok()?;

    let mut current = &parsed;
    for field in field_path {
        current = current.get(field)?;
    }

    let version_str = current.as_str()?;

    Some(VersionInfo {
        raw: version_str.to_string(),
        parsed: parse_semantic_version(version_str),
        source,
        path: path.clone(),
    })
}

fn parse_rust_toolchain_toml(path: &PathBuf) -> Option<VersionInfo> {
    let content = fs::read_to_string(path).ok()?;
    let parsed: toml::Value = toml::from_str(&content).ok()?;

    let channel = parsed.get("toolchain")
        .and_then(|t| t.get("channel"))
        .or_else(|| parsed.get("channel"))
        .and_then(|c| c.as_str())?;

    Some(VersionInfo {
        raw: channel.to_string(),
        parsed: parse_semantic_version(channel),
        source: VersionSource::RustToolchainToml,
        path: path.clone(),
    })
}

fn parse_cargo_toml_rust_version(path: &PathBuf) -> Option<VersionInfo> {
    parse_toml_field(path, &["package", "rust-version"], VersionSource::CargoTomlRustVersion)
}

fn parse_pyproject_toml(path: &PathBuf) -> Option<VersionInfo> {
    parse_toml_field(path, &["project", "requires-python"], VersionSource::PyprojectRequiresPython)
}

fn parse_python_version_file(path: &PathBuf) -> Option<VersionInfo> {
    parse_simple_version_file(path, VersionSource::PythonVersionFile)
}

fn parse_pipfile(path: &PathBuf) -> Option<VersionInfo> {
    parse_toml_field(path, &["requires", "python_version"], VersionSource::PipfilePythonVersion)
}

fn parse_setup_py(path: &PathBuf) -> Option<VersionInfo> {
    static PYTHON_REQUIRES_RE: OnceLock<regex::Regex> = OnceLock::new();
    
    let content = fs::read_to_string(path).ok()?;
    let re = PYTHON_REQUIRES_RE.get_or_init(|| {
        regex::Regex::new(r#"python_requires\s*=\s*["']([^"']+)["']"#)
            .expect("invalid regex pattern")
    });

    if let Some(captures) = re.captures(&content) {
        let version = captures.get(1)?.as_str();
        return Some(VersionInfo {
            raw: version.to_string(),
            parsed: parse_semantic_version(version),
            source: VersionSource::SetupPyPythonRequires,
            path: path.clone(),
        });
    }

    None
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

fn parse_nvmrc(path: &PathBuf) -> Option<VersionInfo> {
    parse_simple_version_file(path, VersionSource::NvmrcFile)
}

fn parse_node_version_file(path: &PathBuf) -> Option<VersionInfo> {
    parse_simple_version_file(path, VersionSource::NodeVersionFile)
}

fn parse_bun_version_file(path: &PathBuf) -> Option<VersionInfo> {
    parse_simple_version_file(path, VersionSource::BunVersionFile)
}

fn single_to_vec<T>(item: Option<T>) -> Option<Vec<T>> {
    item.map(|v| vec![v])
}

fn detect_version_for_language(
    lang_detection: &LanguageDetection,
    expected_language: &Language,
    parser: impl Fn(&LanguageDetectionSource, &PathBuf) -> Option<Vec<VersionInfo>>,
) -> Option<VersionDetection> {
    if std::mem::discriminant(&lang_detection.language) != std::mem::discriminant(expected_language) {
        return None;
    }

    let mut versions = Vec::new();

    for detected_source in &lang_detection.detected_from {
        if let Some(path) = &detected_source.path &&
           let Some(mut version_infos) = parser(&detected_source.source, path) {
            versions.append(&mut version_infos);
        }
    }

    if versions.is_empty() {
        None
    } else {
        Some(VersionDetection {
            language: lang_detection.language.clone(),
            versions,
        })
    }
}

pub struct GoVersionDetector;

impl VersionDetector for GoVersionDetector {
    fn detect(&self, lang_detection: &LanguageDetection) -> Option<VersionDetection> {
        detect_version_for_language(lang_detection, &Language::Go, |source, path| {
            match source {
                LanguageDetectionSource::GoMod => single_to_vec(parse_go_mod(path)),
                LanguageDetectionSource::GoVersionFile => single_to_vec(parse_go_version_file(path)),
                _ => None,
            }
        })
    }
}

pub struct RustVersionDetector;

impl VersionDetector for RustVersionDetector {
    fn detect(&self, lang_detection: &LanguageDetection) -> Option<VersionDetection> {
        detect_version_for_language(lang_detection, &Language::Rust, |source, path| {
            match source {
                LanguageDetectionSource::CargoToml => single_to_vec(parse_cargo_toml_rust_version(path)),
                LanguageDetectionSource::RustToolchain => single_to_vec(parse_rust_toolchain(path)),
                LanguageDetectionSource::RustToolchainToml => single_to_vec(parse_rust_toolchain_toml(path)),
                _ => None,
            }
        })
    }
}

pub struct PythonVersionDetector;

impl VersionDetector for PythonVersionDetector {
    fn detect(&self, lang_detection: &LanguageDetection) -> Option<VersionDetection> {
        detect_version_for_language(lang_detection, &Language::Python, |source, path| {
            match source {
                LanguageDetectionSource::PyprojectToml => single_to_vec(parse_pyproject_toml(path)),
                LanguageDetectionSource::PythonVersionFile => single_to_vec(parse_python_version_file(path)),
                LanguageDetectionSource::Pipfile => single_to_vec(parse_pipfile(path)),
                LanguageDetectionSource::SetupPy => single_to_vec(parse_setup_py(path)),
                _ => None,
            }
        })
    }
}

pub struct JavaScriptVersionDetector;

impl VersionDetector for JavaScriptVersionDetector {
    fn detect(&self, lang_detection: &LanguageDetection) -> Option<VersionDetection> {
        detect_version_for_language(lang_detection, &Language::JavaScript, |source, path| {
            match source {
                LanguageDetectionSource::PackageJson => Some(parse_package_json(path)),
                LanguageDetectionSource::NvmrcFile => single_to_vec(parse_nvmrc(path)),
                LanguageDetectionSource::NodeVersionFile => single_to_vec(parse_node_version_file(path)),
                LanguageDetectionSource::BunVersionFile => single_to_vec(parse_bun_version_file(path)),
                _ => None,
            }
        })
    }
}
