use serde::Serialize;
pub use std::path::Path;
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
};

pub mod language;
pub mod version;
pub mod package_manager;

pub use language::*;
pub use version::*;
pub use package_manager::*;

#[derive(Debug, Serialize)]
pub struct ProjectMetadata {
    pub languages: Vec<LanguageDetection>,
    pub versions: Vec<VersionDetection>,
    pub package_managers: Vec<PackageManagerDetection>,
}

#[derive(Default)]
pub struct DetectionEngine;

impl DetectionEngine {
    pub fn detect(&self, path: &Path) -> ProjectMetadata {
        let languages: Vec<LanguageDetection> =
            DirectoryIterator(VecDeque::from([path.to_path_buf()]))
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
        }
    }
}

struct DirectoryIterator(VecDeque<PathBuf>);

impl Iterator for DirectoryIterator {
    type Item = PathBuf;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop_front().inspect(|p| {
            if p.is_dir()
                && let Ok(entries) = p.read_dir()
            {
                entries
                    .filter_map(|entry| entry.ok())
                    .for_each(|entry| self.0.push_back(entry.path()));
            }
        })
    }
}
