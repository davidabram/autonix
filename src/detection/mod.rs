use serde::Serialize;
pub use std::path::Path;

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
    language_detectors: Vec<Box<dyn LanguageDetector>>,
    version_detectors: Vec<Box<dyn VersionDetector>>,
}

impl DetectionEngine {
    pub fn new() -> Self {
        let language_detectors: Vec<Box<dyn LanguageDetector>> = vec![
            Box::new(language::GoDetector),
            Box::new(language::RustDetector),
            Box::new(language::PythonDetector),
            Box::new(language::JavaScriptDetector),
        ];

        let version_detectors: Vec<Box<dyn VersionDetector>> = vec![
            Box::new(version::GoVersionDetector),
            Box::new(version::RustVersionDetector),
            Box::new(version::PythonVersionDetector),
            Box::new(version::JavaScriptVersionDetector),
        ];

        DetectionEngine {
            language_detectors,
            version_detectors,
        }
    }

    pub fn detect(&self, path: &Path) -> ProjectMetadata {
        let mut languages = Vec::new();
        let mut versions = Vec::new();

        for detector in &self.language_detectors {
            if let Some(detection) = detector.detect(path) {
                languages.push(detection);
            }
        }

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
