use serde::Serialize;
pub use std::path::Path;
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
};

pub mod language;
pub mod package_manager;
pub mod task_runner;
pub mod version;

pub use language::*;
pub use package_manager::*;
pub use task_runner::*;
pub use version::*;

#[derive(Debug, Serialize)]
pub struct ProjectMetadata {
    pub languages: Vec<LanguageDetection>,
    pub versions: Vec<VersionDetection>,
    pub package_managers: Vec<PackageManagerDetection>,
    pub task_runners: Vec<TaskRunnerDetection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetectionScope {
    #[default]
    All,
    Root,
}

#[derive(Default)]
pub struct DetectionEngine;

impl DetectionEngine {
    pub fn detect(&self, path: &Path) -> ProjectMetadata {
        self.detect_with_scope(path, DetectionScope::All)
    }

    pub fn detect_with_scope(&self, path: &Path, scope: DetectionScope) -> ProjectMetadata {
        let paths: Vec<PathBuf> = DirectoryIterator::new(path.to_path_buf(), scope).collect();

        let languages: Vec<LanguageDetection> = paths
            .iter()
            .filter_map(|path| LanguageDetectionSignal::try_from(path.clone()).ok())
            .fold(
                HashMap::<Language, Vec<LanguageDetectionSignal>>::new(),
                |mut acc, signal| {
                    let lang = (&signal).into();
                    acc.entry(lang).or_default().push(signal);
                    acc
                },
            )
            .into_iter()
            .map(|(language, sources)| LanguageDetection::new(language, sources))
            .collect();

        let task_runners: Vec<TaskRunnerDetection> = paths
            .iter()
            .filter_map(|path| TaskRunnerFile::try_from(path.clone()).ok())
            .map(TaskRunnerDetection::from)
            .collect();

        let versions = languages
            .iter()
            .filter_map(|lang| VersionDetection::try_from(lang).ok())
            .collect();

        let package_managers = languages
            .iter()
            .filter_map(|lang| PackageManagerDetection::try_from(lang).ok())
            .collect();

        ProjectMetadata {
            languages,
            versions,
            package_managers,
            task_runners,
        }
    }
}

const IGNORED_DIR_BASENAMES: &[&str] = &[
    // VCS
    ".git",
    ".hg",
    ".svn",
    // Node
    "node_modules",
    ".yarn",
    ".pnpm-store",
    ".turbo",
    ".nx",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".parcel-cache",
    // Rust
    "target",
    // Python
    ".venv",
    "venv",
    "env",
    "__pycache__",
    ".tox",
    ".nox",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    // General build/artifacts
    "dist",
    "build",
    "out",
    "coverage",
    ".cache",
    ".direnv",
    ".idea",
    ".vscode",
    "vendor",
    ".terraform",
];

struct DirectoryIterator {
    queue: VecDeque<PathBuf>,
    root: PathBuf,
    scope: DetectionScope,
}

impl DirectoryIterator {
    fn new(root: PathBuf, scope: DetectionScope) -> Self {
        Self {
            queue: VecDeque::from([root.clone()]),
            root,
            scope,
        }
    }

    fn should_ignore_dir(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| IGNORED_DIR_BASENAMES.contains(&name))
            .unwrap_or(false)
    }
}

impl Iterator for DirectoryIterator {
    type Item = PathBuf;
    fn next(&mut self) -> Option<Self::Item> {
        self.queue.pop_front().inspect(|p| match self.scope {
            DetectionScope::All => {
                if p.is_dir()
                    && !Self::should_ignore_dir(p)
                    && let Ok(entries) = p.read_dir()
                {
                    entries
                        .filter_map(|entry| entry.ok())
                        .for_each(|entry| self.queue.push_back(entry.path()));
                }
            }
            DetectionScope::Root => {
                if p == &self.root
                    && p.is_dir()
                    && !Self::should_ignore_dir(p)
                    && let Ok(entries) = p.read_dir()
                {
                    entries
                        .filter_map(|entry| entry.ok())
                        .for_each(|entry| self.queue.push_back(entry.path()));
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_temp_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn test_directory_iterator_empty() {
        let dir = TempDir::new().unwrap();
        let iterator = DirectoryIterator::new(dir.path().to_path_buf(), DetectionScope::All);
        let paths: Vec<PathBuf> = iterator.collect();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], dir.path());
    }

    #[test]
    fn test_directory_iterator_with_files() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, "file1.txt", "content");
        create_temp_file(&dir, "file2.rs", "fn main() {}");

        let iterator = DirectoryIterator::new(dir.path().to_path_buf(), DetectionScope::All);
        let paths: Vec<PathBuf> = iterator.collect();

        assert!(paths.len() >= 3);
        assert!(paths.contains(&dir.path().to_path_buf()));
    }

    #[test]
    fn test_directory_iterator_with_subdirectories() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();
        create_temp_file(&dir, "file1.txt", "content");
        create_temp_file(&dir, "subdir/file2.txt", "content");

        let iterator = DirectoryIterator::new(dir.path().to_path_buf(), DetectionScope::All);
        let paths: Vec<PathBuf> = iterator.collect();

        assert!(paths.iter().any(|p| p.ends_with("subdir")));
        assert!(paths.iter().any(|p| p.ends_with("file1.txt")));
        assert!(paths.iter().any(|p| p.ends_with("file2.txt")));
    }

    #[test]
    fn test_directory_iterator_nonexistent_path() {
        let nonexistent = PathBuf::from("/nonexistent/path/that/does/not/exist");
        let iterator = DirectoryIterator::new(nonexistent.clone(), DetectionScope::All);
        let paths: Vec<PathBuf> = iterator.collect();

        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], nonexistent);
    }

    #[test]
    fn test_detection_engine_default() {
        let engine = DetectionEngine;
        let dir = TempDir::new().unwrap();
        let metadata = engine.detect(dir.path());

        assert!(metadata.languages.is_empty());
        assert!(metadata.versions.is_empty());
        assert!(metadata.package_managers.is_empty());
    }

    #[test]
    fn test_detection_engine_detect_rust_project() {
        let dir = TempDir::new().unwrap();
        create_temp_file(
            &dir,
            "Cargo.toml",
            r#"
[package]
name = "test"
version = "0.1.0"
rust-version = "1.70.0"
"#,
        );
        create_temp_file(&dir, "Cargo.lock", "");
        create_temp_file(&dir, "src/main.rs", "fn main() {}");

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());

        assert_eq!(metadata.languages.len(), 1);
        assert!(matches!(metadata.languages[0].language, Language::Rust));
        assert_eq!(metadata.versions.len(), 1);
        assert_eq!(metadata.package_managers.len(), 1);
        assert!(matches!(
            metadata.package_managers[0].package_managers[0].package_manager,
            crate::detection::package_manager::PackageManager::Cargo
        ));
    }

    #[test]
    fn test_detection_engine_detect_go_project() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, "go.mod", "module example.com\n\ngo 1.21\n");
        create_temp_file(&dir, "go.sum", "");
        create_temp_file(&dir, "main.go", "package main\n\nfunc main() {}");

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());

        assert_eq!(metadata.languages.len(), 1);
        assert!(matches!(metadata.languages[0].language, Language::Go));
        assert_eq!(metadata.versions.len(), 1);
        assert_eq!(metadata.package_managers.len(), 1);
    }

    #[test]
    fn test_detection_engine_detect_python_project() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, "requirements.txt", "requests==2.28.0");
        create_temp_file(&dir, ".python-version", "3.11.0");
        create_temp_file(&dir, "main.py", "print('hello')");

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());

        assert_eq!(metadata.languages.len(), 1);
        assert!(matches!(metadata.languages[0].language, Language::Python));
        assert_eq!(metadata.versions.len(), 1);
        assert_eq!(metadata.package_managers.len(), 1);
    }

    #[test]
    fn test_detection_engine_detect_javascript_project() {
        let dir = TempDir::new().unwrap();
        create_temp_file(
            &dir,
            "package.json",
            r#"{
  "name": "test",
  "packageManager": "pnpm@9.0.0",
  "engines": {
    "node": ">=18.0.0"
  }
}"#,
        );
        create_temp_file(&dir, "pnpm-lock.yaml", "");
        create_temp_file(&dir, "index.js", "console.log('hello')");

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());

        assert_eq!(metadata.languages.len(), 1);
        assert!(matches!(
            metadata.languages[0].language,
            Language::JavaScript
        ));
        assert_eq!(metadata.versions.len(), 1);
        assert_eq!(metadata.package_managers.len(), 1);
    }

    #[test]
    fn test_detection_engine_detect_multi_language_project() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, "Cargo.toml", "[package]\nname = \"test\"");
        create_temp_file(&dir, "package.json", "{}");
        create_temp_file(&dir, "src/main.rs", "fn main() {}");
        create_temp_file(&dir, "index.js", "console.log('test')");

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());

        assert_eq!(metadata.languages.len(), 2);
        let langs: Vec<_> = metadata.languages.iter().map(|l| &l.language).collect();
        assert!(langs.contains(&&Language::Rust));
        assert!(langs.contains(&&Language::JavaScript));
    }

    #[test]
    fn test_detection_engine_detect_with_nested_directories() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join("backend")).unwrap();
        fs::create_dir(dir.path().join("frontend")).unwrap();

        create_temp_file(&dir, "backend/go.mod", "module example.com\n\ngo 1.21\n");
        create_temp_file(&dir, "backend/main.go", "package main");
        create_temp_file(&dir, "frontend/package.json", "{}");
        create_temp_file(&dir, "frontend/index.ts", "console.log('test')");

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());

        assert_eq!(metadata.languages.len(), 2);
    }

    #[test]
    fn test_detection_engine_root_scope_ignores_nested_directories() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join("backend")).unwrap();

        create_temp_file(&dir, "backend/go.mod", "module example.com\n\ngo 1.21\n");
        create_temp_file(&dir, "backend/main.go", "package main");

        let engine = DetectionEngine;
        let metadata = engine.detect_with_scope(dir.path(), DetectionScope::Root);

        assert!(metadata.languages.is_empty());
        assert!(metadata.versions.is_empty());
        assert!(metadata.package_managers.is_empty());
        assert!(metadata.task_runners.is_empty());
    }

    #[test]
    fn test_detection_engine_root_scope_detects_root_files_only() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join("frontend")).unwrap();

        create_temp_file(&dir, "Cargo.toml", "[package]\nname = \"test\"\n");
        create_temp_file(&dir, "frontend/package.json", "{}");

        let engine = DetectionEngine;
        let metadata = engine.detect_with_scope(dir.path(), DetectionScope::Root);

        assert_eq!(metadata.languages.len(), 1);
        assert!(matches!(metadata.languages[0].language, Language::Rust));
    }

    #[test]
    fn test_project_metadata_serialization() {
        let metadata = ProjectMetadata {
            languages: vec![],
            versions: vec![],
            package_managers: vec![],
            task_runners: vec![],
        };

        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("languages"));
        assert!(json.contains("versions"));
        assert!(json.contains("package_managers"));
    }

    #[test]
    fn test_project_metadata_serialization_with_data() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, "go.mod", "module test\n\ngo 1.21\n");

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());
        let json = serde_json::to_string_pretty(&metadata).unwrap();

        assert!(json.contains("Go"));
        assert!(json.contains("1.21"));
    }

    #[test]
    fn test_detection_engine_detect_task_runners() {
        let dir = TempDir::new().unwrap();
        create_temp_file(
            &dir,
            "Makefile",
            "test:\n\tcargo test\n\nbuild:\n\tcargo build",
        );
        create_temp_file(
            &dir,
            "package.json",
            r#"{"scripts": {"test": "jest", "build": "vite build"}}"#,
        );

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());

        assert_eq!(metadata.task_runners.len(), 2);
        assert!(
            metadata
                .task_runners
                .iter()
                .any(|tr| matches!(tr.task_runner, TaskRunner::Make))
        );
        assert!(
            metadata
                .task_runners
                .iter()
                .any(|tr| matches!(tr.task_runner, TaskRunner::NpmScripts))
        );

        // Verify commands were extracted
        let makefile_tr = metadata
            .task_runners
            .iter()
            .find(|tr| matches!(tr.task_runner, TaskRunner::Make))
            .unwrap();
        assert!(!makefile_tr.commands.test.is_empty());
        assert!(!makefile_tr.commands.build.is_empty());
    }

    #[test]
    fn test_traversal_ignores_node_modules() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join("node_modules")).unwrap();
        fs::create_dir(dir.path().join("node_modules/foo")).unwrap();
        create_temp_file(&dir, "node_modules/foo/package.json", r#"{"name": "foo"}"#);

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());

        assert_eq!(metadata.languages.len(), 0);
        assert_eq!(metadata.task_runners.len(), 0);
    }

    #[test]
    fn test_traversal_ignores_common_dirs() {
        let dir = TempDir::new().unwrap();

        fs::create_dir(dir.path().join(".git")).unwrap();
        create_temp_file(&dir, ".git/config", "");

        fs::create_dir(dir.path().join("target")).unwrap();
        create_temp_file(&dir, "target/Cargo.toml", r#"[package]\nname = "test""#);

        fs::create_dir(dir.path().join(".venv")).unwrap();
        create_temp_file(&dir, ".venv/setup.py", "");

        fs::create_dir(dir.path().join("dist")).unwrap();
        create_temp_file(&dir, "dist/package.json", "{}");

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());

        assert_eq!(metadata.languages.len(), 0);
        assert_eq!(metadata.task_runners.len(), 0);
    }
}
