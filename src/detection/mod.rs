use serde::Serialize;
pub use std::path::Path;
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
};

mod language;

pub use language::*;

#[derive(Debug, Serialize)]
pub struct ProjectMetadata {
    languages: Vec<LanguageDetection>,
}

pub struct DetectionEngine;

impl DetectionEngine {
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

        ProjectMetadata { languages }
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
