use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value as JsonValue;

use crate::detection::{
    CommandExecutable, Language, PackageManager, ProjectMetadata, SemanticVersion, TaskCommand,
    TaskRunner, VersionConstraint, VersionInfo, VersionSource,
};

#[derive(Debug, Clone)]
struct CheckSpec {
    key: String,
    derivation_name: String,
    display: String,
    required_exec: String,
    command: String,
    workdir: String,
}

pub fn generate_dev_flake(metadata: &ProjectMetadata, root: &Path) -> String {
    let languages = detected_languages(metadata);

    let task_runners: HashSet<TaskRunner> = metadata
        .task_runners
        .iter()
        .map(|tr| tr.task_runner)
        .collect();

    let mut need_node = languages.contains(&Language::JavaScript)
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

    let mut need_python = languages.contains(&Language::Python)
        || task_runners
            .iter()
            .any(|tr| matches!(tr, TaskRunner::Tox | TaskRunner::Nox | TaskRunner::Invoke));

    let need_go = languages.contains(&Language::Go);
    let need_rust = languages.contains(&Language::Rust);

    let go_version = best_version_info(metadata, Language::Go, GO_VERSION_SOURCES);
    let python_version = best_version_info(metadata, Language::Python, PYTHON_VERSION_SOURCES);
    let node_version = best_version_info(metadata, Language::JavaScript, NODE_VERSION_SOURCES);
    let rust_version = best_version_info(metadata, Language::Rust, RUST_VERSION_SOURCES);

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

    let checks = collect_checks(
        metadata,
        root,
        &mut required_package_managers,
        &mut need_node,
        &mut need_python,
    );

    let mut required_node_tools: BTreeSet<&'static str> = BTreeSet::new();
    for task_runner in task_runners {
        match task_runner {
            TaskRunner::Vite => {
                required_node_tools.insert("vite");
            }
            TaskRunner::Webpack => {
                required_node_tools.insert("webpack");
            }
            TaskRunner::Rspack => {
                required_node_tools.insert("rspack");
            }
            TaskRunner::Rollup => {
                required_node_tools.insert("rollup");
            }
            TaskRunner::Turbo => {
                required_node_tools.insert("turbo");
            }
            TaskRunner::Nx => {
                required_node_tools.insert("nx");
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

    let mut required_task_runner_tools: BTreeSet<&'static str> = BTreeSet::new();
    for tr in metadata.task_runners.iter().map(|t| t.task_runner) {
        match tr {
            TaskRunner::Make => {
                required_task_runner_tools.insert("gnumake");
            }
            TaskRunner::Just => {
                required_task_runner_tools.insert("just");
            }
            TaskRunner::Task => {
                required_task_runner_tools.insert("go-task");
            }
            TaskRunner::Tox => {
                required_task_runner_tools.insert("tox");
            }
            TaskRunner::Nox => {
                required_task_runner_tools.insert("nox");
            }
            TaskRunner::Invoke => {
                required_task_runner_tools.insert("invoke");
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

    let mut notices = Vec::new();
    if let Some(version) = go_version
        && let Some(parsed) = &version.parsed
    {
        let requested = format!("{} (from {:?})", version.raw, version.source);
        let selected = go_want_attr
            .as_deref()
            .unwrap_or("go (unversioned; go_* not inferred)");
        let note = if parsed.patch.is_some() {
            "note: nixpkgs provides Go by major/minor (patch may differ)"
        } else {
            ""
        };
        let msg = format!("Go: requested {requested} -> want {selected} {note}")
            .trim()
            .to_string();
        notices.push(msg);
    }

    if let Some(version) = python_version
        && let Some(parsed) = &version.parsed
    {
        let requested = format!("{} (from {:?})", version.raw, version.source);
        let selected = python_want_attr
            .as_deref()
            .unwrap_or("python3 (unversioned; pythonXY not inferred)");
        let note =
            if parsed.patch.is_some() || !matches!(parsed.constraint, VersionConstraint::Exact) {
                "note: nixpkgs provides Python by major/minor (patch may differ)"
            } else {
                ""
            };
        let msg = format!("Python: requested {requested} -> want {selected} {note}")
            .trim()
            .to_string();
        notices.push(msg);
    }

    if let Some(version) = node_version
        && let Some(parsed) = &version.parsed
    {
        let requested = format!("{} (from {:?})", version.raw, version.source);
        let selected = node_want_attr
            .as_deref()
            .unwrap_or("nodejs (unversioned; nodejs_* not inferred)");
        let note = if parsed.minor.is_some() || parsed.patch.is_some() {
            "note: nixpkgs provides Node.js by major (minor/patch may differ)"
        } else {
            ""
        };
        let msg = format!("Node: requested {requested} -> want {selected} {note}")
            .trim()
            .to_string();
        notices.push(msg);
    }

    if need_rust {
        if let Some(v) = rust_want_version.as_deref() {
            notices.push(format!(
                "Rust: requested {} -> try rust-bin.stable.{v} (fallback latest)",
                rust_version
                    .map(|vi| format!("{} (from {:?})", vi.raw, vi.source))
                    .unwrap_or_else(|| "(unknown)".to_string())
            ));
        } else if let Some(vi) = rust_version {
            notices.push(format!(
                "Rust: detected {} (from {:?}) -> using rust-bin.stable.latest (not exact pin)",
                vi.raw, vi.source
            ));
        }
    }

    let mut output = String::new();

    let uses_rust_overlay = need_rust;

    output.push_str("{\n");
    output.push_str("  description = \"Generated by autonix (devShells.default + checks)\";\n\n");
    output.push_str("  inputs = {\n");
    output.push_str("    nixpkgs.url = \"github:NixOS/nixpkgs/nixos-unstable\";\n");
    output.push_str("    flake-utils.url = \"github:numtide/flake-utils\";\n");
    if uses_rust_overlay {
        output.push_str("    rust-overlay = {\n");
        output.push_str("      url = \"github:oxalica/rust-overlay\";\n");
        output.push_str("      inputs.nixpkgs.follows = \"nixpkgs\";\n");
        output.push_str("    };\n");
    }
    output.push_str("  };\n\n");

    output.push_str("  outputs = { self, nixpkgs, flake-utils");
    if uses_rust_overlay {
        output.push_str(", rust-overlay");
    }
    output.push_str(" }: \n");
    output.push_str("    flake-utils.lib.eachDefaultSystem (system:\n");
    output.push_str("      let\n");

    if uses_rust_overlay {
        output.push_str("        overlays = [ (import rust-overlay) ];\n");
        output.push_str("        pkgs = import nixpkgs { inherit system overlays; };\n");
    } else {
        output.push_str("        pkgs = import nixpkgs { inherit system; };\n");
    }
    output.push_str("        lib = pkgs.lib;\n\n");

    if need_go {
        let want_go_attr = go_want_attr.as_deref().unwrap_or("go");
        writeln_nix_string(&mut output, "        wantGoAttr", want_go_attr);
        output.push_str(
            "        goAttr = if builtins.hasAttr wantGoAttr pkgs then wantGoAttr else \"go\";\n",
        );
        output.push_str("        go = pkgs.${goAttr};\n\n");
    }

    if need_python {
        let want_python_attr = python_want_attr.as_deref().unwrap_or("python3");
        writeln_nix_string(&mut output, "        wantPythonAttr", want_python_attr);
        output.push_str("        pythonAttr = if builtins.hasAttr wantPythonAttr pkgs then wantPythonAttr else \"python3\";\n");
        output.push_str("        python = pkgs.${pythonAttr};\n");
        output.push_str("        wantPythonPackagesAttr = \"${pythonAttr}Packages\";\n");
        output.push_str(
            "        pythonPackages = if builtins.hasAttr wantPythonPackagesAttr pkgs then pkgs.${wantPythonPackagesAttr} else pkgs.python3Packages;\n\n",
        );
    }

    if need_node {
        let want_node_attr = node_want_attr.as_deref().unwrap_or("nodejs");
        writeln_nix_string(&mut output, "        wantNodeAttr", want_node_attr);
        output.push_str("        nodeAttr = if builtins.hasAttr wantNodeAttr pkgs then wantNodeAttr else \"nodejs\";\n");
        output.push_str("        node = pkgs.${nodeAttr};\n\n");
    }

    if uses_rust_overlay {
        if let Some(want) = rust_want_version.as_deref() {
            writeln_nix_string(&mut output, "        wantRustVersion", want);
            output.push_str(
                "        rustToolchainBase = if builtins.hasAttr wantRustVersion pkgs.rust-bin.stable\n",
            );
            output.push_str("          then pkgs.rust-bin.stable.${wantRustVersion}.default\n");
            output.push_str("          else pkgs.rust-bin.stable.latest.default;\n");
        } else {
            output.push_str("        rustToolchainBase = pkgs.rust-bin.stable.latest.default;\n");
        }

        output.push_str("        rustToolchain = rustToolchainBase.override {\n");
        output.push_str("          extensions = [ \"rust-src\" \"rust-analyzer\" ];\n");
        output.push_str("        };\n\n");
    }

    let mut dev_packages_lines: Vec<String> = Vec::new();

    if need_go {
        dev_packages_lines.push("            go".to_string());
        dev_packages_lines.push("            pkgs.gopls".to_string());
    }

    if need_python {
        dev_packages_lines.push("            python".to_string());
    }

    if need_node {
        dev_packages_lines.push("            node".to_string());
        dev_packages_lines.push("            pkgs.nodePackages.typescript".to_string());
        dev_packages_lines
            .push("            pkgs.nodePackages.typescript-language-server".to_string());
    }

    if uses_rust_overlay {
        dev_packages_lines.push("            rustToolchain".to_string());
    }

    if required_task_runner_tools.contains("gnumake") {
        dev_packages_lines.push("            pkgs.gnumake".to_string());
    }
    if required_task_runner_tools.contains("just") {
        dev_packages_lines.push("            pkgs.just".to_string());
    }
    if required_task_runner_tools.contains("go-task") {
        dev_packages_lines.push("            pkgs.go-task".to_string());
    }

    if need_python && required_task_runner_tools.contains("tox") {
        output.push_str(
            "        tox = if builtins.hasAttr \"tox\" pythonPackages then pythonPackages.tox else null;\n",
        );
    }
    if need_python && required_task_runner_tools.contains("nox") {
        output.push_str(
            "        nox = if builtins.hasAttr \"nox\" pythonPackages then pythonPackages.nox else null;\n",
        );
    }
    if need_python && required_task_runner_tools.contains("invoke") {
        output.push_str(
            "        invoke = if builtins.hasAttr \"invoke\" pythonPackages then pythonPackages.invoke else null;\n",
        );
    }

    if need_node {
        if required_package_managers.contains(&PackageManager::Pnpm) {
            output.push_str(
                "        pnpm = if builtins.hasAttr \"pnpm\" pkgs.nodePackages then pkgs.nodePackages.pnpm else null;\n",
            );
        }
        if required_package_managers.contains(&PackageManager::Yarn) {
            output.push_str(
                "        yarn = if builtins.hasAttr \"yarn\" pkgs then pkgs.yarn else null;\n",
            );
        }
        if required_package_managers.contains(&PackageManager::Bun) {
            output.push_str(
                "        bun = if builtins.hasAttr \"bun\" pkgs then pkgs.bun else null;\n",
            );
        }
        if required_package_managers.contains(&PackageManager::Deno) {
            output.push_str(
                "        deno = if builtins.hasAttr \"deno\" pkgs then pkgs.deno else null;\n",
            );
        }
    }

    if need_python {
        if required_package_managers.contains(&PackageManager::Poetry) {
            output.push_str(
                "        poetry = if builtins.hasAttr \"poetry\" pkgs then pkgs.poetry else null;\n",
            );
        }
        if required_package_managers.contains(&PackageManager::Uv) {
            output
                .push_str("        uv = if builtins.hasAttr \"uv\" pkgs then pkgs.uv else null;\n");
        }
        if required_package_managers.contains(&PackageManager::Pdm) {
            output.push_str(
                "        pdm = if builtins.hasAttr \"pdm\" pkgs then pkgs.pdm else null;\n",
            );
        }
        if required_package_managers.contains(&PackageManager::Pipenv) {
            output.push_str(
                "        pipenv = if builtins.hasAttr \"pipenv\" pkgs then pkgs.pipenv else null;\n",
            );
        }
    }

    if need_node {
        for tool in &required_node_tools {
            let var = format!("nodeTool{}", tool.to_ascii_uppercase());
            writeln_nix_string(&mut output, &format!("        {var}"), tool);
            output.push_str(&format!(
                "        {var}Pkg = if builtins.hasAttr \"{tool}\" pkgs.nodePackages then pkgs.nodePackages.{tool} else null;\n",
            ));
        }
        output.push('\n');
    }

    if need_python {
        output.push_str(
            "        pyright = if builtins.hasAttr \"pyright\" pkgs then pkgs.pyright\n          else if builtins.hasAttr \"pyright\" pkgs.nodePackages then pkgs.nodePackages.pyright\n          else null;\n\n",
        );
    }

    output.push_str("        devPackages =\n");
    output.push_str("          [\n");
    for line in dev_packages_lines {
        output.push_str(&format!("{line}\n"));
    }
    output.push_str("          ]");

    if need_python && required_task_runner_tools.contains("tox") {
        output.push_str("\n          ++ lib.optional (tox != null) tox");
    }
    if need_python && required_task_runner_tools.contains("nox") {
        output.push_str("\n          ++ lib.optional (nox != null) nox");
    }
    if need_python && required_task_runner_tools.contains("invoke") {
        output.push_str("\n          ++ lib.optional (invoke != null) invoke");
    }

    if need_node {
        if required_package_managers.contains(&PackageManager::Pnpm) {
            output.push_str("\n          ++ lib.optional (pnpm != null) pnpm");
        }
        if required_package_managers.contains(&PackageManager::Yarn) {
            output.push_str("\n          ++ lib.optional (yarn != null) yarn");
        }
        if required_package_managers.contains(&PackageManager::Bun) {
            output.push_str("\n          ++ lib.optional (bun != null) bun");
        }
        if required_package_managers.contains(&PackageManager::Deno) {
            output.push_str("\n          ++ lib.optional (deno != null) deno");
        }
    }

    if need_python {
        if required_package_managers.contains(&PackageManager::Poetry) {
            output.push_str("\n          ++ lib.optional (poetry != null) poetry");
        }
        if required_package_managers.contains(&PackageManager::Uv) {
            output.push_str("\n          ++ lib.optional (uv != null) uv");
        }
        if required_package_managers.contains(&PackageManager::Pdm) {
            output.push_str("\n          ++ lib.optional (pdm != null) pdm");
        }
        if required_package_managers.contains(&PackageManager::Pipenv) {
            output.push_str("\n          ++ lib.optional (pipenv != null) pipenv");
        }
    }

    if need_node {
        for tool in &required_node_tools {
            let var = format!("nodeTool{}Pkg", tool.to_ascii_uppercase());
            output.push_str(&format!(
                "\n          ++ lib.optional ({var} != null) {var}"
            ));
        }
    }

    if need_python {
        output.push_str("\n          ++ lib.optional (pyright != null) pyright");
    }

    output.push_str(";\n\n");

    output.push_str("      in\n");
    output.push_str("      {\n");

    output.push_str("        devShells.default = pkgs.mkShell {\n");
    output.push_str("          packages = devPackages;\n");

    let need_shell_hook =
        !notices.is_empty() || need_go || need_python || need_node || uses_rust_overlay;
    if !need_shell_hook {
        output.push_str("        };\n\n");
    } else {
        output.push_str("          shellHook = ''\n");
        output.push_str("            echo \"autonix: generated devShell (best-effort)\"\n");

        if need_go {
            output.push_str(
                "            echo \"autonix: Go attr: ${goAttr} (requested ${wantGoAttr})\"\n",
            );
            output.push_str("            if [ \"${goAttr}\" != \"${wantGoAttr}\" ]; then\n");
            output.push_str(
                "              echo \"autonix: NOTE: ${wantGoAttr} not found; using ${goAttr}\"\n",
            );
            output.push_str("            fi\n");
        }

        if need_python {
            output.push_str(
                "            echo \"autonix: Python attr: ${pythonAttr} (requested ${wantPythonAttr})\"\n",
            );
            output
                .push_str("            if [ \"${pythonAttr}\" != \"${wantPythonAttr}\" ]; then\n");
            output.push_str(
                "              echo \"autonix: NOTE: ${wantPythonAttr} not found; using ${pythonAttr}\"\n",
            );
            output.push_str("            fi\n");
        }

        if need_node {
            output.push_str(
                "            echo \"autonix: Node attr: ${nodeAttr} (requested ${wantNodeAttr})\"\n",
            );
            output.push_str("            if [ \"${nodeAttr}\" != \"${wantNodeAttr}\" ]; then\n");
            output.push_str(
                "              echo \"autonix: NOTE: ${wantNodeAttr} not found; using ${nodeAttr}\"\n",
            );
            output.push_str("            fi\n");
        }

        if uses_rust_overlay {
            output
                .push_str("            echo \"autonix: Rust toolchain enabled (rust-overlay)\"\n");
        }

        for msg in notices {
            let escaped = nix_escape_string(&msg);
            output.push_str(&format!(
                "            echo ${{lib.escapeShellArg \"{escaped}\"}}\n",
            ));
        }
        output.push_str("          '';\n");
        output.push_str("        };\n\n");
    }

    output.push_str("        checks = {\n");
    for check in checks {
        output.push_str(&render_check(&check));
    }
    output.push_str("        };\n");

    output.push_str("      });\n");
    output.push_str("}\n");

    output
}

const GO_VERSION_SOURCES: &[VersionSource] =
    &[VersionSource::GoModDirective, VersionSource::GoVersionFile];

const PYTHON_VERSION_SOURCES: &[VersionSource] = &[
    VersionSource::PyprojectRequiresPython,
    VersionSource::PythonVersionFile,
    VersionSource::PipfilePythonVersion,
    VersionSource::SetupPyPythonRequires,
];

const NODE_VERSION_SOURCES: &[VersionSource] = &[
    VersionSource::PackageJsonEnginesNode,
    VersionSource::NvmrcFile,
    VersionSource::NodeVersionFile,
];

const RUST_VERSION_SOURCES: &[VersionSource] = &[
    VersionSource::RustToolchainFile,
    VersionSource::RustToolchainToml,
    VersionSource::CargoTomlRustVersion,
];

fn detected_languages(metadata: &ProjectMetadata) -> HashSet<Language> {
    metadata
        .languages
        .iter()
        .map(|l| l.language.clone())
        .collect()
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

fn resolve_js_package_manager(package_json_path: &Path) -> PackageManager {
    if let Ok(content) = fs::read_to_string(package_json_path)
        && let Ok(parsed) = serde_json::from_str::<JsonValue>(&content)
        && let Some(package_manager_str) = parsed.get("packageManager").and_then(|v| v.as_str())
    {
        let name = package_manager_str
            .split('@')
            .next()
            .unwrap_or(package_manager_str);

        match name {
            "npm" => return PackageManager::Npm,
            "pnpm" => return PackageManager::Pnpm,
            "yarn" => return PackageManager::Yarn,
            "bun" => return PackageManager::Bun,
            _ => {}
        }
    }

    let Some(parent) = package_json_path.parent() else {
        return PackageManager::Npm;
    };

    if parent.join("bun.lockb").exists() || parent.join("bun.lock").exists() {
        return PackageManager::Bun;
    }
    if parent.join("pnpm-lock.yaml").exists() {
        return PackageManager::Pnpm;
    }
    if parent.join("yarn.lock").exists() {
        return PackageManager::Yarn;
    }
    if parent.join("package-lock.json").exists() {
        return PackageManager::Npm;
    }

    PackageManager::Npm
}

fn package_manager_command(pm: PackageManager) -> &'static str {
    match pm {
        PackageManager::Npm => "npm",
        PackageManager::Pnpm => "pnpm",
        PackageManager::Yarn => "yarn",
        PackageManager::Bun => "bun",
        PackageManager::Deno => "deno",
        PackageManager::Pip
        | PackageManager::Uv
        | PackageManager::Poetry
        | PackageManager::Pdm
        | PackageManager::Pipenv
        | PackageManager::Cargo
        | PackageManager::Go => "",
    }
}

fn build_package_manager_run(pm: PackageManager, script_name: &str) -> String {
    match pm {
        PackageManager::Npm => format!("npm run {script_name}"),
        PackageManager::Pnpm => format!("pnpm run {script_name}"),
        PackageManager::Yarn => format!("yarn run {script_name}"),
        PackageManager::Bun => format!("bun run {script_name}"),
        _ => format!("npm run {script_name}"),
    }
}

fn collect_checks(
    metadata: &ProjectMetadata,
    root: &Path,
    required_package_managers: &mut HashSet<PackageManager>,
    need_node: &mut bool,
    need_python: &mut bool,
) -> Vec<CheckSpec> {
    let mut specs = Vec::new();
    let mut key_counts: HashMap<String, usize> = HashMap::new();

    for tr in &metadata.task_runners {
        let runner_path = relativize_path(root, &tr.path).unwrap_or_else(|| tr.path.clone());
        let runner_slug = slugify_path(&runner_path);

        for (category, cmds) in [("test", &tr.commands.test), ("build", &tr.commands.build)] {
            for cmd in cmds {
                let (required_exec, command, workdir, display, pm_used) =
                    resolve_command(cmd, root);

                if let Some(pm) = pm_used {
                    required_package_managers.insert(pm);
                    if matches!(
                        pm,
                        PackageManager::Npm
                            | PackageManager::Pnpm
                            | PackageManager::Yarn
                            | PackageManager::Bun
                    ) {
                        *need_node = true;
                    }
                }

                if required_exec == "tox" || required_exec == "nox" || required_exec == "invoke" {
                    *need_python = true;
                }

                let runner_name = task_runner_name(tr.task_runner).to_ascii_lowercase();
                let mut key = format!(
                    "{category}-{runner_name}-{}-{runner_slug}",
                    slugify_identifier(&cmd.name)
                );
                let count = key_counts.entry(key.clone()).or_insert(0);
                *count += 1;
                if *count > 1 {
                    key = format!("{key}-{}", *count);
                }

                let derivation_name = format!("check-{key}");

                specs.push(CheckSpec {
                    key,
                    derivation_name,
                    display,
                    required_exec,
                    command,
                    workdir,
                });
            }
        }
    }

    specs.sort_by(|a, b| a.key.cmp(&b.key));
    specs
}

fn resolve_command(
    cmd: &TaskCommand,
    root: &Path,
) -> (String, String, String, String, Option<PackageManager>) {
    match &cmd.executable {
        CommandExecutable::Direct { command } => {
            let required_exec = command_first_word(command).unwrap_or("").to_string();
            (
                required_exec,
                command.clone(),
                ".".to_string(),
                command.clone(),
                None,
            )
        }
        CommandExecutable::PackageManagerScript {
            script_name,
            script_body,
            package_json_path,
        } => {
            let pm = resolve_js_package_manager(package_json_path);
            let pm_cmd = package_manager_command(pm);

            let run_command = build_package_manager_run(pm, script_name);

            let workdir = package_json_path
                .parent()
                .and_then(|p| relativize_path(root, p))
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string());

            let display = format!("{run_command}  # {script_body}");

            (pm_cmd.to_string(), run_command, workdir, display, Some(pm))
        }
    }
}

fn render_check(check: &CheckSpec) -> String {
    let mut out = String::new();

    let key_escaped = nix_escape_string(&check.key);
    let drv_escaped = nix_escape_string(&check.derivation_name);
    let display_escaped = nix_escape_string(&check.display);
    let required_exec_escaped = nix_escape_string(&check.required_exec);
    let cmd_escaped = nix_escape_string(&check.command);
    let workdir_escaped = nix_escape_string(&check.workdir);

    out.push_str(&format!("          \"{key_escaped}\" = let\n"));
    out.push_str(&format!("            cmd = \"{cmd_escaped}\";\n"));
    out.push_str(&format!(
        "            requiredExec = \"{required_exec_escaped}\";\n"
    ));
    out.push_str(&format!("            workdir = \"{workdir_escaped}\";\n"));
    out.push_str(&format!("            display = \"{display_escaped}\";\n"));
    out.push_str("          in pkgs.runCommand \"");
    out.push_str(&drv_escaped);
    out.push_str("\" {\n");
    out.push_str("            nativeBuildInputs = devPackages;\n");
    out.push_str("          } ''\n");
    out.push_str("            set -euo pipefail\n");
    out.push_str("            export HOME=\"$TMPDIR/home\"\n");
    out.push_str("            mkdir -p \"$HOME\"\n");
    out.push('\n');
    out.push_str("            echo \"autonix: running ${display}\"\n");
    out.push_str("            if ! command -v \"${requiredExec}\" >/dev/null 2>&1; then\n");
    out.push_str("              echo \"autonix: missing required executable in PATH: ${requiredExec}\" >&2\n");
    out.push_str("              exit 1\n");
    out.push_str("            fi\n");
    out.push('\n');
    out.push_str("            cp -r ${./.} source\n");
    out.push_str("            chmod -R u+w source\n");
    out.push_str("            cd \"source/${workdir}\"\n");
    out.push('\n');
    out.push_str("            ${pkgs.bash}/bin/bash -lc ${lib.escapeShellArg cmd}\n");
    out.push('\n');
    out.push_str("            mkdir -p $out\n");
    out.push_str("            echo ok > $out/result\n");
    out.push_str("          '';\n");

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

fn writeln_nix_string(output: &mut String, name: &str, value: &str) {
    let escaped = nix_escape_string(value);
    output.push_str(&format!("{name} = \"{escaped}\";\n"));
}

fn nix_escape_string(input: &str) -> String {
    let mut out = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '$' if matches!(chars.peek(), Some('{')) => out.push_str("\\$"),
            _ => out.push(ch),
        }
    }

    out
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

        assert!(flake.contains("rust-overlay"));
        assert!(flake.contains("rust-bin.stable.latest.default"));
        assert!(flake.contains("rust-analyzer"));
        assert!(flake.contains("rust-src"));
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

        assert!(flake.contains("pnpm run test"));
        assert!(flake.contains("jest"));
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

        assert!(flake.contains("pnpm run test"));
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

        assert!(flake.contains("npm run test"));
    }

    #[test]
    fn includes_versioned_go_when_available() {
        let dir = TempDir::new().unwrap();
        create_temp_file(&dir, "go.mod", "module example.com\n\ngo 1.21\n");
        create_temp_file(&dir, "main.go", "package main\nfunc main(){}\n");

        let engine = DetectionEngine;
        let metadata = engine.detect(dir.path());
        let flake = generate_dev_flake(&metadata, dir.path());

        assert!(flake.contains("go_1_21"));
    }
}
