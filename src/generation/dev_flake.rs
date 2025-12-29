use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value as JsonValue;

use crate::detection::{
    CommandExecutable, Language, PackageManager, ProjectMetadata, SemanticVersion, TaskCommand,
    TaskRunner, VersionConstraint, VersionInfo, VersionSource,
};
use crate::generation::constants;
use crate::generation::nix_builder;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckCategory {
    Test,
    Build,
}

#[derive(Debug, Clone)]
pub struct LanguagePackages {
    pub language: Language,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct CheckFile {
    pub language: Option<Language>,
    pub category: CheckCategory,
    pub content: String,
    pub relative_path: PathBuf,
}

#[derive(Debug)]
pub struct GeneratedFlake {
    pub main_flake: String,
    pub devshell: String,
    pub language_packages: Vec<LanguagePackages>,
    pub check_files: Vec<CheckFile>,
    pub rust_overlay: Option<String>,
}

#[derive(Debug, Clone)]
struct CheckSpec {
    language: Option<Language>,
    category: CheckCategory,
    key: String,
    derivation_name: String,
    display: String,
    required_exec: String,
    command: String,
    workdir: String,
}

#[derive(Debug, Clone)]
struct CommandInfo {
    required_exec: String,
    command: String,
    workdir: String,
    display: String,
}

pub fn generate_dev_flake(metadata: &ProjectMetadata, root: &Path) -> GeneratedFlake {
    let detected_languages = detected_languages(metadata);

    let task_runners: HashSet<TaskRunner> = metadata
        .task_runners
        .iter()
        .map(|tr| tr.task_runner)
        .collect();

    let mut need_node = detected_languages.contains(&Language::JavaScript)
        || task_runners.iter().any(|tr| {
            matches!(
                tr,
                TaskRunner::NpmScripts
                    | TaskRunner::Vite
                    | TaskRunner::Webpack
                    | TaskRunner::Rspack
                    | TaskRunner::Rollup
                    | TaskRunner::Turbo
                    | TaskRunner::Nx
            )
        });

    let mut need_python = detected_languages.contains(&Language::Python)
        || task_runners
            .iter()
            .any(|tr| matches!(tr, TaskRunner::Tox | TaskRunner::Nox | TaskRunner::Invoke));

    let need_go =
        detected_languages.contains(&Language::Go) || task_runners.contains(&TaskRunner::GoTask);
    let need_rust =
        detected_languages.contains(&Language::Rust) || task_runners.contains(&TaskRunner::Cargo);

    let go_version = best_version_info(metadata, Language::Go, constants::GO_VERSION_SOURCES);
    let python_version = best_version_info(
        metadata,
        Language::Python,
        constants::PYTHON_VERSION_SOURCES,
    );
    let node_version = best_version_info(
        metadata,
        Language::JavaScript,
        constants::NODE_VERSION_SOURCES,
    );
    let rust_version = best_version_info(metadata, Language::Rust, constants::RUST_VERSION_SOURCES);

    let go_want_attr = go_version
        .and_then(|v| v.parsed.as_ref())
        .and_then(go_attr_from_version);
    let python_want_attr = python_version
        .and_then(|v| v.parsed.as_ref())
        .and_then(python_attr_from_version);
    let node_want_attr = node_version
        .and_then(|v| v.parsed.as_ref())
        .and_then(node_attr_from_version);

    let rust_want_version = rust_version
        .filter(|v| {
            matches!(
                v.parsed.as_ref().map(|p| p.constraint),
                Some(VersionConstraint::Exact)
            )
        })
        .and_then(|v| v.parsed.as_ref())
        .and_then(rust_version_string_from_version);

    let mut required_package_managers = detected_package_managers(metadata);

    let checks_by_lang = collect_checks(
        metadata,
        root,
        &mut required_package_managers,
        &mut need_node,
        &mut need_python,
    );

    let required_node_tools = required_node_tools(&task_runners);
    let required_task_runner_tools = required_task_runner_tools(metadata);

    let uses_rust_overlay = need_rust;

    let go_notice = go_notice(go_version, go_want_attr.as_deref());
    let python_notice = python_notice(python_version, python_want_attr.as_deref());
    let node_notice = node_notice(node_version, node_want_attr.as_deref());
    let rust_notice = rust_notice(need_rust, rust_version, rust_want_version.as_deref());

    let mut language_packages = Vec::new();

    if need_go {
        let want_go_attr = go_want_attr.as_deref().unwrap_or("go");
        language_packages.push(LanguagePackages {
            language: Language::Go,
            content: generate_golang_packages_nix(want_go_attr, go_notice.as_deref()),
        });
    }

    if need_python {
        let want_python_attr = python_want_attr.as_deref().unwrap_or("python3");
        language_packages.push(LanguagePackages {
            language: Language::Python,
            content: generate_python_packages_nix(
                want_python_attr,
                python_notice.as_deref(),
                &required_package_managers,
                &required_task_runner_tools,
            ),
        });
    }

    if need_node {
        let want_node_attr = node_want_attr.as_deref().unwrap_or("nodejs");
        language_packages.push(LanguagePackages {
            language: Language::JavaScript,
            content: generate_nodejs_packages_nix(
                want_node_attr,
                node_notice.as_deref(),
                &required_package_managers,
                &required_node_tools,
            ),
        });
    }

    if need_rust {
        language_packages.push(LanguagePackages {
            language: Language::Rust,
            content: generate_rust_packages_nix(
                rust_want_version.as_deref(),
                rust_notice.as_deref(),
            ),
        });
    }

    let rust_overlay = uses_rust_overlay.then(generate_rust_overlay_nix);

    let devshell = generate_devshell_nix();
    let check_files = generate_check_files(&checks_by_lang);

    let main_flake = generate_main_flake(
        need_go,
        need_python,
        need_node,
        need_rust,
        uses_rust_overlay,
        &check_files,
        &required_task_runner_tools,
    );

    GeneratedFlake {
        main_flake,
        devshell,
        language_packages,
        check_files,
        rust_overlay,
    }
}

fn required_node_tools(task_runners: &HashSet<TaskRunner>) -> BTreeSet<&'static str> {
    let mut required: BTreeSet<&'static str> = BTreeSet::new();

    for task_runner in task_runners {
        match task_runner {
            TaskRunner::Vite => {
                required.insert(constants::NODE_TOOL_VITE);
            }
            TaskRunner::Webpack => {
                required.insert(constants::NODE_TOOL_WEBPACK);
            }
            TaskRunner::Rspack => {
                required.insert(constants::NODE_TOOL_RSPACK);
            }
            TaskRunner::Rollup => {
                required.insert(constants::NODE_TOOL_ROLLUP);
            }
            TaskRunner::Turbo => {
                required.insert(constants::NODE_TOOL_TURBO);
            }
            TaskRunner::Nx => {
                required.insert(constants::NODE_TOOL_NX);
            }
            TaskRunner::NpmScripts
            | TaskRunner::Make
            | TaskRunner::Just
            | TaskRunner::Task
            | TaskRunner::Tox
            | TaskRunner::Nox
            | TaskRunner::Invoke
            | TaskRunner::Cargo
            | TaskRunner::GoTask => {}
        }
    }

    required
}

fn required_task_runner_tools(metadata: &ProjectMetadata) -> BTreeSet<&'static str> {
    let mut required: BTreeSet<&'static str> = BTreeSet::new();

    for task_runner in metadata.task_runners.iter().map(|t| t.task_runner) {
        match task_runner {
            TaskRunner::Make => {
                required.insert(constants::GENERIC_TOOL_GNUMAKE);
            }
            TaskRunner::Just => {
                required.insert(constants::GENERIC_TOOL_JUST);
            }
            TaskRunner::Task => {
                required.insert(constants::GENERIC_TOOL_GO_TASK);
            }
            TaskRunner::Tox => {
                required.insert(constants::PYTHON_TOOL_TOX);
            }
            TaskRunner::Nox => {
                required.insert(constants::PYTHON_TOOL_NOX);
            }
            TaskRunner::Invoke => {
                required.insert(constants::PYTHON_TOOL_INVOKE);
            }
            TaskRunner::GoTask | TaskRunner::Cargo | TaskRunner::NpmScripts => {}
            TaskRunner::Vite
            | TaskRunner::Webpack
            | TaskRunner::Rspack
            | TaskRunner::Rollup
            | TaskRunner::Turbo
            | TaskRunner::Nx => {}
        }
    }

    required
}

fn generate_version_notice(
    language_name: &str,
    version_info: Option<&VersionInfo>,
    selected_attr: Option<&str>,
    default_fallback: &str,
    note: Option<&str>,
) -> Option<String> {
    let version = version_info?;
    let _parsed = version.parsed.as_ref()?;

    let requested = format!("{} (from {:?})", version.raw, version.source);
    let selected = selected_attr.unwrap_or(default_fallback);
    let note = note.unwrap_or("");

    Some(
        format!("{language_name}: requested {requested} -> want {selected} {note}")
            .trim()
            .to_string(),
    )
}

fn go_notice(go_version: Option<&VersionInfo>, go_want_attr: Option<&str>) -> Option<String> {
    let patch_note = go_version
        .and_then(|v| v.parsed.as_ref())
        .filter(|p| p.patch.is_some())
        .map(|_| "note: nixpkgs provides Go by major/minor (patch may differ)");

    generate_version_notice(
        "Go",
        go_version,
        go_want_attr,
        "go (unversioned; go_* not inferred)",
        patch_note,
    )
}

fn python_notice(
    python_version: Option<&VersionInfo>,
    python_want_attr: Option<&str>,
) -> Option<String> {
    let patch_note = python_version
        .and_then(|v| v.parsed.as_ref())
        .filter(|p| p.patch.is_some() || !matches!(p.constraint, VersionConstraint::Exact))
        .map(|_| "note: nixpkgs provides Python by major/minor (patch may differ)");

    generate_version_notice(
        "Python",
        python_version,
        python_want_attr,
        "python3 (unversioned; pythonXY not inferred)",
        patch_note,
    )
}

fn node_notice(node_version: Option<&VersionInfo>, node_want_attr: Option<&str>) -> Option<String> {
    let patch_note = node_version
        .and_then(|v| v.parsed.as_ref())
        .filter(|p| p.minor.is_some() || p.patch.is_some())
        .map(|_| "note: nixpkgs provides Node.js by major (minor/patch may differ)");

    generate_version_notice(
        "Node",
        node_version,
        node_want_attr,
        "nodejs (unversioned; nodejs_* not inferred)",
        patch_note,
    )
}

fn rust_notice(
    need_rust: bool,
    rust_version: Option<&VersionInfo>,
    rust_want_version: Option<&str>,
) -> Option<String> {
    if !need_rust {
        return None;
    }

    if let Some(v) = rust_want_version {
        return Some(format!(
            "Rust: requested {} -> try rust-bin.stable.{v} (fallback latest)",
            rust_version
                .map(|vi| format!("{} (from {:?})", vi.raw, vi.source))
                .unwrap_or_else(|| "(unknown)".to_string())
        ));
    }

    if let Some(vi) = rust_version {
        return Some(format!(
            "Rust: detected {} (from {:?}) -> using rust-bin.stable.latest (not exact pin)",
            vi.raw, vi.source
        ));
    }

    None
}

fn generate_main_flake_header() -> String {
    let mut out = String::new();

    out.push_str("# Generated by autonix\n");
    out.push_str("#\n");
    out.push_str("# This flake uses a multi-file structure:\n");
    out.push_str("#   .autonix/devShell.nix              - Development shell\n");
    out.push_str("#   .autonix/{language}/packages.nix   - Language toolchains and tools\n");
    out.push_str("#   .autonix/{language}/*-checks.nix   - Build and test checks\n");
    out.push_str("#\n");

    out
}

fn generate_file_header(description: &str) -> String {
    format!("# Generated by autonix\n# {description}\n")
}

fn generate_flake_inputs(uses_rust_overlay: bool) -> String {
    let mut out = String::new();

    out.push_str("  inputs = {\n");
    out.push_str("    nixpkgs.url = \"github:NixOS/nixpkgs/nixos-unstable\";\n");
    out.push_str("    flake-utils.url = \"github:numtide/flake-utils\";\n");

    if uses_rust_overlay {
        out.push_str("    rust-overlay = {\n");
        out.push_str("      url = \"github:oxalica/rust-overlay\";\n");
        out.push_str("      inputs.nixpkgs.follows = \"nixpkgs\";\n");
        out.push_str("    };\n");
    }

    out.push_str("  };\n\n");

    out
}

fn generate_flake_let_bindings(
    uses_rust_overlay: bool,
    need_go: bool,
    need_python: bool,
    need_node: bool,
    need_rust: bool,
    required_task_runner_tools: &BTreeSet<&'static str>,
) -> String {
    let mut out = String::new();

    out.push_str("      let\n");

    if uses_rust_overlay {
        out.push_str("        overlays = [ (import .autonix/rust/overlay.nix { inherit rust-overlay; }) ];\n");
        out.push_str("        pkgs = import nixpkgs { inherit system overlays; };\n");
    } else {
        out.push_str("        pkgs = import nixpkgs { inherit system; };\n");
    }
    out.push_str("        lib = pkgs.lib;\n\n");

    if need_go {
        out.push_str(
            "        golangPackages = import .autonix/golang/packages.nix { inherit pkgs lib; };\n",
        );
    }
    if need_python {
        out.push_str(
            "        pythonPackages = import .autonix/python/packages.nix { inherit pkgs lib; };\n",
        );
    }
    if need_node {
        out.push_str(
            "        nodejsPackages = import .autonix/nodejs/packages.nix { inherit pkgs lib; };\n",
        );
    }
    if need_rust {
        out.push_str(
            "        rustPackages = import .autonix/rust/packages.nix { inherit pkgs lib; };\n",
        );
    }
    if need_go || need_python || need_node || need_rust {
        out.push('\n');
    }

    let generic_packages = generic_task_runner_packages(required_task_runner_tools);
    let include_generic_packages = !generic_packages.is_empty();

    if include_generic_packages {
        out.push_str("        genericPackages = [\n");
        for pkg in generic_packages {
            writeln!(out, "          pkgs.{pkg}").unwrap();
        }
        out.push_str("        ];\n\n");
    }

    let mut dev_sources: Vec<&str> = Vec::new();
    if include_generic_packages {
        dev_sources.push("genericPackages");
    }
    if need_go {
        dev_sources.push("golangPackages.packages");
    }
    if need_python {
        dev_sources.push("pythonPackages.packages");
    }
    if need_node {
        dev_sources.push("nodejsPackages.packages");
    }
    if need_rust {
        dev_sources.push("rustPackages.packages");
    }

    out.push_str("        devPackages = []");
    for src in dev_sources {
        write!(out, "\n          ++ {src}").unwrap();
    }
    out.push_str(";\n\n");

    let mut notice_sources: Vec<&str> = Vec::new();
    if need_go {
        notice_sources.push("golangPackages.notices");
    }
    if need_python {
        notice_sources.push("pythonPackages.notices");
    }
    if need_node {
        notice_sources.push("nodejsPackages.notices");
    }
    if need_rust {
        notice_sources.push("rustPackages.notices");
    }

    out.push_str("        notices = []");
    for src in notice_sources {
        write!(out, "\n          ++ {src}").unwrap();
    }
    out.push_str(";\n\n");

    out
}

fn generate_devshell_binding(
    need_go: bool,
    need_python: bool,
    need_node: bool,
    need_rust: bool,
) -> String {
    let mut out = String::new();

    out.push_str("        devShells.default = import .autonix/devShell.nix {\n");
    out.push_str("          inherit pkgs lib devPackages notices;\n");

    if need_go {
        out.push_str("          go = golangPackages.go or null;\n");
        out.push_str("          goAttr = golangPackages.goAttr or null;\n");
        out.push_str("          wantGoAttr = golangPackages.wantGoAttr or null;\n");
    }
    if need_python {
        out.push_str("          python = pythonPackages.python or null;\n");
        out.push_str("          pythonAttr = pythonPackages.pythonAttr or null;\n");
        out.push_str("          wantPythonAttr = pythonPackages.wantPythonAttr or null;\n");
    }
    if need_node {
        out.push_str("          node = nodejsPackages.node or null;\n");
        out.push_str("          nodeAttr = nodejsPackages.nodeAttr or null;\n");
        out.push_str("          wantNodeAttr = nodejsPackages.wantNodeAttr or null;\n");
    }
    if need_rust {
        out.push_str("          rustToolchain = rustPackages.rustToolchain or null;\n");
    }

    out.push_str("        };\n\n");

    out
}

fn generate_checks_binding(check_files: &[CheckFile]) -> String {
    let mut out = String::new();

    if check_files.is_empty() {
        out.push_str("        checks = {};\n");
    } else {
        out.push_str("        checks = {}\n");
        for file in check_files {
            let path = file.relative_path.to_string_lossy();
            writeln!(
                out,
                "          // (import .autonix/{path} {{ inherit pkgs lib devPackages projectRoot; }})"
            )
            .unwrap();
        }
        out.push_str("          ;\n");
    }

    out
}

fn generate_main_flake(
    need_go: bool,
    need_python: bool,
    need_node: bool,
    need_rust: bool,
    uses_rust_overlay: bool,
    check_files: &[CheckFile],
    required_task_runner_tools: &BTreeSet<&'static str>,
) -> String {
    let mut out = String::new();

    out.push_str(&generate_main_flake_header());

    out.push_str("{\n");
    out.push_str("  description = \"Generated by autonix (devShells.default + checks)\";\n\n");

    out.push_str(&generate_flake_inputs(uses_rust_overlay));

    out.push_str("  outputs = { self, nixpkgs, flake-utils");
    if uses_rust_overlay {
        out.push_str(", rust-overlay");
    }
    out.push_str(" }:\n");
    out.push_str("    flake-utils.lib.eachDefaultSystem (system:\n");

    out.push_str(&generate_flake_let_bindings(
        uses_rust_overlay,
        need_go,
        need_python,
        need_node,
        need_rust,
        required_task_runner_tools,
    ));

    if !check_files.is_empty() {
        out.push_str("        projectRoot = ./.;\n\n");
    }

    out.push_str("      in\n");
    out.push_str("      {\n");

    out.push_str(&generate_devshell_binding(
        need_go,
        need_python,
        need_node,
        need_rust,
    ));
    out.push_str(&generate_checks_binding(check_files));

    out.push_str("      });\n");
    out.push_str("}\n");

    out
}

fn generic_task_runner_packages(
    required_task_runner_tools: &BTreeSet<&'static str>,
) -> Vec<&'static str> {
    required_task_runner_tools
        .iter()
        .copied()
        .filter(|&tool| {
            matches!(
                tool,
                constants::GENERIC_TOOL_GNUMAKE
                    | constants::GENERIC_TOOL_JUST
                    | constants::GENERIC_TOOL_GO_TASK
            )
        })
        .collect()
}

fn generate_devshell_nix() -> String {
    let mut out = String::new();

    out.push_str(&generate_file_header("Development shell configuration"));
    out.push_str(
        "{ pkgs, lib, devPackages, notices ? []\n, go ? null, goAttr ? null, wantGoAttr ? null\n, python ? null, pythonAttr ? null, wantPythonAttr ? null\n, node ? null, nodeAttr ? null, wantNodeAttr ? null\n, rustToolchain ? null\n}:\n\n",
    );

    out.push_str("pkgs.mkShell {\n");
    out.push_str("  packages = devPackages;\n\n");

    out.push_str("  shellHook = ''\n");
    out.push_str("    echo \"autonix: generated devShell (best-effort)\"\n\n");

    out.push_str(
        "    ${lib.optionalString (go != null) ''\n      echo \"autonix: Go attr: ${goAttr} (requested ${wantGoAttr})\"\n      if [ \"${goAttr}\" != \"${wantGoAttr}\" ]; then\n        echo \"autonix: NOTE: ${wantGoAttr} not found; using ${goAttr}\"\n      fi\n    ''}\n\n",
    );

    out.push_str(
        "    ${lib.optionalString (python != null) ''\n      echo \"autonix: Python attr: ${pythonAttr} (requested ${wantPythonAttr})\"\n      if [ \"${pythonAttr}\" != \"${wantPythonAttr}\" ]; then\n        echo \"autonix: NOTE: ${wantPythonAttr} not found; using ${pythonAttr}\"\n      fi\n    ''}\n\n",
    );

    out.push_str(
        "    ${lib.optionalString (node != null) ''\n      echo \"autonix: Node attr: ${nodeAttr} (requested ${wantNodeAttr})\"\n      if [ \"${nodeAttr}\" != \"${wantNodeAttr}\" ]; then\n        echo \"autonix: NOTE: ${wantNodeAttr} not found; using ${nodeAttr}\"\n      fi\n    ''}\n\n",
    );

    out.push_str(
        "    ${lib.optionalString (rustToolchain != null) ''\n      echo \"autonix: Rust toolchain enabled (rust-overlay)\"\n    ''}\n\n",
    );

    out.push_str(
        "    ${lib.concatMapStringsSep \"\\n\" (msg: \"echo ${lib.escapeShellArg msg}\") notices}\n",
    );

    out.push_str("  '';\n");
    out.push_str("}\n");

    out
}

fn generate_rust_overlay_nix() -> String {
    let mut out = String::new();
    out.push_str(&generate_file_header("Rust overlay (oxalica/rust-overlay)"));
    out.push_str("{ rust-overlay }: import rust-overlay\n");
    out
}

fn generate_golang_packages_nix(want_go_attr: &str, notice: Option<&str>) -> String {
    let mut out = String::new();

    out.push_str(&generate_file_header("Go toolchain and development tools"));
    out.push_str("{ pkgs, lib }:\n\n");

    out.push_str("let\n");
    nix_builder::write_nix_string_binding(&mut out, "  ", "wantGoAttr", want_go_attr);
    nix_builder::write_attr_with_fallback(&mut out, "  ", "goAttr", "wantGoAttr", "pkgs", "go");
    out.push_str("  go = pkgs.${goAttr};\n\n");

    out.push_str(&nix_builder::NoticeListBuilder::new("  ").build(notice));

    out.push_str("in\n{\n");
    out.push_str("  inherit go goAttr wantGoAttr notices;\n\n");
    out.push_str("  packages = [\n");
    out.push_str("    go\n");
    out.push_str(&format!("    pkgs.{}\n", constants::GO_TOOL_GOPLS));
    out.push_str("  ];\n");
    out.push_str("}\n");

    out
}

fn generate_python_packages_nix(
    want_python_attr: &str,
    notice: Option<&str>,
    required_package_managers: &HashSet<PackageManager>,
    required_task_runner_tools: &BTreeSet<&'static str>,
) -> String {
    let include_tox = required_task_runner_tools.contains(constants::PYTHON_TOOL_TOX);
    let include_nox = required_task_runner_tools.contains(constants::PYTHON_TOOL_NOX);
    let include_invoke = required_task_runner_tools.contains(constants::PYTHON_TOOL_INVOKE);

    let include_poetry = required_package_managers.contains(&PackageManager::Poetry);
    let include_uv = required_package_managers.contains(&PackageManager::Uv);
    let include_pdm = required_package_managers.contains(&PackageManager::Pdm);
    let include_pipenv = required_package_managers.contains(&PackageManager::Pipenv);

    let mut out = String::new();

    out.push_str(&generate_file_header(
        "Python toolchain and development tools",
    ));
    out.push_str("{ pkgs, lib }:\n\n");

    out.push_str("let\n");
    nix_builder::write_nix_string_binding(&mut out, "  ", "wantPythonAttr", want_python_attr);
    nix_builder::write_attr_with_fallback(
        &mut out,
        "  ",
        "pythonAttr",
        "wantPythonAttr",
        "pkgs",
        "python3",
    );
    out.push_str("  python = pkgs.${pythonAttr};\n");
    out.push_str("  wantPythonPackagesAttr = \"${pythonAttr}Packages\";\n");
    out.push_str(
        "  pythonPackages = if builtins.hasAttr wantPythonPackagesAttr pkgs then pkgs.${wantPythonPackagesAttr} else pkgs.python3Packages;\n\n",
    );

    if include_tox {
        out.push_str(&format!(
            "  {tool} = if builtins.hasAttr \"{tool}\" pythonPackages then pythonPackages.{tool} else null;\n",
            tool = constants::PYTHON_TOOL_TOX,
        ));
    }
    if include_nox {
        out.push_str(&format!(
            "  {tool} = if builtins.hasAttr \"{tool}\" pythonPackages then pythonPackages.{tool} else null;\n",
            tool = constants::PYTHON_TOOL_NOX,
        ));
    }
    if include_invoke {
        out.push_str(&format!(
            "  {tool} = if builtins.hasAttr \"{tool}\" pythonPackages then pythonPackages.{tool} else null;\n",
            tool = constants::PYTHON_TOOL_INVOKE,
        ));
    }

    if include_poetry {
        out.push_str(
            "  poetry = if builtins.hasAttr \"poetry\" pkgs then pkgs.poetry else null;\n",
        );
    }
    if include_uv {
        out.push_str("  uv = if builtins.hasAttr \"uv\" pkgs then pkgs.uv else null;\n");
    }
    if include_pdm {
        out.push_str("  pdm = if builtins.hasAttr \"pdm\" pkgs then pkgs.pdm else null;\n");
    }
    if include_pipenv {
        out.push_str(
            "  pipenv = if builtins.hasAttr \"pipenv\" pkgs then pkgs.pipenv else null;\n",
        );
    }

    out.push_str(
        "  pyright = if builtins.hasAttr \"pyright\" pkgs then pkgs.pyright\n    else if builtins.hasAttr \"pyright\" pkgs.nodePackages then pkgs.nodePackages.pyright\n    else null;\n\n",
    );

    out.push_str(&nix_builder::NoticeListBuilder::new("  ").build(notice));

    out.push_str("in\n{\n");
    out.push_str("  inherit python pythonPackages pythonAttr wantPythonAttr notices;\n\n");

    out.push_str("  packages = [ python ]");

    if include_tox {
        out.push_str("\n    ++ lib.optional (tox != null) tox");
    }
    if include_nox {
        out.push_str("\n    ++ lib.optional (nox != null) nox");
    }
    if include_invoke {
        out.push_str("\n    ++ lib.optional (invoke != null) invoke");
    }

    if include_poetry {
        out.push_str("\n    ++ lib.optional (poetry != null) poetry");
    }
    if include_uv {
        out.push_str("\n    ++ lib.optional (uv != null) uv");
    }
    if include_pdm {
        out.push_str("\n    ++ lib.optional (pdm != null) pdm");
    }
    if include_pipenv {
        out.push_str("\n    ++ lib.optional (pipenv != null) pipenv");
    }

    out.push_str("\n    ++ lib.optional (pyright != null) pyright");

    out.push_str(";\n");
    out.push_str("}\n");

    out
}

fn generate_nodejs_packages_nix(
    want_node_attr: &str,
    notice: Option<&str>,
    required_package_managers: &HashSet<PackageManager>,
    required_node_tools: &BTreeSet<&'static str>,
) -> String {
    let include_pnpm = required_package_managers.contains(&PackageManager::Pnpm);
    let include_yarn = required_package_managers.contains(&PackageManager::Yarn);
    let include_bun = required_package_managers.contains(&PackageManager::Bun);
    let include_deno = required_package_managers.contains(&PackageManager::Deno);

    let mut out = String::new();

    out.push_str(&generate_file_header(
        "Node.js toolchain and development tools",
    ));
    out.push_str("{ pkgs, lib }:\n\n");

    out.push_str("let\n");
    nix_builder::write_nix_string_binding(&mut out, "  ", "wantNodeAttr", want_node_attr);
    nix_builder::write_attr_with_fallback(
        &mut out,
        "  ",
        "nodeAttr",
        "wantNodeAttr",
        "pkgs",
        "nodejs",
    );
    out.push_str("  node = pkgs.${nodeAttr};\n\n");

    if include_pnpm {
        out.push_str(
            "  pnpm = if builtins.hasAttr \"pnpm\" pkgs.nodePackages then pkgs.nodePackages.pnpm else null;\n",
        );
    }
    if include_yarn {
        out.push_str("  yarn = if builtins.hasAttr \"yarn\" pkgs then pkgs.yarn else null;\n");
    }
    if include_bun {
        out.push_str("  bun = if builtins.hasAttr \"bun\" pkgs then pkgs.bun else null;\n");
    }
    if include_deno {
        out.push_str("  deno = if builtins.hasAttr \"deno\" pkgs then pkgs.deno else null;\n");
    }

    for tool in required_node_tools {
        out.push_str(&format!(
            "  {tool} = if builtins.hasAttr \"{tool}\" pkgs.nodePackages then pkgs.nodePackages.{tool} else null;\n"
        ));
    }

    out.push('\n');
    out.push_str(&nix_builder::NoticeListBuilder::new("  ").build(notice));

    out.push_str("in\n{\n");
    out.push_str("  inherit node nodeAttr wantNodeAttr notices;\n\n");

    out.push_str("  packages =\n");
    out.push_str("    [\n");
    out.push_str("      node\n");
    out.push_str(&format!(
        "      pkgs.nodePackages.{}\n",
        constants::NODE_PKG_TYPESCRIPT
    ));
    out.push_str(&format!(
        "      pkgs.nodePackages.{}\n",
        constants::NODE_PKG_TYPESCRIPT_LS
    ));
    out.push_str("    ]");

    if include_pnpm {
        out.push_str("\n    ++ lib.optional (pnpm != null) pnpm");
    }
    if include_yarn {
        out.push_str("\n    ++ lib.optional (yarn != null) yarn");
    }
    if include_bun {
        out.push_str("\n    ++ lib.optional (bun != null) bun");
    }
    if include_deno {
        out.push_str("\n    ++ lib.optional (deno != null) deno");
    }

    for tool in required_node_tools {
        out.push_str(&format!("\n    ++ lib.optional ({tool} != null) {tool}"));
    }

    out.push_str(";\n");
    out.push_str("}\n");

    out
}

fn generate_rust_packages_nix(want_rust_version: Option<&str>, notice: Option<&str>) -> String {
    let mut out = String::new();

    out.push_str(&generate_file_header(
        "Rust toolchain and development tools",
    ));
    out.push_str("{ pkgs, lib }:\n\n");

    out.push_str("let\n");
    if let Some(want) = want_rust_version {
        nix_builder::write_nix_string_binding(&mut out, "  ", "wantRustVersion", want);
        out.push_str(
            "  rustToolchainBase = if builtins.hasAttr wantRustVersion pkgs.rust-bin.stable\n    then pkgs.rust-bin.stable.${wantRustVersion}.default\n    else pkgs.rust-bin.stable.latest.default;\n",
        );
    } else {
        out.push_str("  rustToolchainBase = pkgs.rust-bin.stable.latest.default;\n");
    }

    out.push('\n');
    out.push_str("  rustToolchain = rustToolchainBase.override {\n");
    out.push_str("    extensions = [ \"rust-src\" \"rust-analyzer\" ];\n");
    out.push_str("  };\n\n");

    out.push_str(&nix_builder::NoticeListBuilder::new("  ").build(notice));

    out.push_str("in\n{\n");
    out.push_str("  inherit rustToolchain notices;\n\n");
    out.push_str("  packages = [ rustToolchain ];\n");
    out.push_str("}\n");

    out
}

fn detected_languages(metadata: &ProjectMetadata) -> HashSet<Language> {
    metadata
        .languages
        .iter()
        .map(|l| l.language.clone())
        .collect()
}

fn primary_language(metadata: &ProjectMetadata) -> Option<Language> {
    let langs = detected_languages(metadata);
    (langs.len() == 1)
        .then(|| langs.into_iter().next())
        .flatten()
}

fn detected_package_managers(metadata: &ProjectMetadata) -> HashSet<PackageManager> {
    metadata
        .package_managers
        .iter()
        .flat_map(|pm| pm.package_managers.iter().map(|p| p.package_manager))
        .collect()
}

fn best_version_info<'a>(
    metadata: &'a ProjectMetadata,
    language: Language,
    allowed_sources: &[VersionSource],
) -> Option<&'a VersionInfo> {
    let versions = metadata
        .versions
        .iter()
        .find(|vd| vd.language == language)?
        .versions
        .iter()
        .filter(|v| allowed_sources.contains(&v.source))
        .filter(|v| v.parsed.is_some());

    versions.max_by_key(|v| {
        let Some(parsed) = &v.parsed else {
            return (0_u32, 0_u32, 0_u32);
        };
        (
            parsed.major.unwrap_or(0),
            parsed.minor.unwrap_or(0),
            parsed.patch.unwrap_or(0),
        )
    })
}

fn go_attr_from_version(version: &SemanticVersion) -> Option<String> {
    let major = version.major?;
    let minor = version.minor?;
    Some(format!("go_{major}_{minor}"))
}

fn python_attr_from_version(version: &SemanticVersion) -> Option<String> {
    let major = version.major?;
    let minor = version.minor?;
    Some(format!("python{major}{minor}"))
}

fn node_attr_from_version(version: &SemanticVersion) -> Option<String> {
    let major = version.major?;
    Some(format!("nodejs_{major}"))
}

fn rust_version_string_from_version(version: &SemanticVersion) -> Option<String> {
    let major = version.major?;
    let minor = version.minor?;
    let patch = version.patch?;
    Some(format!("{major}.{minor}.{patch}"))
}

fn read_package_json_manager(path: &Path) -> Option<PackageManager> {
    let content = fs::read_to_string(path).ok()?;
    let parsed: JsonValue = serde_json::from_str(&content).ok()?;
    let package_manager_str = parsed.get("packageManager")?.as_str()?;

    let name = package_manager_str
        .split('@')
        .next()
        .unwrap_or(package_manager_str);

    match name {
        "npm" => Some(PackageManager::Npm),
        "pnpm" => Some(PackageManager::Pnpm),
        "yarn" => Some(PackageManager::Yarn),
        "bun" => Some(PackageManager::Bun),
        _ => None,
    }
}

fn detect_lockfile_manager(dir: &Path) -> Option<PackageManager> {
    if dir.join("bun.lockb").exists() || dir.join("bun.lock").exists() {
        return Some(PackageManager::Bun);
    }
    if dir.join("pnpm-lock.yaml").exists() {
        return Some(PackageManager::Pnpm);
    }
    if dir.join("yarn.lock").exists() {
        return Some(PackageManager::Yarn);
    }
    if dir.join("package-lock.json").exists() {
        return Some(PackageManager::Npm);
    }

    None
}

fn resolve_js_package_manager(package_json_path: &Path) -> PackageManager {
    if let Some(pm) = read_package_json_manager(package_json_path) {
        return pm;
    }

    let Some(parent) = package_json_path.parent() else {
        return PackageManager::Npm;
    };

    if let Some(pm) = detect_lockfile_manager(parent) {
        return pm;
    }

    PackageManager::Npm
}

fn collect_checks(
    metadata: &ProjectMetadata,
    root: &Path,
    required_package_managers: &mut HashSet<PackageManager>,
    need_node: &mut bool,
    need_python: &mut bool,
) -> HashMap<Option<Language>, HashMap<CheckCategory, Vec<CheckSpec>>> {
    let mut grouped: HashMap<Option<Language>, HashMap<CheckCategory, Vec<CheckSpec>>> =
        HashMap::new();
    let mut key_counts: HashMap<String, usize> = HashMap::new();

    let primary_language = primary_language(metadata);

    for tr in &metadata.task_runners {
        let runner_path = relativize_path(root, &tr.path).unwrap_or_else(|| tr.path.clone());
        let runner_slug = slugify_path(&runner_path);

        for (category, cmds) in [
            (CheckCategory::Test, &tr.commands.test),
            (CheckCategory::Build, &tr.commands.build),
        ] {
            for cmd in cmds {
                let (cmd_info, pm_used) = resolve_task_command(cmd, root);

                if let Some(pm) = pm_used {
                    required_package_managers.insert(pm);
                    if pm.is_js_package_manager() {
                        *need_node = true;
                    }
                }

                if cmd_info.required_exec == constants::PYTHON_TOOL_TOX
                    || cmd_info.required_exec == constants::PYTHON_TOOL_NOX
                    || cmd_info.required_exec == constants::PYTHON_TOOL_INVOKE
                {
                    *need_python = true;
                }

                let language = infer_check_language(
                    tr.task_runner,
                    &cmd_info.required_exec,
                    primary_language.clone(),
                );

                let spec = build_check_spec(
                    cmd,
                    cmd_info,
                    language.clone(),
                    category,
                    tr.task_runner,
                    &runner_slug,
                    &mut key_counts,
                );

                grouped
                    .entry(spec.language.clone())
                    .or_default()
                    .entry(spec.category)
                    .or_default()
                    .push(spec);
            }
        }
    }

    for by_cat in grouped.values_mut() {
        for specs in by_cat.values_mut() {
            specs.sort_by(|a, b| a.key.cmp(&b.key));
        }
    }

    grouped
}

fn infer_check_language(
    task_runner: TaskRunner,
    required_exec: &str,
    primary_language: Option<Language>,
) -> Option<Language> {
    match task_runner {
        TaskRunner::Cargo => return Some(Language::Rust),
        TaskRunner::GoTask => return Some(Language::Go),
        TaskRunner::NpmScripts
        | TaskRunner::Vite
        | TaskRunner::Webpack
        | TaskRunner::Rspack
        | TaskRunner::Rollup
        | TaskRunner::Turbo
        | TaskRunner::Nx => return Some(Language::JavaScript),
        TaskRunner::Tox | TaskRunner::Nox | TaskRunner::Invoke => return Some(Language::Python),
        TaskRunner::Make | TaskRunner::Just | TaskRunner::Task => {}
    }

    match required_exec {
        "cargo" => Some(Language::Rust),
        "go" => Some(Language::Go),
        "npm" | "pnpm" | "yarn" | "bun" | "node" => Some(Language::JavaScript),
        "python"
        | "python3"
        | constants::PYTHON_TOOL_TOX
        | constants::PYTHON_TOOL_NOX
        | constants::PYTHON_TOOL_INVOKE => Some(Language::Python),
        "make" | "just" | "task" => primary_language,
        _ => primary_language,
    }
}

fn language_dir_name(lang: Option<&Language>) -> &'static str {
    lang.map(|l| l.dir_name()).unwrap_or("generic")
}

fn language_display_name(lang: Option<Language>) -> &'static str {
    match lang {
        Some(Language::Go) => "Go",
        Some(Language::Python) => "Python",
        Some(Language::JavaScript) => "Node.js",
        Some(Language::Rust) => "Rust",
        None => "Generic",
    }
}

fn category_name(category: CheckCategory) -> &'static str {
    match category {
        CheckCategory::Test => "test",
        CheckCategory::Build => "build",
    }
}

fn category_file_name(category: CheckCategory) -> &'static str {
    match category {
        CheckCategory::Test => "test-checks.nix",
        CheckCategory::Build => "build-checks.nix",
    }
}

fn resolve_task_command(cmd: &TaskCommand, root: &Path) -> (CommandInfo, Option<PackageManager>) {
    match &cmd.executable {
        CommandExecutable::Direct { command } => {
            let required_exec = command_first_word(command).unwrap_or("").to_string();
            let info = CommandInfo {
                required_exec,
                command: command.clone(),
                workdir: ".".to_string(),
                display: command.clone(),
            };
            (info, None)
        }
        CommandExecutable::PackageManagerScript {
            script_name,
            script_body,
            package_json_path,
        } => {
            let pm = resolve_js_package_manager(package_json_path);
            let pm_cmd = pm.command_name().unwrap_or("");

            let run_command = pm
                .run_script(script_name)
                .unwrap_or_else(|| format!("npm run {script_name}"));

            let workdir = package_json_path
                .parent()
                .and_then(|p| relativize_path(root, p))
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string());

            let display = format!("{run_command}  # {script_body}");

            let info = CommandInfo {
                required_exec: pm_cmd.to_string(),
                command: run_command,
                workdir,
                display,
            };

            (info, Some(pm))
        }
    }
}

fn build_check_spec(
    cmd: &TaskCommand,
    cmd_info: CommandInfo,
    language: Option<Language>,
    category: CheckCategory,
    task_runner: TaskRunner,
    runner_slug: &str,
    key_counts: &mut HashMap<String, usize>,
) -> CheckSpec {
    let language_prefix = language_dir_name(language.as_ref());
    let category_name = category_name(category);
    let runner_name = task_runner_name(task_runner).to_ascii_lowercase();

    let mut key = format!(
        "{language_prefix}-{category_name}-{runner_name}-{}-{runner_slug}",
        slugify_identifier(&cmd.name)
    );

    let count = key_counts.entry(key.clone()).or_insert(0);
    *count += 1;
    if *count > 1 {
        key = format!("{key}-{}", *count);
    }

    let derivation_name = format!("check-{key}");

    CheckSpec {
        language,
        category,
        key,
        derivation_name,
        display: cmd_info.display,
        required_exec: cmd_info.required_exec,
        command: cmd_info.command,
        workdir: cmd_info.workdir,
    }
}

fn generate_check_files(
    checks_by_lang: &HashMap<Option<Language>, HashMap<CheckCategory, Vec<CheckSpec>>>,
) -> Vec<CheckFile> {
    let mut out = Vec::new();

    let language_order = [
        None,
        Some(Language::Go),
        Some(Language::Python),
        Some(Language::JavaScript),
        Some(Language::Rust),
    ];

    for language in language_order {
        let Some(by_cat) = checks_by_lang.get(&language) else {
            continue;
        };

        for category in [CheckCategory::Test, CheckCategory::Build] {
            let Some(checks) = by_cat.get(&category) else {
                continue;
            };
            if checks.is_empty() {
                continue;
            }

            let relative_path = PathBuf::from(format!(
                "{}/{}",
                language_dir_name(language.as_ref()),
                category_file_name(category)
            ));

            let desc = format!(
                "{} {} checks",
                language_display_name(language.clone()),
                category_name(category)
            );

            let content = generate_check_file_content(&desc, checks);

            out.push(CheckFile {
                language: language.clone(),
                category,
                content,
                relative_path,
            });
        }
    }

    out
}

fn generate_check_file_content(description: &str, checks: &[CheckSpec]) -> String {
    let mut out = String::new();

    out.push_str(&generate_file_header(description));
    out.push_str("{ pkgs, lib, devPackages, projectRoot }:\n\n");
    out.push_str("{\n");

    for check in checks {
        let builder = nix_builder::CheckDerivationBuilder::new(
            check.key.clone(),
            check.derivation_name.clone(),
            check.display.clone(),
            check.required_exec.clone(),
            check.command.clone(),
            check.workdir.clone(),
        );
        out.push_str(&builder.build());
    }

    out.push_str("}\n");

    out
}

fn task_runner_name(task_runner: TaskRunner) -> &'static str {
    match task_runner {
        TaskRunner::Make => "Make",
        TaskRunner::Just => "Just",
        TaskRunner::Task => "Task",
        TaskRunner::NpmScripts => "NpmScripts",
        TaskRunner::Vite => "Vite",
        TaskRunner::Webpack => "Webpack",
        TaskRunner::Rspack => "Rspack",
        TaskRunner::Rollup => "Rollup",
        TaskRunner::Turbo => "Turbo",
        TaskRunner::Nx => "Nx",
        TaskRunner::Tox => "Tox",
        TaskRunner::Nox => "Nox",
        TaskRunner::Invoke => "Invoke",
        TaskRunner::Cargo => "Cargo",
        TaskRunner::GoTask => "GoTask",
    }
}

fn slugify_identifier(input: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;

    for ch in input.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            out.push(lower);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }

    out.trim_matches('-').to_string()
}

fn slugify_path(path: &Path) -> String {
    let s = path.to_string_lossy();
    slugify_identifier(&s)
}

fn relativize_path(root: &Path, path: &Path) -> Option<PathBuf> {
    if let Ok(root_abs) = root.canonicalize()
        && let Ok(path_abs) = path.canonicalize()
        && let Ok(stripped) = path_abs.strip_prefix(root_abs)
    {
        return Some(stripped.to_path_buf());
    }

    path.strip_prefix(root).ok().map(Path::to_path_buf)
}

fn command_first_word(command: &str) -> Option<&str> {
    command.split_whitespace().next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detection::DetectionEngine;
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

    fn all_check_contents(flake: &GeneratedFlake) -> String {
        flake
            .check_files
            .iter()
            .map(|f| f.content.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn language_packages_content(flake: &GeneratedFlake, language: Language) -> Option<&str> {
        flake
            .language_packages
            .iter()
            .find(|p| p.language == language)
            .map(|p| p.content.as_str())
    }

    #[test]
    fn generates_rust_overlay_when_rust_detected() {
        let dir = TempDir::new().unwrap();
        create_temp_file(
            &dir,
            "Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nrust-version = \"1.70.0\"\n",
        );
        create_temp_file(&dir, "src/main.rs", "fn main() {}\n");

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());
        let flake = generate_dev_flake(&metadata, dir.path());

        assert!(flake.main_flake.contains("rust-overlay"));
        assert!(flake.main_flake.contains(".autonix/rust/overlay.nix"));
        assert!(flake.rust_overlay.as_deref().is_some());

        let rust_pkgs = language_packages_content(&flake, Language::Rust).unwrap();
        assert!(rust_pkgs.contains("rust-bin.stable.latest.default"));
        assert!(rust_pkgs.contains("rust-analyzer"));
        assert!(rust_pkgs.contains("rust-src"));
    }

    #[test]
    fn resolves_package_manager_from_package_manager_field() {
        let dir = TempDir::new().unwrap();
        create_temp_file(
            &dir,
            "package.json",
            r#"{
  "name": "demo",
  "packageManager": "pnpm@9.0.0",
  "scripts": { "test": "jest" }
 }"#,
        );

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());
        let flake = generate_dev_flake(&metadata, dir.path());

        let checks = all_check_contents(&flake);
        assert!(checks.contains("pnpm run test"));
        assert!(checks.contains("jest"));
    }

    #[test]
    fn resolves_package_manager_from_lockfile_fallback() {
        let dir = TempDir::new().unwrap();
        create_temp_file(
            &dir,
            "package.json",
            r#"{
  "name": "demo",
  "scripts": { "test": "jest" }
 }"#,
        );
        create_temp_file(&dir, "pnpm-lock.yaml", "");

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());
        let flake = generate_dev_flake(&metadata, dir.path());

        let checks = all_check_contents(&flake);
        assert!(checks.contains("pnpm run test"));
    }

    #[test]
    fn defaults_to_npm_when_no_package_manager_signal() {
        let dir = TempDir::new().unwrap();
        create_temp_file(
            &dir,
            "package.json",
            r#"{
  "name": "demo",
  "scripts": { "test": "jest" }
 }"#,
        );

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());
        let flake = generate_dev_flake(&metadata, dir.path());

        let checks = all_check_contents(&flake);
        assert!(checks.contains("npm run test"));
    }

    #[test]
    fn includes_versioned_go_when_available() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, "go.mod", "module example.com\n\ngo 1.21\n");
        create_temp_file(&dir, "main.go", "package main\nfunc main(){}\n");

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());
        let flake = generate_dev_flake(&metadata, dir.path());

        let go_pkgs = language_packages_content(&flake, Language::Go).unwrap();
        assert!(go_pkgs.contains("go_1_21"));
    }

    #[test]
    fn test_multi_language_project_generates_all_packages() {
        let dir = TempDir::new().unwrap();

        create_temp_file(&dir, "go.mod", "module example.com\n\ngo 1.21\n");
        create_temp_file(&dir, "main.go", "package main\nfunc main(){}\n");
        create_temp_file(
            &dir,
            "package.json",
            r#"{"name": "test", "scripts": {"test": "jest"}}"#,
        );
        create_temp_file(
            &dir,
            "pyproject.toml",
            "[project]\nname = \"test\"\nrequires-python = \">=3.11\"\n",
        );
        create_temp_file(
            &dir,
            "Cargo.toml",
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        );
        create_temp_file(&dir, "src/main.rs", "fn main() {}\n");

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());
        let flake = generate_dev_flake(&metadata, dir.path());

        assert_eq!(flake.language_packages.len(), 4);
        assert!(
            flake
                .language_packages
                .iter()
                .any(|p| p.language == Language::Go)
        );
        assert!(
            flake
                .language_packages
                .iter()
                .any(|p| p.language == Language::Python)
        );
        assert!(
            flake
                .language_packages
                .iter()
                .any(|p| p.language == Language::JavaScript)
        );
        assert!(
            flake
                .language_packages
                .iter()
                .any(|p| p.language == Language::Rust)
        );

        assert!(flake.main_flake.contains("golangPackages"));
        assert!(flake.main_flake.contains("pythonPackages"));
        assert!(flake.main_flake.contains("nodejsPackages"));
        assert!(flake.main_flake.contains("rustPackages"));
    }

    #[test]
    fn test_check_key_deduplication_for_slug_collisions() {
        let dir = TempDir::new().unwrap();
        create_temp_file(
            &dir,
            "package.json",
            r#"{"scripts": {"test:unit": "echo one", "test_unit": "echo two"}}"#,
        );

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());
        let flake = generate_dev_flake(&metadata, dir.path());

        let checks = all_check_contents(&flake);
        assert!(checks.contains("\"nodejs-test-npmscripts-test-unit-package-json\""));
        assert!(checks.contains("\"nodejs-test-npmscripts-test-unit-package-json-2\""));
    }

    #[test]
    fn test_nested_package_json_workdir_resolution() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("packages/frontend")).unwrap();
        create_temp_file(
            &dir,
            "packages/frontend/package.json",
            r#"{"scripts": {"test": "vitest"}}"#,
        );

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());
        let flake = generate_dev_flake(&metadata, dir.path());

        let checks = all_check_contents(&flake);
        assert!(checks.contains("workdir = \"packages/frontend\";"));
    }

    #[test]
    fn test_version_fallback_when_unavailable() {
        let dir = TempDir::new().unwrap();
        create_temp_file(
            &dir,
            "pyproject.toml",
            "[project]\nrequires-python = \">=3.99\"\n",
        );

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());
        let flake = generate_dev_flake(&metadata, dir.path());

        let python_pkgs = language_packages_content(&flake, Language::Python).unwrap();
        assert!(python_pkgs.contains("wantPythonAttr = \"python399\""));
        assert!(python_pkgs.contains("else \"python3\""));
    }

    #[test]
    fn test_nix_escape_quotes() {
        assert_eq!(
            nix_builder::escape_nix_string(r#"hello "world""#),
            r#"hello \"world\""#
        );
    }

    #[test]
    fn test_nix_escape_dollar_brace() {
        assert_eq!(nix_builder::escape_nix_string("${foo}"), r"\${foo}");
        assert_eq!(nix_builder::escape_nix_string("$foo"), "$foo");
    }

    #[test]
    fn test_nix_escape_backslash() {
        assert_eq!(nix_builder::escape_nix_string(r"foo\bar"), r"foo\\bar");
    }

    #[test]
    fn test_nix_escape_newlines_tabs() {
        assert_eq!(
            nix_builder::escape_nix_string("foo\nbar\ttab"),
            "foo\\nbar\\ttab"
        );
    }

    #[test]
    fn test_nix_escape_combined() {
        assert_eq!(
            nix_builder::escape_nix_string(r#"echo "${VAR}" > file\nend"#),
            r#"echo \"\${VAR}\" > file\\nend"#
        );
    }

    #[test]
    fn test_best_version_info_prefers_highest_version() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, "go.mod", "module example.com\n\ngo 1.21\n");
        create_temp_file(&dir, ".go-version", "1.19\n");
        create_temp_file(&dir, "main.go", "package main\nfunc main(){}\n");

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());
        let version = best_version_info(&metadata, Language::Go, constants::GO_VERSION_SOURCES);

        let parsed = version.unwrap().parsed.as_ref().unwrap();
        assert_eq!(parsed.major, Some(1));
        assert_eq!(parsed.minor, Some(21));
    }

    #[test]
    fn test_best_version_info_filters_by_allowed_sources() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, ".node-version", "18.0.0\n");
        create_temp_file(&dir, ".bun-version", "999.0.0\n");

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());
        let version = best_version_info(
            &metadata,
            Language::JavaScript,
            constants::NODE_VERSION_SOURCES,
        )
        .unwrap();

        assert!(matches!(version.source, VersionSource::NodeVersionFile));
        assert_eq!(version.raw, "18.0.0");
    }

    #[test]
    fn test_python_attr_from_version() {
        let version = SemanticVersion {
            major: Some(3),
            minor: Some(11),
            patch: None,
            pre_release: None,
            build: None,
            constraint: VersionConstraint::Exact,
        };
        assert_eq!(
            python_attr_from_version(&version),
            Some("python311".to_string())
        );
    }

    #[test]
    fn test_python_attr_from_version_missing_minor() {
        let version = SemanticVersion {
            major: Some(3),
            minor: None,
            patch: None,
            pre_release: None,
            build: None,
            constraint: VersionConstraint::Exact,
        };
        assert_eq!(python_attr_from_version(&version), None);
    }

    #[test]
    fn test_node_attr_from_version() {
        let version = SemanticVersion {
            major: Some(20),
            minor: Some(10),
            patch: Some(0),
            pre_release: None,
            build: None,
            constraint: VersionConstraint::Exact,
        };
        assert_eq!(
            node_attr_from_version(&version),
            Some("nodejs_20".to_string())
        );
    }

    #[test]
    fn test_go_attr_from_version() {
        let version = SemanticVersion {
            major: Some(1),
            minor: Some(21),
            patch: Some(3),
            pre_release: None,
            build: None,
            constraint: VersionConstraint::Exact,
        };
        assert_eq!(go_attr_from_version(&version), Some("go_1_21".to_string()));
    }

    #[test]
    fn test_slugify_identifier() {
        assert_eq!(slugify_identifier("test:build"), "test-build");
        assert_eq!(slugify_identifier("my_test___name"), "my-test-name");
        assert_eq!(slugify_identifier("CamelCase"), "camelcase");
        assert_eq!(slugify_identifier("multiple---dashes"), "multiple-dashes");
    }

    #[test]
    fn test_infer_check_language_from_cargo() {
        let lang = infer_check_language(TaskRunner::Cargo, "cargo", None);
        assert_eq!(lang, Some(Language::Rust));
    }

    #[test]
    fn test_infer_check_language_from_npm_scripts() {
        let lang = infer_check_language(TaskRunner::NpmScripts, "npm", None);
        assert_eq!(lang, Some(Language::JavaScript));
    }

    #[test]
    fn test_infer_check_language_falls_back_to_primary() {
        let lang = infer_check_language(TaskRunner::Make, "make", Some(Language::Python));
        assert_eq!(lang, Some(Language::Python));
    }

    #[test]
    fn test_resolve_task_command_direct() {
        let dir = TempDir::new().unwrap();
        let cmd = TaskCommand {
            name: "test".to_string(),
            executable: CommandExecutable::Direct {
                command: "cargo test".to_string(),
            },
            description: None,
        };

        let (info, pm) = resolve_task_command(&cmd, dir.path());
        assert_eq!(info.required_exec, "cargo");
        assert_eq!(info.command, "cargo test");
        assert_eq!(info.workdir, ".");
        assert_eq!(info.display, "cargo test");
        assert!(pm.is_none());
    }

    #[test]
    fn test_build_check_spec_deduplication() {
        let mut key_counts = HashMap::new();
        let cmd = TaskCommand {
            name: "test".to_string(),
            executable: CommandExecutable::Direct {
                command: "npm test".to_string(),
            },
            description: None,
        };
        let info = CommandInfo {
            required_exec: "npm".to_string(),
            command: "npm test".to_string(),
            workdir: ".".to_string(),
            display: "npm test".to_string(),
        };

        let spec1 = build_check_spec(
            &cmd,
            info.clone(),
            Some(Language::JavaScript),
            CheckCategory::Test,
            TaskRunner::NpmScripts,
            "root",
            &mut key_counts,
        );

        let spec2 = build_check_spec(
            &cmd,
            info,
            Some(Language::JavaScript),
            CheckCategory::Test,
            TaskRunner::NpmScripts,
            "root",
            &mut key_counts,
        );

        assert_ne!(spec1.key, spec2.key);
        assert!(spec2.key.ends_with("-2"));
    }

    #[test]
    fn test_resolve_js_pm_from_package_json_field() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(&dir, "package.json", r#"{"packageManager": "pnpm@9.0.0"}"#);
        assert_eq!(resolve_js_package_manager(&path), PackageManager::Pnpm);
    }

    #[test]
    fn test_resolve_js_pm_from_lockfile_bun() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, "package.json", r#"{"name": "test"}"#);
        create_temp_file(&dir, "bun.lockb", "");

        let pm = resolve_js_package_manager(&dir.path().join("package.json"));
        assert_eq!(pm, PackageManager::Bun);
    }

    #[test]
    fn test_resolve_js_pm_from_lockfile_pnpm() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, "package.json", r#"{"name": "test"}"#);
        create_temp_file(&dir, "pnpm-lock.yaml", "");

        let pm = resolve_js_package_manager(&dir.path().join("package.json"));
        assert_eq!(pm, PackageManager::Pnpm);
    }

    #[test]
    fn test_resolve_js_pm_defaults_to_npm() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, "package.json", r#"{"name": "test"}"#);

        let pm = resolve_js_package_manager(&dir.path().join("package.json"));
        assert_eq!(pm, PackageManager::Npm);
    }

    #[test]
    fn test_read_package_json_manager_missing_field() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(&dir, "package.json", r#"{"name": "test"}"#);
        assert_eq!(read_package_json_manager(&path), None);
    }

    #[test]
    fn test_detect_lockfile_manager_bun_priority() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, "bun.lockb", "");
        create_temp_file(&dir, "package-lock.json", "");

        assert_eq!(
            detect_lockfile_manager(dir.path()),
            Some(PackageManager::Bun)
        );
    }

    #[test]
    fn test_detect_lockfile_manager_no_lockfile() {
        let dir = TempDir::new().unwrap();
        assert_eq!(detect_lockfile_manager(dir.path()), None);
    }

    #[test]
    fn test_relativize_path_basic() {
        let dir = TempDir::new().unwrap();
        let subdir = dir.path().join("sub");
        fs::create_dir_all(&subdir).unwrap();

        let rel = relativize_path(dir.path(), &subdir).unwrap();
        assert_eq!(rel.to_str().unwrap(), "sub");
    }

    #[test]
    fn test_relativize_path_same_dir() {
        let dir = TempDir::new().unwrap();

        let rel = relativize_path(dir.path(), dir.path()).unwrap();
        assert_eq!(rel.to_str().unwrap(), "");
    }

    #[test]
    fn test_go_notice_with_patch_version() {
        let version_info = VersionInfo {
            raw: "1.21.3".to_string(),
            source: VersionSource::GoModDirective,
            parsed: Some(SemanticVersion {
                major: Some(1),
                minor: Some(21),
                patch: Some(3),
                pre_release: None,
                build: None,
                constraint: VersionConstraint::Exact,
            }),
            path: PathBuf::from("go.mod"),
        };

        let notice = go_notice(Some(&version_info), Some("go_1_21")).unwrap();
        assert!(notice.contains("1.21.3"));
        assert!(notice.contains("go_1_21"));
        assert!(notice.contains("patch may differ"));
    }

    #[test]
    fn test_python_notice_with_constraint() {
        let version_info = VersionInfo {
            raw: ">=3.10".to_string(),
            source: VersionSource::PyprojectRequiresPython,
            parsed: Some(SemanticVersion {
                major: Some(3),
                minor: Some(10),
                patch: None,
                pre_release: None,
                build: None,
                constraint: VersionConstraint::GreaterOrEqual,
            }),
            path: PathBuf::from("pyproject.toml"),
        };

        let notice = python_notice(Some(&version_info), Some("python310")).unwrap();
        assert!(notice.contains(">=3.10"));
        assert!(notice.contains("patch may differ"));
    }

    #[test]
    fn test_generate_version_notice() {
        let version_info = VersionInfo {
            raw: "1.21.3".to_string(),
            source: VersionSource::GoModDirective,
            parsed: Some(SemanticVersion {
                major: Some(1),
                minor: Some(21),
                patch: Some(3),
                pre_release: None,
                build: None,
                constraint: VersionConstraint::Exact,
            }),
            path: PathBuf::from("go.mod"),
        };

        let notice = generate_version_notice(
            "Go",
            Some(&version_info),
            Some("go_1_21"),
            "go",
            Some("note: test"),
        )
        .unwrap();

        assert!(notice.contains("Go: requested 1.21.3"));
        assert!(notice.contains("go_1_21"));
        assert!(notice.contains("note: test"));
    }

    #[test]
    fn test_generate_flake_inputs_without_rust_overlay() {
        let result = generate_flake_inputs(false);
        assert!(result.contains("nixpkgs.url"));
        assert!(result.contains("flake-utils.url"));
        assert!(!result.contains("rust-overlay"));
    }

    #[test]
    fn test_generate_flake_inputs_with_rust_overlay() {
        let result = generate_flake_inputs(true);
        assert!(result.contains("rust-overlay"));
        assert!(result.contains("oxalica/rust-overlay"));
    }

    #[test]
    fn test_generate_devshell_binding_all_languages() {
        let result = generate_devshell_binding(true, true, true, true);
        assert!(result.contains("golangPackages.go"));
        assert!(result.contains("pythonPackages.python"));
        assert!(result.contains("nodejsPackages.node"));
        assert!(result.contains("rustPackages.rustToolchain"));
    }

    #[test]
    fn test_generate_checks_binding_empty() {
        let result = generate_checks_binding(&[]);
        assert_eq!(result.trim(), "checks = {};");
    }

    #[test]
    fn test_command_first_word() {
        assert_eq!(command_first_word("npm run test"), Some("npm"));
        assert_eq!(command_first_word("cargo build --release"), Some("cargo"));
        assert_eq!(command_first_word("  spaced  "), Some("spaced"));
        assert_eq!(command_first_word(""), None);
    }
}
