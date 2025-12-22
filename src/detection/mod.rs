use serde::Serialize;
pub use std::path::Path;
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
};

pub mod language;
pub mod version;

pub use language::*;
pub use version::*;

#[derive(Debug, Serialize)]
pub struct ProjectMetadata {
    pub languages: Vec<LanguageDetection>,
    pub versions: Vec<VersionDetection>,
}

pub struct DetectionEngine {
    version_detectors: Vec<Box<dyn VersionDetector>>,
}

impl DetectionEngine {
    pub fn new() -> Self {
        let version_detectors: Vec<Box<dyn VersionDetector>> = vec![
            Box::new(version::GoVersionDetector),
            Box::new(version::RustVersionDetector),
            Box::new(version::PythonVersionDetector),
            Box::new(version::JavaScriptVersionDetector),
        ];

        DetectionEngine { version_detectors }
    }

    pub fn detect(&self, path: &Path) -> ProjectMetadata {
        let languages = DirectoryIterator(VecDeque::from([path.to_path_buf()]))
            .filter_map(|path| LanguageDetectionSignal::try_from(path).ok())
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

        let mut versions = Vec::new();
        for lang_detection in &languages {
            for version_detector in &self.version_detectors {
                if let Some(version_detection) = version_detector.detect(lang_detection) {
                    versions.push(version_detection);
                    break;
                }
            }
        }

        ProjectMetadata { languages, versions }
    }
}

struct DirectoryIterator(VecDeque<PathBuf>);

impl Iterator for DirectoryIterator {
    type Item = PathBuf;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop_front().map(|p| {
            if p.is_dir() {
                p.read_dir()
                    .unwrap()
                    .for_each(|p| self.0.push_back(p.unwrap().path()));
            }
            p
        })
    }
}
