pub use std::path::Path;
use serde::Serialize;

mod language;

pub use language::*;

#[derive(Debug, Serialize)]
pub struct ProjectMetadata {
    pub languages: Vec<LanguageDetection>,
}

pub struct DetectionEngine {
    language_detectors: Vec<Box<dyn LanguageDetector>>,
}


impl DetectionEngine {
    pub fn new() -> Self {
        let language_detectors: Vec<Box<dyn LanguageDetector>> = vec![
            Box::new(language::GoDetector),
            Box::new(language::RustDetector),
            Box::new(language::PythonDetector),
            Box::new(language::JavaScriptDetector),
        ];

        DetectionEngine { language_detectors }
    }

    pub fn  detect(&self, path: &Path) -> ProjectMetadata {
        let mut languages = Vec::new();

        for detector in &self.language_detectors {
            if let Some(detection) = detector.detect(path) {
                languages.push(detection);
            }
        }

        ProjectMetadata { languages }
    }
}

