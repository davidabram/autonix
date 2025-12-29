use crate::detection::VersionSource;

pub const NODE_TOOL_VITE: &str = "vite";
pub const NODE_TOOL_WEBPACK: &str = "webpack";
pub const NODE_TOOL_RSPACK: &str = "rspack";
pub const NODE_TOOL_ROLLUP: &str = "rollup";
pub const NODE_TOOL_TURBO: &str = "turbo";
pub const NODE_TOOL_NX: &str = "nx";

pub const GENERIC_TOOL_GNUMAKE: &str = "gnumake";
pub const GENERIC_TOOL_JUST: &str = "just";
pub const GENERIC_TOOL_GO_TASK: &str = "go-task";

pub const PYTHON_TOOL_TOX: &str = "tox";
pub const PYTHON_TOOL_NOX: &str = "nox";
pub const PYTHON_TOOL_INVOKE: &str = "invoke";

pub const GO_TOOL_GOPLS: &str = "gopls";
pub const NODE_PKG_TYPESCRIPT: &str = "typescript";
pub const NODE_PKG_TYPESCRIPT_LS: &str = "typescript-language-server";

pub const GO_VERSION_SOURCES: &[VersionSource] =
    &[VersionSource::GoModDirective, VersionSource::GoVersionFile];

pub const PYTHON_VERSION_SOURCES: &[VersionSource] = &[
    VersionSource::PyprojectRequiresPython,
    VersionSource::PythonVersionFile,
    VersionSource::PipfilePythonVersion,
    VersionSource::SetupPyPythonRequires,
];

pub const NODE_VERSION_SOURCES: &[VersionSource] = &[
    VersionSource::PackageJsonEnginesNode,
    VersionSource::NvmrcFile,
    VersionSource::NodeVersionFile,
];

pub const RUST_VERSION_SOURCES: &[VersionSource] = &[
    VersionSource::RustToolchainFile,
    VersionSource::RustToolchainToml,
    VersionSource::CargoTomlRustVersion,
];
