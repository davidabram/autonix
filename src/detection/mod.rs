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

pub struct DetectionEngine;

impl DetectionEngine {
    pub fn new() -> Self {
        DetectionEngine
    }

    pub fn detect(&self, path: &Path) -> ProjectMetadata {
        let languages: Vec<LanguageDetection> = DirectoryIterator(VecDeque::from([path.to_path_buf()]))
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

        let versions = languages
            .iter()
            .filter_map(VersionDetection::from_language_detection)
            .collect();

        ProjectMetadata { languages, versions }
    }
}

struct DirectoryIterator(VecDeque<PathBuf>);

impl Iterator for DirectoryIterator {
    type Item = PathBuf;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop_front().inspect(|p| {
            if p.is_dir() && let Ok(entries) = p.read_dir() {
                entries
                    .filter_map(|entry| entry.ok())
                    .for_each(|entry| self.0.push_back(entry.path()));
            }
        })
    }
}
