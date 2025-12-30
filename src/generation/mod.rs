pub mod constants;
pub mod dev_flake;
pub mod nix_builder;

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::detection::ProjectMetadata;

pub use dev_flake::generate_dev_flake;
pub use dev_flake::{CheckCategory, CheckFile, GeneratedFlake, LanguagePackages};

pub fn write_dev_flake(metadata: &ProjectMetadata, root: &Path) -> Result<(), std::io::Error> {
    let flake = generate_dev_flake(metadata, root);

    let autonix_dir = root.join(".autonix");
    fs::create_dir_all(&autonix_dir)?;

    fs::write(autonix_dir.join("flake.nix"), flake.main_flake)?;
    fs::write(autonix_dir.join("devShell.nix"), flake.devshell)?;

    if let Some(overlay) = flake.rust_overlay {
        let rust_dir = autonix_dir.join("rust");
        fs::create_dir_all(&rust_dir)?;
        fs::write(rust_dir.join("overlay.nix"), overlay)?;
    }

    for lang_pkg in flake.language_packages {
        let lang_dir = autonix_dir.join(lang_pkg.language.dir_name());
        fs::create_dir_all(&lang_dir)?;
        fs::write(lang_dir.join("packages.nix"), lang_pkg.content)?;
    }

    for check_file in flake.check_files {
        let full_path = autonix_dir.join(&check_file.relative_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(full_path, check_file.content)?;
    }

    Ok(())
}

fn _relative_path(base: &Path, path: &Path) -> Option<PathBuf> {
    path.strip_prefix(base).ok().map(Path::to_path_buf)
}
