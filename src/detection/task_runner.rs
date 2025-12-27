use regex::Regex;
use serde::Serialize;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandCategory {
    Test,
    Build,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "PascalCase")]
pub enum TaskRunner {
    // Universal
    Make,
    Just,
    Task,

    // JavaScript/TypeScript
    NpmScripts,
    Vite,
    Webpack,
    Rspack,
    Rollup,
    Turbo,
    Nx,

    // Python
    Tox,
    Nox,
    Invoke,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum TaskRunnerSource {
    // Universal
    Makefile,
    Justfile,
    TaskfileYml,
    TaskfileYaml,

    // JavaScript
    PackageJson,
    ViteConfigJs,
    ViteConfigTs,
    ViteConfigMjs,
    WebpackConfigJs,
    WebpackConfigTs,
    WebpackConfigMjs,
    WebpackConfigCjs,
    RspackConfigJs,
    RspackConfigTs,
    RspackConfigMjs,
    RspackConfigCjs,
    RollupConfigJs,
    RollupConfigMjs,
    RollupConfigCjs,
    RollupConfigTs,
    TurboJson,
    NxJson,

    // Python
    ToxIni,
    NoxPy,
    Noxfile,
    TasksPy,
    InvokeYaml,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskCommand {
    pub name: String,
    pub command: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct TaskRunnerCommands {
    pub test: Vec<TaskCommand>,
    pub build: Vec<TaskCommand>,
    pub other: Vec<TaskCommand>,
}

impl TaskRunnerCommands {
    fn add_command(&mut self, cmd: TaskCommand, category: CommandCategory) {
        match category {
            CommandCategory::Test => self.test.push(cmd),
            CommandCategory::Build => self.build.push(cmd),
            CommandCategory::Other => self.other.push(cmd),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskRunnerFile {
    pub task_runner: TaskRunner,
    pub source: TaskRunnerSource,
    pub path: PathBuf,
    pub content: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TaskRunnerDetection {
    pub task_runner: TaskRunner,
    pub source: TaskRunnerSource,
    pub path: PathBuf,
    pub commands: TaskRunnerCommands,
}

impl From<&TaskRunnerSource> for TaskRunner {
    fn from(source: &TaskRunnerSource) -> Self {
        match source {
            TaskRunnerSource::Makefile => TaskRunner::Make,
            TaskRunnerSource::Justfile => TaskRunner::Just,
            TaskRunnerSource::TaskfileYml | TaskRunnerSource::TaskfileYaml => TaskRunner::Task,
            TaskRunnerSource::PackageJson => TaskRunner::NpmScripts,
            TaskRunnerSource::ViteConfigJs
            | TaskRunnerSource::ViteConfigTs
            | TaskRunnerSource::ViteConfigMjs => TaskRunner::Vite,
            TaskRunnerSource::WebpackConfigJs
            | TaskRunnerSource::WebpackConfigTs
            | TaskRunnerSource::WebpackConfigMjs
            | TaskRunnerSource::WebpackConfigCjs => TaskRunner::Webpack,
            TaskRunnerSource::RspackConfigJs
            | TaskRunnerSource::RspackConfigTs
            | TaskRunnerSource::RspackConfigMjs
            | TaskRunnerSource::RspackConfigCjs => TaskRunner::Rspack,
            TaskRunnerSource::RollupConfigJs
            | TaskRunnerSource::RollupConfigMjs
            | TaskRunnerSource::RollupConfigCjs
            | TaskRunnerSource::RollupConfigTs => TaskRunner::Rollup,
            TaskRunnerSource::TurboJson => TaskRunner::Turbo,
            TaskRunnerSource::NxJson => TaskRunner::Nx,
            TaskRunnerSource::ToxIni => TaskRunner::Tox,
            TaskRunnerSource::NoxPy | TaskRunnerSource::Noxfile => TaskRunner::Nox,
            TaskRunnerSource::TasksPy | TaskRunnerSource::InvokeYaml => TaskRunner::Invoke,
        }
    }
}

impl TryFrom<PathBuf> for TaskRunnerFile {
    type Error = ();

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        if !path.is_file() {
            return Err(());
        }

        let filename = path.file_name().ok_or(())?.to_str().ok_or(())?;

        let source = match filename {
            // Universal
            "Makefile" | "makefile" | "GNUmakefile" => TaskRunnerSource::Makefile,
            "justfile" | "Justfile" => TaskRunnerSource::Justfile,
            "Taskfile.yml" => TaskRunnerSource::TaskfileYml,
            "Taskfile.yaml" => TaskRunnerSource::TaskfileYaml,

            // JavaScript/TypeScript
            "package.json" => TaskRunnerSource::PackageJson,
            "vite.config.js" => TaskRunnerSource::ViteConfigJs,
            "vite.config.ts" | "vite.config.mts" => TaskRunnerSource::ViteConfigTs,
            "vite.config.mjs" => TaskRunnerSource::ViteConfigMjs,
            "webpack.config.js" => TaskRunnerSource::WebpackConfigJs,
            "webpack.config.ts" | "webpack.config.mts" => TaskRunnerSource::WebpackConfigTs,
            "webpack.config.mjs" => TaskRunnerSource::WebpackConfigMjs,
            "webpack.config.cjs" => TaskRunnerSource::WebpackConfigCjs,
            "rspack.config.js" => TaskRunnerSource::RspackConfigJs,
            "rspack.config.ts" | "rspack.config.mts" => TaskRunnerSource::RspackConfigTs,
            "rspack.config.mjs" => TaskRunnerSource::RspackConfigMjs,
            "rspack.config.cjs" => TaskRunnerSource::RspackConfigCjs,
            "rollup.config.js" => TaskRunnerSource::RollupConfigJs,
            "rollup.config.mjs" => TaskRunnerSource::RollupConfigMjs,
            "rollup.config.cjs" => TaskRunnerSource::RollupConfigCjs,
            "rollup.config.ts" | "rollup.config.mts" => TaskRunnerSource::RollupConfigTs,
            "turbo.json" => TaskRunnerSource::TurboJson,
            "nx.json" => TaskRunnerSource::NxJson,

            // Python
            "tox.ini" => TaskRunnerSource::ToxIni,
            "nox.py" => TaskRunnerSource::NoxPy,
            "noxfile.py" => TaskRunnerSource::Noxfile,
            "tasks.py" => TaskRunnerSource::TasksPy,
            "invoke.yaml" | "invoke.yml" => TaskRunnerSource::InvokeYaml,

            _ => return Err(()),
        };

        let content = fs::read_to_string(&path).ok();

        let task_runner = TaskRunner::from(&source);

        Ok(TaskRunnerFile {
            task_runner,
            source,
            path,
            content,
        })
    }
}

impl TaskRunnerFile {
    fn extract_commands(&self) -> TaskRunnerCommands {
        let Some(content) = &self.content else {
            return TaskRunnerCommands::default();
        };

        match self.source {
            TaskRunnerSource::Makefile => extract_makefile_commands(content),
            TaskRunnerSource::Justfile => extract_justfile_commands(content),
            TaskRunnerSource::PackageJson => extract_npm_commands(content),
            TaskRunnerSource::TaskfileYml | TaskRunnerSource::TaskfileYaml => {
                extract_taskfile_commands(content)
            }
            TaskRunnerSource::ViteConfigJs
            | TaskRunnerSource::ViteConfigTs
            | TaskRunnerSource::ViteConfigMjs => get_vite_commands(),
            TaskRunnerSource::WebpackConfigJs
            | TaskRunnerSource::WebpackConfigTs
            | TaskRunnerSource::WebpackConfigMjs
            | TaskRunnerSource::WebpackConfigCjs => get_webpack_commands(),
            TaskRunnerSource::RspackConfigJs
            | TaskRunnerSource::RspackConfigTs
            | TaskRunnerSource::RspackConfigMjs
            | TaskRunnerSource::RspackConfigCjs => get_rspack_commands(),
            TaskRunnerSource::RollupConfigJs
            | TaskRunnerSource::RollupConfigMjs
            | TaskRunnerSource::RollupConfigCjs
            | TaskRunnerSource::RollupConfigTs => get_rollup_commands(),
            TaskRunnerSource::TurboJson => extract_turbo_commands(content),
            _ => TaskRunnerCommands::default(),
        }
    }
}

const TEST_KEYWORDS: &[&str] = &["test", "spec", "jest", "mocha", "pytest"];
const BUILD_KEYWORDS: &[&str] = &["build", "compile", "bundle", "package", "dist"];

fn classify_command(name: &str) -> CommandCategory {
    let name_lower = name.to_lowercase();

    if TEST_KEYWORDS.iter().any(|&kw| name_lower.contains(kw)) {
        CommandCategory::Test
    } else if BUILD_KEYWORDS.iter().any(|&kw| name_lower.contains(kw)) {
        CommandCategory::Build
    } else {
        CommandCategory::Other
    }
}

fn extract_makefile_commands(content: &str) -> TaskRunnerCommands {
    let mut commands = TaskRunnerCommands::default();

    let target_re = Regex::new(r"^([a-zA-Z0-9_-]+)\s*:").unwrap();

    for line in content.lines() {
        if let Some(caps) = target_re.captures(line) {
            let target = caps.get(1).unwrap().as_str();

            if target.starts_with('.') {
                continue;
            }

            let cmd = TaskCommand {
                name: target.to_string(),
                command: format!("make {}", target),
                description: None,
            };

            commands.add_command(cmd, classify_command(target));
        }
    }

    commands
}

fn extract_justfile_commands(content: &str) -> TaskRunnerCommands {
    let mut commands = TaskRunnerCommands::default();

    let recipe_re = Regex::new(r"^([a-zA-Z0-9_-]+)\s*:").unwrap();

    for line in content.lines() {
        if let Some(caps) = recipe_re.captures(line) {
            let recipe = caps.get(1).unwrap().as_str();

            let cmd = TaskCommand {
                name: recipe.to_string(),
                command: format!("just {}", recipe),
                description: None,
            };

            commands.add_command(cmd, classify_command(recipe));
        }
    }

    commands
}

fn extract_npm_commands(content: &str) -> TaskRunnerCommands {
    let mut commands = TaskRunnerCommands::default();

    let Ok(json) = serde_json::from_str::<JsonValue>(content) else {
        return commands;
    };

    let Some(scripts) = json.get("scripts").and_then(|s| s.as_object()) else {
        return commands;
    };

    for (name, command_val) in scripts {
        let Some(command_str) = command_val.as_str() else {
            continue;
        };

        let cmd = TaskCommand {
            name: name.clone(),
            command: command_str.to_string(),
            description: None,
        };

        commands.add_command(cmd, classify_command(name));
    }

    commands
}

fn extract_taskfile_commands(content: &str) -> TaskRunnerCommands {
    let mut commands = TaskRunnerCommands::default();

    let Ok(yaml) = serde_yaml::from_str::<YamlValue>(content) else {
        return commands;
    };

    if let Some(tasks) = yaml.get("tasks").and_then(|t| t.as_mapping()) {
        for (task_name, _task_data) in tasks {
            if let Some(name) = task_name.as_str() {
                let cmd = TaskCommand {
                    name: name.to_string(),
                    command: format!("task {}", name),
                    description: None,
                };

                commands.add_command(cmd, classify_command(name));
            }
        }
    }

    commands
}

fn get_vite_commands() -> TaskRunnerCommands {
    let mut commands = TaskRunnerCommands::default();

    let dev_cmd = TaskCommand {
        name: "dev".to_string(),
        command: "vite".to_string(),
        description: Some("Start dev server".to_string()),
    };
    commands.add_command(dev_cmd, CommandCategory::Other);

    let build_cmd = TaskCommand {
        name: "build".to_string(),
        command: "vite build".to_string(),
        description: Some("Build for production".to_string()),
    };
    commands.add_command(build_cmd, CommandCategory::Build);

    let preview_cmd = TaskCommand {
        name: "preview".to_string(),
        command: "vite preview".to_string(),
        description: Some("Preview production build".to_string()),
    };
    commands.add_command(preview_cmd, CommandCategory::Other);

    commands
}

fn get_webpack_commands() -> TaskRunnerCommands {
    let mut commands = TaskRunnerCommands::default();

    let build_cmd = TaskCommand {
        name: "build".to_string(),
        command: "webpack build".to_string(),
        description: Some("Build for production".to_string()),
    };
    commands.add_command(build_cmd, CommandCategory::Build);

    let serve_cmd = TaskCommand {
        name: "serve".to_string(),
        command: "webpack serve".to_string(),
        description: Some("Start dev server".to_string()),
    };
    commands.add_command(serve_cmd, CommandCategory::Other);

    let watch_cmd = TaskCommand {
        name: "watch".to_string(),
        command: "webpack watch".to_string(),
        description: Some("Watch for file changes".to_string()),
    };
    commands.add_command(watch_cmd, CommandCategory::Other);

    commands
}

fn get_rspack_commands() -> TaskRunnerCommands {
    let mut commands = TaskRunnerCommands::default();

    let dev_cmd = TaskCommand {
        name: "dev".to_string(),
        command: "rspack dev".to_string(),
        description: Some("Start dev server".to_string()),
    };
    commands.add_command(dev_cmd, CommandCategory::Other);

    let build_cmd = TaskCommand {
        name: "build".to_string(),
        command: "rspack build".to_string(),
        description: Some("Build for production".to_string()),
    };
    commands.add_command(build_cmd, CommandCategory::Build);

    let preview_cmd = TaskCommand {
        name: "preview".to_string(),
        command: "rspack preview".to_string(),
        description: Some("Preview production build".to_string()),
    };
    commands.add_command(preview_cmd, CommandCategory::Other);

    commands
}

fn get_rollup_commands() -> TaskRunnerCommands {
    let mut commands = TaskRunnerCommands::default();

    let build_cmd = TaskCommand {
        name: "build".to_string(),
        command: "rollup -c".to_string(),
        description: Some("Build bundle".to_string()),
    };
    commands.add_command(build_cmd, CommandCategory::Build);

    let watch_cmd = TaskCommand {
        name: "watch".to_string(),
        command: "rollup -c -w".to_string(),
        description: Some("Watch and rebuild on changes".to_string()),
    };
    commands.add_command(watch_cmd, CommandCategory::Other);

    commands
}

fn extract_turbo_commands(content: &str) -> TaskRunnerCommands {
    let mut commands = TaskRunnerCommands::default();

    let Ok(json) = serde_json::from_str::<JsonValue>(content) else {
        return commands;
    };

    let tasks = json
        .get("pipeline")
        .or_else(|| json.get("tasks"))
        .and_then(|t| t.as_object());

    let Some(tasks) = tasks else {
        return commands;
    };

    for (task_name, _task_config) in tasks {
        let cmd = TaskCommand {
            name: task_name.clone(),
            command: format!("turbo run {}", task_name),
            description: None,
        };

        commands.add_command(cmd, classify_command(task_name));
    }

    commands
}

impl From<TaskRunnerFile> for TaskRunnerDetection {
    fn from(file: TaskRunnerFile) -> Self {
        let commands = file.extract_commands();

        TaskRunnerDetection {
            task_runner: file.task_runner,
            source: file.source,
            path: file.path,
            commands,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_task_runner_from_source_makefile() {
        assert_eq!(
            TaskRunner::from(&TaskRunnerSource::Makefile),
            TaskRunner::Make
        );
    }

    #[test]
    fn test_task_runner_from_source_justfile() {
        assert_eq!(
            TaskRunner::from(&TaskRunnerSource::Justfile),
            TaskRunner::Just
        );
    }

    #[test]
    fn test_task_runner_from_source_taskfile() {
        assert_eq!(
            TaskRunner::from(&TaskRunnerSource::TaskfileYml),
            TaskRunner::Task
        );
        assert_eq!(
            TaskRunner::from(&TaskRunnerSource::TaskfileYaml),
            TaskRunner::Task
        );
    }

    #[test]
    fn test_task_runner_from_source_npm() {
        assert_eq!(
            TaskRunner::from(&TaskRunnerSource::PackageJson),
            TaskRunner::NpmScripts
        );
    }

    #[test]
    fn test_task_runner_commands_default() {
        let commands = TaskRunnerCommands::default();
        assert!(commands.test.is_empty());
        assert!(commands.build.is_empty());
        assert!(commands.other.is_empty());
    }

    #[test]
    fn test_try_from_makefile() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(&dir, "Makefile", "test:\n\techo test");
        let result = TaskRunnerFile::try_from(path);
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.task_runner, TaskRunner::Make);
        assert_eq!(file.source, TaskRunnerSource::Makefile);
        assert!(file.content.is_some());
    }

    #[test]
    fn test_try_from_justfile() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(&dir, "justfile", "test:\n  echo test");
        let result = TaskRunnerFile::try_from(path);
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.task_runner, TaskRunner::Just);
        assert_eq!(file.source, TaskRunnerSource::Justfile);
    }

    #[test]
    fn test_try_from_package_json() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(&dir, "package.json", r#"{"scripts": {"test": "jest"}}"#);
        let result = TaskRunnerFile::try_from(path);
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.task_runner, TaskRunner::NpmScripts);
        assert_eq!(file.source, TaskRunnerSource::PackageJson);
    }

    #[test]
    fn test_try_from_nonexistent_file() {
        let path = PathBuf::from("/nonexistent/path/Makefile");
        let result = TaskRunnerFile::try_from(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_directory() {
        let dir = TempDir::new().unwrap();
        let result = TaskRunnerFile::try_from(dir.path().to_path_buf());
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_unknown_file() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(&dir, "unknown.txt", "content");
        let result = TaskRunnerFile::try_from(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_makefile_commands() {
        let content = r#"
.PHONY: test build

test:
	cargo test

build:
	cargo build --release

lint:
	cargo clippy

fmt:
	cargo fmt
"#;
        let commands = extract_makefile_commands(content);
        assert_eq!(commands.test.len(), 1);
        assert_eq!(commands.build.len(), 1);
        assert_eq!(commands.other.len(), 2);
        assert_eq!(commands.test[0].name, "test");
        assert_eq!(commands.test[0].command, "make test");
    }

    #[test]
    fn test_extract_npm_commands() {
        let content = r#"{
  "scripts": {
    "test": "jest",
    "build": "vite build",
    "dev": "vite dev",
    "lint": "eslint .",
    "format": "prettier --write ."
  }
}"#;
        let commands = extract_npm_commands(content);
        assert_eq!(commands.test.len(), 1);
        assert_eq!(commands.build.len(), 1);
        assert_eq!(commands.other.len(), 3);
        assert_eq!(commands.test[0].name, "test");
        assert_eq!(commands.test[0].command, "jest");
    }

    #[test]
    fn test_classify_command() {
        assert_eq!(classify_command("test"), CommandCategory::Test);
        assert_eq!(classify_command("unit-test"), CommandCategory::Test);
        assert_eq!(classify_command("build"), CommandCategory::Build);
        assert_eq!(classify_command("compile"), CommandCategory::Build);
        assert_eq!(classify_command("dev"), CommandCategory::Other);
        assert_eq!(classify_command("serve"), CommandCategory::Other);
        assert_eq!(classify_command("lint"), CommandCategory::Other);
        assert_eq!(classify_command("eslint"), CommandCategory::Other);
        assert_eq!(classify_command("format"), CommandCategory::Other);
        assert_eq!(classify_command("fmt"), CommandCategory::Other);
        assert_eq!(classify_command("random"), CommandCategory::Other);
    }

    #[test]
    fn test_extract_vite_commands() {
        let commands = get_vite_commands();
        assert_eq!(commands.build.len(), 1);
        assert_eq!(commands.other.len(), 2);
        assert_eq!(commands.test.len(), 0);

        assert_eq!(commands.build[0].name, "build");
        assert_eq!(commands.build[0].command, "vite build");
        assert_eq!(
            commands.build[0].description,
            Some("Build for production".to_string())
        );

        let dev_cmd = commands.other.iter().find(|c| c.name == "dev").unwrap();
        assert_eq!(dev_cmd.command, "vite");
        assert_eq!(dev_cmd.description, Some("Start dev server".to_string()));

        let preview_cmd = commands.other.iter().find(|c| c.name == "preview").unwrap();
        assert_eq!(preview_cmd.command, "vite preview");
        assert_eq!(
            preview_cmd.description,
            Some("Preview production build".to_string())
        );
    }

    #[test]
    fn test_try_from_vite_config() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(&dir, "vite.config.js", "export default {}");
        let result = TaskRunnerFile::try_from(path);
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.task_runner, TaskRunner::Vite);
        assert_eq!(file.source, TaskRunnerSource::ViteConfigJs);
    }

    #[test]
    fn test_vite_file_to_detection() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(&dir, "vite.config.ts", "export default {}");
        let file = TaskRunnerFile::try_from(path).unwrap();
        let detection = TaskRunnerDetection::from(file);

        assert_eq!(detection.task_runner, TaskRunner::Vite);
        assert_eq!(detection.commands.build.len(), 1);
        assert_eq!(detection.commands.other.len(), 2);
        assert_eq!(detection.commands.test.len(), 0);
    }

    #[test]
    fn test_extract_webpack_commands() {
        let commands = get_webpack_commands();
        assert_eq!(commands.build.len(), 1);
        assert_eq!(commands.test.len(), 0);

        assert_eq!(commands.build[0].name, "build");
        assert_eq!(commands.build[0].command, "webpack build");
        assert_eq!(
            commands.build[0].description,
            Some("Build for production".to_string())
        );

        let serve_cmd = commands.other.iter().find(|c| c.name == "serve").unwrap();
        assert_eq!(serve_cmd.command, "webpack serve");
        assert_eq!(serve_cmd.description, Some("Start dev server".to_string()));

        let watch_cmd = commands.other.iter().find(|c| c.name == "watch").unwrap();
        assert_eq!(watch_cmd.command, "webpack watch");
        assert_eq!(
            watch_cmd.description,
            Some("Watch for file changes".to_string())
        );
    }

    #[test]
    fn test_extract_rspack_commands() {
        let commands = get_rspack_commands();
        assert_eq!(commands.build.len(), 1);
        assert_eq!(commands.other.len(), 2);
        assert_eq!(commands.test.len(), 0);

        assert_eq!(commands.build[0].name, "build");
        assert_eq!(commands.build[0].command, "rspack build");
        assert_eq!(
            commands.build[0].description,
            Some("Build for production".to_string())
        );

        let dev_cmd = commands.other.iter().find(|c| c.name == "dev").unwrap();
        assert_eq!(dev_cmd.command, "rspack dev");
        assert_eq!(dev_cmd.description, Some("Start dev server".to_string()));

        let preview_cmd = commands.other.iter().find(|c| c.name == "preview").unwrap();
        assert_eq!(preview_cmd.command, "rspack preview");
        assert_eq!(
            preview_cmd.description,
            Some("Preview production build".to_string())
        );
    }

    #[test]
    fn test_try_from_webpack_config() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(&dir, "webpack.config.js", "module.exports = {}");
        let result = TaskRunnerFile::try_from(path);
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.task_runner, TaskRunner::Webpack);
        assert_eq!(file.source, TaskRunnerSource::WebpackConfigJs);
    }

    #[test]
    fn test_try_from_webpack_config_ts() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(&dir, "webpack.config.ts", "export default {}");
        let result = TaskRunnerFile::try_from(path);
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.task_runner, TaskRunner::Webpack);
        assert_eq!(file.source, TaskRunnerSource::WebpackConfigTs);
    }

    #[test]
    fn test_try_from_rspack_config() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(&dir, "rspack.config.js", "module.exports = {}");
        let result = TaskRunnerFile::try_from(path);
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.task_runner, TaskRunner::Rspack);
        assert_eq!(file.source, TaskRunnerSource::RspackConfigJs);
    }

    #[test]
    fn test_try_from_rspack_config_ts() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(&dir, "rspack.config.ts", "export default {}");
        let result = TaskRunnerFile::try_from(path);
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.task_runner, TaskRunner::Rspack);
        assert_eq!(file.source, TaskRunnerSource::RspackConfigTs);
    }

    #[test]
    fn test_webpack_file_to_detection() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(&dir, "webpack.config.js", "module.exports = {}");
        let file = TaskRunnerFile::try_from(path).unwrap();
        let detection = TaskRunnerDetection::from(file);

        assert_eq!(detection.task_runner, TaskRunner::Webpack);
        assert_eq!(detection.commands.build.len(), 1);
        assert_eq!(detection.commands.other.len(), 2);
        assert_eq!(detection.commands.test.len(), 0);
    }

    #[test]
    fn test_rspack_file_to_detection() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(&dir, "rspack.config.ts", "export default {}");
        let file = TaskRunnerFile::try_from(path).unwrap();
        let detection = TaskRunnerDetection::from(file);

        assert_eq!(detection.task_runner, TaskRunner::Rspack);
        assert_eq!(detection.commands.build.len(), 1);
        assert_eq!(detection.commands.other.len(), 2);
        assert_eq!(detection.commands.test.len(), 0);
    }

    #[test]
    fn test_extract_rollup_commands() {
        let commands = get_rollup_commands();
        assert_eq!(commands.build.len(), 1);
        assert_eq!(commands.other.len(), 1);
        assert_eq!(commands.test.len(), 0);

        assert_eq!(commands.build[0].name, "build");
        assert_eq!(commands.build[0].command, "rollup -c");
        assert_eq!(
            commands.build[0].description,
            Some("Build bundle".to_string())
        );

        let watch_cmd = commands.other.iter().find(|c| c.name == "watch").unwrap();
        assert_eq!(watch_cmd.command, "rollup -c -w");
        assert_eq!(
            watch_cmd.description,
            Some("Watch and rebuild on changes".to_string())
        );
    }

    #[test]
    fn test_try_from_rollup_config() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(&dir, "rollup.config.js", "export default {}");
        let result = TaskRunnerFile::try_from(path);
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.task_runner, TaskRunner::Rollup);
        assert_eq!(file.source, TaskRunnerSource::RollupConfigJs);
    }

    #[test]
    fn test_rollup_file_to_detection() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(&dir, "rollup.config.js", "export default {}");
        let file = TaskRunnerFile::try_from(path).unwrap();
        let detection = TaskRunnerDetection::from(file);

        assert_eq!(detection.task_runner, TaskRunner::Rollup);
        assert_eq!(detection.commands.build.len(), 1);
        assert_eq!(detection.commands.other.len(), 1);
        assert_eq!(detection.commands.test.len(), 0);
    }

    #[test]
    fn test_try_from_rollup_config_variants() {
        let dir = TempDir::new().unwrap();

        let path = create_temp_file(&dir, "rollup.config.mjs", "export default {}");
        let result = TaskRunnerFile::try_from(path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().source, TaskRunnerSource::RollupConfigMjs);

        let path = create_temp_file(&dir, "rollup.config.cjs", "module.exports = {}");
        let result = TaskRunnerFile::try_from(path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().source, TaskRunnerSource::RollupConfigCjs);

        let path = create_temp_file(&dir, "rollup.config.ts", "export default {}");
        let result = TaskRunnerFile::try_from(path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().source, TaskRunnerSource::RollupConfigTs);
    }

    #[test]
    fn test_task_runner_file_to_detection() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(
            &dir,
            "Makefile",
            "test:\n\tcargo test\n\nbuild:\n\tcargo build",
        );
        let file = TaskRunnerFile::try_from(path).unwrap();
        let detection = TaskRunnerDetection::from(file);

        assert_eq!(detection.task_runner, TaskRunner::Make);
        assert!(!detection.commands.test.is_empty());
        assert!(!detection.commands.build.is_empty());
    }

    #[test]
    fn test_extract_turbo_commands_v1() {
        let content = r#"{
  "$schema": "https://turbo.build/schema.json",
  "pipeline": {
    "build": {
      "outputs": [".next/**"],
      "dependsOn": ["^build"]
    },
    "test": {
      "dependsOn": ["build"]
    },
    "dev": {
      "cache": false
    },
    "lint": {}
  }
}"#;
        let commands = extract_turbo_commands(content);
        assert_eq!(commands.test.len(), 1);
        assert_eq!(commands.build.len(), 1);
        assert_eq!(commands.other.len(), 2);

        assert_eq!(commands.test[0].name, "test");
        assert_eq!(commands.test[0].command, "turbo run test");

        assert_eq!(commands.build[0].name, "build");
        assert_eq!(commands.build[0].command, "turbo run build");

        let dev_cmd = commands.other.iter().find(|c| c.name == "dev").unwrap();
        assert_eq!(dev_cmd.command, "turbo run dev");

        let lint_cmd = commands.other.iter().find(|c| c.name == "lint").unwrap();
        assert_eq!(lint_cmd.command, "turbo run lint");
    }

    #[test]
    fn test_extract_turbo_commands_v2() {
        let content = r#"{
  "$schema": "https://turbo.build/schema.json",
  "tasks": {
    "build": {
      "outputs": [".next/**"]
    },
    "test": {},
    "dev": {
      "cache": false,
      "persistent": true
    }
  }
}"#;
        let commands = extract_turbo_commands(content);
        assert_eq!(commands.test.len(), 1);
        assert_eq!(commands.build.len(), 1);
        assert_eq!(commands.other.len(), 1);

        assert_eq!(commands.test[0].name, "test");
        assert_eq!(commands.build[0].name, "build");
        assert_eq!(commands.other[0].name, "dev");
    }

    #[test]
    fn test_extract_turbo_commands_invalid_json() {
        let content = "not valid json";
        let commands = extract_turbo_commands(content);
        assert!(commands.test.is_empty());
        assert!(commands.build.is_empty());
        assert!(commands.other.is_empty());
    }

    #[test]
    fn test_try_from_turbo_json() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(
            &dir,
            "turbo.json",
            r#"{"pipeline": {"build": {}, "test": {}}}"#,
        );
        let result = TaskRunnerFile::try_from(path);
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.task_runner, TaskRunner::Turbo);
        assert_eq!(file.source, TaskRunnerSource::TurboJson);
    }

    #[test]
    fn test_turbo_file_to_detection() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(
            &dir,
            "turbo.json",
            r#"{
  "pipeline": {
    "build": {},
    "test": {},
    "dev": {},
    "lint": {}
  }
}"#,
        );
        let file = TaskRunnerFile::try_from(path).unwrap();
        let detection = TaskRunnerDetection::from(file);

        assert_eq!(detection.task_runner, TaskRunner::Turbo);
        assert_eq!(detection.commands.build.len(), 1);
        assert_eq!(detection.commands.test.len(), 1);
        assert_eq!(detection.commands.other.len(), 2);
    }
}
