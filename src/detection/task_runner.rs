use regex::Regex;
use serde::Serialize;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandCategory {
    Test,
    Build,
    Other,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum CommandExecutable {
    Direct {
        command: String,
    },
    PackageManagerScript {
        script_name: String,
        script_body: String,
        package_json_path: PathBuf,
    },
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

    // Rust
    Cargo,

    // Go
    GoTask,
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

    // Rust
    CargoToml,

    // Go
    GoMod,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskCommand {
    pub name: String,
    pub executable: CommandExecutable,
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
            TaskRunnerSource::CargoToml => TaskRunner::Cargo,
            TaskRunnerSource::GoMod => TaskRunner::GoTask,
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

            // Rust
            "Cargo.toml" => TaskRunnerSource::CargoToml,

            // Go
            "go.mod" => TaskRunnerSource::GoMod,

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
            TaskRunnerSource::PackageJson => extract_npm_commands(content, &self.path),
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
            TaskRunnerSource::NxJson => extract_nx_commands(content),
            TaskRunnerSource::ToxIni => extract_tox_commands(content),
            TaskRunnerSource::NoxPy | TaskRunnerSource::Noxfile => extract_nox_commands(content),
            TaskRunnerSource::TasksPy => extract_invoke_commands(content),
            TaskRunnerSource::InvokeYaml => extract_invoke_yaml_commands(content),
            TaskRunnerSource::CargoToml => get_cargo_commands(),
            TaskRunnerSource::GoMod => get_go_commands(),
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
                executable: CommandExecutable::Direct {
                    command: format!("make {}", target),
                },
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
                executable: CommandExecutable::Direct {
                    command: format!("just {}", recipe),
                },
                description: None,
            };

            commands.add_command(cmd, classify_command(recipe));
        }
    }

    commands
}

fn extract_npm_commands(content: &str, package_json_path: &Path) -> TaskRunnerCommands {
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
            executable: CommandExecutable::PackageManagerScript {
                script_name: name.clone(),
                script_body: command_str.to_string(),
                package_json_path: package_json_path.to_path_buf(),
            },
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
                    executable: CommandExecutable::Direct {
                        command: format!("task {}", name),
                    },
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
        executable: CommandExecutable::Direct {
            command: "vite".to_string(),
        },
        description: Some("Start dev server".to_string()),
    };
    commands.add_command(dev_cmd, CommandCategory::Other);

    let build_cmd = TaskCommand {
        name: "build".to_string(),
        executable: CommandExecutable::Direct {
            command: "vite build".to_string(),
        },
        description: Some("Build for production".to_string()),
    };
    commands.add_command(build_cmd, CommandCategory::Build);

    let preview_cmd = TaskCommand {
        name: "preview".to_string(),
        executable: CommandExecutable::Direct {
            command: "vite preview".to_string(),
        },
        description: Some("Preview production build".to_string()),
    };
    commands.add_command(preview_cmd, CommandCategory::Other);

    commands
}

fn get_webpack_commands() -> TaskRunnerCommands {
    let mut commands = TaskRunnerCommands::default();

    let build_cmd = TaskCommand {
        name: "build".to_string(),
        executable: CommandExecutable::Direct {
            command: "webpack build".to_string(),
        },
        description: Some("Build for production".to_string()),
    };
    commands.add_command(build_cmd, CommandCategory::Build);

    let serve_cmd = TaskCommand {
        name: "serve".to_string(),
        executable: CommandExecutable::Direct {
            command: "webpack serve".to_string(),
        },
        description: Some("Start dev server".to_string()),
    };
    commands.add_command(serve_cmd, CommandCategory::Other);

    let watch_cmd = TaskCommand {
        name: "watch".to_string(),
        executable: CommandExecutable::Direct {
            command: "webpack watch".to_string(),
        },
        description: Some("Watch for file changes".to_string()),
    };
    commands.add_command(watch_cmd, CommandCategory::Other);

    commands
}

fn get_rspack_commands() -> TaskRunnerCommands {
    let mut commands = TaskRunnerCommands::default();

    let dev_cmd = TaskCommand {
        name: "dev".to_string(),
        executable: CommandExecutable::Direct {
            command: "rspack dev".to_string(),
        },
        description: Some("Start dev server".to_string()),
    };
    commands.add_command(dev_cmd, CommandCategory::Other);

    let build_cmd = TaskCommand {
        name: "build".to_string(),
        executable: CommandExecutable::Direct {
            command: "rspack build".to_string(),
        },
        description: Some("Build for production".to_string()),
    };
    commands.add_command(build_cmd, CommandCategory::Build);

    let preview_cmd = TaskCommand {
        name: "preview".to_string(),
        executable: CommandExecutable::Direct {
            command: "rspack preview".to_string(),
        },
        description: Some("Preview production build".to_string()),
    };
    commands.add_command(preview_cmd, CommandCategory::Other);

    commands
}

fn get_rollup_commands() -> TaskRunnerCommands {
    let mut commands = TaskRunnerCommands::default();

    let build_cmd = TaskCommand {
        name: "build".to_string(),
        executable: CommandExecutable::Direct {
            command: "rollup -c".to_string(),
        },
        description: Some("Build bundle".to_string()),
    };
    commands.add_command(build_cmd, CommandCategory::Build);

    let watch_cmd = TaskCommand {
        name: "watch".to_string(),
        executable: CommandExecutable::Direct {
            command: "rollup -c -w".to_string(),
        },
        description: Some("Watch and rebuild on changes".to_string()),
    };
    commands.add_command(watch_cmd, CommandCategory::Other);

    commands
}

fn get_cargo_commands() -> TaskRunnerCommands {
    let mut commands = TaskRunnerCommands::default();

    let test_cmd = TaskCommand {
        name: "test".to_string(),
        executable: CommandExecutable::Direct {
            command: "cargo test".to_string(),
        },
        description: Some("Run tests".to_string()),
    };
    commands.add_command(test_cmd, CommandCategory::Test);

    let build_cmd = TaskCommand {
        name: "build".to_string(),
        executable: CommandExecutable::Direct {
            command: "cargo build".to_string(),
        },
        description: Some("Build project".to_string()),
    };
    commands.add_command(build_cmd, CommandCategory::Build);

    let build_release_cmd = TaskCommand {
        name: "build-release".to_string(),
        executable: CommandExecutable::Direct {
            command: "cargo build --release".to_string(),
        },
        description: Some("Build optimized release".to_string()),
    };
    commands.add_command(build_release_cmd, CommandCategory::Build);

    let check_cmd = TaskCommand {
        name: "check".to_string(),
        executable: CommandExecutable::Direct {
            command: "cargo check".to_string(),
        },
        description: Some("Check without building".to_string()),
    };
    commands.add_command(check_cmd, CommandCategory::Other);

    let clippy_cmd = TaskCommand {
        name: "clippy".to_string(),
        executable: CommandExecutable::Direct {
            command: "cargo clippy".to_string(),
        },
        description: Some("Run linter".to_string()),
    };
    commands.add_command(clippy_cmd, CommandCategory::Other);

    let fmt_cmd = TaskCommand {
        name: "fmt".to_string(),
        executable: CommandExecutable::Direct {
            command: "cargo fmt".to_string(),
        },
        description: Some("Format code".to_string()),
    };
    commands.add_command(fmt_cmd, CommandCategory::Other);

    commands
}

fn get_go_commands() -> TaskRunnerCommands {
    let mut commands = TaskRunnerCommands::default();

    let test_cmd = TaskCommand {
        name: "test".to_string(),
        executable: CommandExecutable::Direct {
            command: "go test ./...".to_string(),
        },
        description: Some("Run tests".to_string()),
    };
    commands.add_command(test_cmd, CommandCategory::Test);

    let build_cmd = TaskCommand {
        name: "build".to_string(),
        executable: CommandExecutable::Direct {
            command: "go build".to_string(),
        },
        description: Some("Build project".to_string()),
    };
    commands.add_command(build_cmd, CommandCategory::Build);

    let run_cmd = TaskCommand {
        name: "run".to_string(),
        executable: CommandExecutable::Direct {
            command: "go run .".to_string(),
        },
        description: Some("Run project".to_string()),
    };
    commands.add_command(run_cmd, CommandCategory::Other);

    let fmt_cmd = TaskCommand {
        name: "fmt".to_string(),
        executable: CommandExecutable::Direct {
            command: "go fmt ./...".to_string(),
        },
        description: Some("Format code".to_string()),
    };
    commands.add_command(fmt_cmd, CommandCategory::Other);

    let vet_cmd = TaskCommand {
        name: "vet".to_string(),
        executable: CommandExecutable::Direct {
            command: "go vet ./...".to_string(),
        },
        description: Some("Examine code for issues".to_string()),
    };
    commands.add_command(vet_cmd, CommandCategory::Other);

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
            executable: CommandExecutable::Direct {
                command: format!("turbo run {}", task_name),
            },
            description: None,
        };

        commands.add_command(cmd, classify_command(task_name));
    }

    commands
}

fn extract_nx_commands(content: &str) -> TaskRunnerCommands {
    let mut commands = TaskRunnerCommands::default();

    let Ok(json) = serde_json::from_str::<JsonValue>(content) else {
        return commands;
    };

    let target_defaults = json.get("targetDefaults").and_then(|t| t.as_object());

    if let Some(targets) = target_defaults {
        for (target_name, _target_config) in targets {
            let cmd = TaskCommand {
                name: target_name.clone(),
                executable: CommandExecutable::Direct {
                    command: format!("nx run {}", target_name),
                },
                description: None,
            };

            commands.add_command(cmd, classify_command(target_name));
        }
    }

    if commands.test.is_empty()
        && commands.build.is_empty()
        && commands.other.is_empty()
        && let Some(targets) = json.get("targets").and_then(|t| t.as_object())
    {
        for (target_name, _target_config) in targets {
            let cmd = TaskCommand {
                name: target_name.clone(),
                executable: CommandExecutable::Direct {
                    command: format!("nx run {}", target_name),
                },
                description: None,
            };

            commands.add_command(cmd, classify_command(target_name));
        }
    }

    commands
}

fn extract_tox_commands(content: &str) -> TaskRunnerCommands {
    let mut commands = TaskRunnerCommands::default();
    let env_re = Regex::new(r"^\[testenv(?::([a-zA-Z0-9_-]+))?\]").unwrap();
    let lines: Vec<&str> = content.lines().collect();

    for (idx, line) in lines.iter().enumerate() {
        if let Some(caps) = env_re.captures(line.trim()) {
            let env_name = caps.get(1).map(|m| m.as_str()).unwrap_or("default");

            let commands_found: Vec<String> = lines[idx + 1..]
                .iter()
                .enumerate()
                .find(|(_, l)| {
                    let t = l.trim();
                    !t.starts_with('[')
                        && !t.is_empty()
                        && !t.starts_with('#')
                        && t.starts_with("commands")
                })
                .map(|(offset, cmd_line)| {
                    let mut cmds = Vec::new();

                    if let Some(val) = cmd_line
                        .split_once('=')
                        .map(|(_, v)| v.trim())
                        .filter(|v| !v.is_empty())
                    {
                        cmds.push(val.to_string());
                    }

                    cmds.extend(
                        lines[idx + 1 + offset + 1..]
                            .iter()
                            .take_while(|l| {
                                let t = l.trim();
                                (l.starts_with(' ') || l.starts_with('\t')) && !t.starts_with('[')
                            })
                            .filter_map(|l| {
                                let t = l.trim();
                                (!t.is_empty() && !t.starts_with('#')).then(|| t.to_string())
                            }),
                    );

                    cmds
                })
                .unwrap_or_default();

            if !commands_found.is_empty() {
                let cmd = TaskCommand {
                    name: env_name.to_string(),
                    executable: CommandExecutable::Direct {
                        command: if env_name == "default" {
                            "tox".to_string()
                        } else {
                            format!("tox -e {}", env_name)
                        },
                    },
                    description: None,
                };

                commands.add_command(
                    cmd,
                    if env_name == "default" || env_name.starts_with("py") {
                        CommandCategory::Test
                    } else {
                        classify_command(env_name)
                    },
                );
            }
        }
    }

    if commands.test.is_empty() && commands.build.is_empty() && commands.other.is_empty() {
        commands.add_command(
            TaskCommand {
                name: "default".to_string(),
                executable: CommandExecutable::Direct {
                    command: "tox".to_string(),
                },
                description: Some("Run all tox environments".to_string()),
            },
            CommandCategory::Test,
        );
    }

    commands
}

fn extract_nox_commands(content: &str) -> TaskRunnerCommands {
    let mut commands = TaskRunnerCommands::default();
    let func_re = Regex::new(r"^def\s+([a-zA-Z0-9_]+)\s*\(").unwrap();
    let lines: Vec<&str> = content.lines().collect();

    for (idx, line) in lines.iter().enumerate() {
        if line.trim().starts_with("@nox.session") {
            let decorator_text = lines[idx..]
                .iter()
                .map(|l| l.trim())
                .take_while(|t| !t.starts_with("def "))
                .scan(false, |done, t| {
                    if *done {
                        return None;
                    }
                    *done = t.contains(')');
                    Some(t)
                })
                .collect::<Vec<_>>()
                .join(" ");

            let explicit_name = decorator_text.split_once("name=").and_then(|(_, after)| {
                let after = after.trim_start();
                let quote = after.chars().next()?;
                (quote == '"' || quote == '\'').then(|| {
                    after
                        .get(1..)?
                        .find(quote)
                        .map(|end| after[1..1 + end].to_string())
                })?
            });

            let func_name = lines[idx..]
                .iter()
                .map(|l| l.trim())
                .filter(|t| {
                    !t.starts_with('@')
                        && !t.is_empty()
                        && !t.starts_with('#')
                        && (!t.starts_with(')') || t.contains("def "))
                })
                .find_map(|t| func_re.captures(t)?.get(1).map(|m| m.as_str().to_string()));

            if let Some(func_name) = func_name {
                let session_name = explicit_name.as_ref().unwrap_or(&func_name);

                let category = if session_name.starts_with("py") {
                    // Nox-specific: py38, py39, etc. are test sessions >:(
                    CommandCategory::Test
                } else {
                    classify_command(session_name)
                };

                commands.add_command(
                    TaskCommand {
                        name: session_name.to_string(),
                        executable: CommandExecutable::Direct {
                            command: format!("nox -s {}", session_name),
                        },
                        description: None,
                    },
                    category,
                );
            }
        }
    }

    if commands.test.is_empty() && commands.build.is_empty() && commands.other.is_empty() {
        commands.add_command(
            TaskCommand {
                name: "default".to_string(),
                executable: CommandExecutable::Direct {
                    command: "nox".to_string(),
                },
                description: Some("Run all nox sessions".to_string()),
            },
            CommandCategory::Test,
        );
    }

    commands
}

fn extract_invoke_commands(content: &str) -> TaskRunnerCommands {
    let mut commands = TaskRunnerCommands::default();
    let func_re = Regex::new(r"^def\s+([a-zA-Z0-9_]+)\s*\(").unwrap();
    let task_decorator_re = Regex::new(r"^@task\b").unwrap();
    let lines: Vec<&str> = content.lines().collect();

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if task_decorator_re.is_match(trimmed) {
            let func_name = lines[idx..]
                .iter()
                .map(|l| l.trim())
                .filter(|t| {
                    !t.starts_with('@')
                        && !t.is_empty()
                        && !t.starts_with('#')
                        && (!t.starts_with(')') || t.contains("def "))
                })
                .find_map(|t| func_re.captures(t)?.get(1).map(|m| m.as_str().to_string()));

            if let Some(task_name) = func_name {
                commands.add_command(
                    TaskCommand {
                        name: task_name.clone(),
                        executable: CommandExecutable::Direct {
                            command: format!("invoke {}", task_name),
                        },
                        description: None,
                    },
                    classify_command(&task_name),
                );
            }
        }
    }

    commands
}

fn extract_invoke_yaml_commands(content: &str) -> TaskRunnerCommands {
    let mut commands = TaskRunnerCommands::default();

    let Ok(yaml) = serde_yaml::from_str::<YamlValue>(content) else {
        return commands;
    };

    if let Some(tasks) = yaml.get("tasks").and_then(|t| t.as_mapping()) {
        for (task_name, _task_data) in tasks {
            if let Some(name) = task_name.as_str() {
                commands.add_command(
                    TaskCommand {
                        name: name.to_string(),
                        executable: CommandExecutable::Direct {
                            command: format!("invoke {}", name),
                        },
                        description: None,
                    },
                    classify_command(name),
                );
            }
        }
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
        assert!(matches!(
            &commands.test[0].executable,
            CommandExecutable::Direct { command } if command == "make test"
        ));
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
        let package_json_path = PathBuf::from("/test/package.json");
        let commands = extract_npm_commands(content, &package_json_path);
        assert_eq!(commands.test.len(), 1);
        assert_eq!(commands.build.len(), 1);
        assert_eq!(commands.other.len(), 3);

        assert_eq!(commands.test[0].name, "test");
        assert!(matches!(
            &commands.test[0].executable,
            CommandExecutable::PackageManagerScript {
                script_name,
                script_body,
                package_json_path: _
            } if script_name == "test" && script_body == "jest"
        ));
        assert_eq!(commands.test[0].description, None);

        assert_eq!(commands.build[0].name, "build");
        assert!(matches!(
            &commands.build[0].executable,
            CommandExecutable::PackageManagerScript {
                script_name,
                script_body,
                package_json_path: _
            } if script_name == "build" && script_body == "vite build"
        ));
        assert_eq!(commands.build[0].description, None);
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
        assert!(matches!(
            &commands.build[0].executable,
            CommandExecutable::Direct { command } if command == "vite build"
        ));
        assert_eq!(
            commands.build[0].description,
            Some("Build for production".to_string())
        );

        let dev_cmd = commands.other.iter().find(|c| c.name == "dev").unwrap();
        assert!(matches!(
            &dev_cmd.executable,
            CommandExecutable::Direct { command } if command == "vite"
        ));
        assert_eq!(dev_cmd.description, Some("Start dev server".to_string()));

        let preview_cmd = commands.other.iter().find(|c| c.name == "preview").unwrap();
        assert!(matches!(
            &preview_cmd.executable,
            CommandExecutable::Direct { command } if command == "vite preview"
        ));
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
        assert!(matches!(
            &commands.build[0].executable,
            CommandExecutable::Direct { command } if command == "webpack build"
        ));
        assert_eq!(
            commands.build[0].description,
            Some("Build for production".to_string())
        );

        let serve_cmd = commands.other.iter().find(|c| c.name == "serve").unwrap();
        assert!(matches!(
            &serve_cmd.executable,
            CommandExecutable::Direct { command } if command == "webpack serve"
        ));
        assert_eq!(serve_cmd.description, Some("Start dev server".to_string()));

        let watch_cmd = commands.other.iter().find(|c| c.name == "watch").unwrap();
        assert!(matches!(
            &watch_cmd.executable,
            CommandExecutable::Direct { command } if command == "webpack watch"
        ));
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
        assert!(matches!(
            &commands.build[0].executable,
            CommandExecutable::Direct { command } if command == "rspack build"
        ));
        assert_eq!(
            commands.build[0].description,
            Some("Build for production".to_string())
        );

        let dev_cmd = commands.other.iter().find(|c| c.name == "dev").unwrap();
        assert!(matches!(
            &dev_cmd.executable,
            CommandExecutable::Direct { command } if command == "rspack dev"
        ));
        assert_eq!(dev_cmd.description, Some("Start dev server".to_string()));

        let preview_cmd = commands.other.iter().find(|c| c.name == "preview").unwrap();
        assert!(matches!(
            &preview_cmd.executable,
            CommandExecutable::Direct { command } if command == "rspack preview"
        ));
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
        assert!(matches!(
            &commands.build[0].executable,
            CommandExecutable::Direct { command } if command == "rollup -c"
        ));
        assert_eq!(
            commands.build[0].description,
            Some("Build bundle".to_string())
        );

        let watch_cmd = commands.other.iter().find(|c| c.name == "watch").unwrap();
        assert!(matches!(
            &watch_cmd.executable,
            CommandExecutable::Direct { command } if command == "rollup -c -w"
        ));
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
        assert!(matches!(
            &commands.test[0].executable,
            CommandExecutable::Direct { command } if command == "turbo run test"
        ));

        assert_eq!(commands.build[0].name, "build");
        assert!(matches!(
            &commands.build[0].executable,
            CommandExecutable::Direct { command } if command == "turbo run build"
        ));

        let dev_cmd = commands.other.iter().find(|c| c.name == "dev").unwrap();
        assert!(matches!(
            &dev_cmd.executable,
            CommandExecutable::Direct { command } if command == "turbo run dev"
        ));

        let lint_cmd = commands.other.iter().find(|c| c.name == "lint").unwrap();
        assert!(matches!(
            &lint_cmd.executable,
            CommandExecutable::Direct { command } if command == "turbo run lint"
        ));
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

    #[test]
    fn test_extract_nx_commands_target_defaults() {
        let content = r#"{
  "$schema": "./node_modules/nx/schemas/nx-schema.json",
  "targetDefaults": {
    "build": {
      "cache": true,
      "dependsOn": ["^build"]
    },
    "test": {
      "cache": true
    },
    "lint": {
      "cache": true
    },
    "e2e": {
      "cache": true
    }
  }
}"#;
        let commands = extract_nx_commands(content);
        assert_eq!(commands.test.len(), 1);
        assert_eq!(commands.build.len(), 1);
        assert_eq!(commands.other.len(), 2);

        assert_eq!(commands.test[0].name, "test");
        assert!(matches!(
            &commands.test[0].executable,
            CommandExecutable::Direct { command } if command == "nx run test"
        ));

        assert_eq!(commands.build[0].name, "build");
        assert!(matches!(
            &commands.build[0].executable,
            CommandExecutable::Direct { command } if command == "nx run build"
        ));

        assert!(commands.other.iter().any(|c| c.name == "lint"));
        assert!(commands.other.iter().any(|c| c.name == "e2e"));
    }

    #[test]
    fn test_extract_nx_commands_targets() {
        let content = r#"{
  "$schema": "./node_modules/nx/schemas/nx-schema.json",
  "targets": {
    "build": {
      "executor": "@nx/webpack:webpack",
      "options": {}
    },
    "test": {
      "executor": "@nx/jest:jest"
    },
    "serve": {
      "executor": "@nx/webpack:dev-server"
    }
  }
}"#;
        let commands = extract_nx_commands(content);
        assert_eq!(commands.test.len(), 1);
        assert_eq!(commands.build.len(), 1);
        assert_eq!(commands.other.len(), 1);

        assert_eq!(commands.test[0].name, "test");
        assert!(matches!(
            &commands.test[0].executable,
            CommandExecutable::Direct { command } if command == "nx run test"
        ));

        assert_eq!(commands.build[0].name, "build");
        assert!(matches!(
            &commands.build[0].executable,
            CommandExecutable::Direct { command } if command == "nx run build"
        ));

        let serve_cmd = commands.other.iter().find(|c| c.name == "serve").unwrap();
        assert!(matches!(
            &serve_cmd.executable,
            CommandExecutable::Direct { command } if command == "nx run serve"
        ));
    }

    #[test]
    fn test_extract_nx_commands_invalid_json() {
        let content = "not valid json";
        let commands = extract_nx_commands(content);
        assert!(commands.test.is_empty());
        assert!(commands.build.is_empty());
        assert!(commands.other.is_empty());
    }

    #[test]
    fn test_extract_nx_commands_empty() {
        let content = r#"{
  "$schema": "./node_modules/nx/schemas/nx-schema.json"
}"#;
        let commands = extract_nx_commands(content);
        assert!(commands.test.is_empty());
        assert!(commands.build.is_empty());
        assert!(commands.other.is_empty());
    }

    #[test]
    fn test_try_from_nx_json() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(
            &dir,
            "nx.json",
            r#"{"targetDefaults": {"build": {}, "test": {}}}"#,
        );
        let result = TaskRunnerFile::try_from(path);
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.task_runner, TaskRunner::Nx);
        assert_eq!(file.source, TaskRunnerSource::NxJson);
    }

    #[test]
    fn test_nx_file_to_detection() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(
            &dir,
            "nx.json",
            r#"{
  "targetDefaults": {
    "build": {
      "cache": true
    },
    "test": {
      "cache": true
    },
    "lint": {},
    "serve": {}
  }
}"#,
        );
        let file = TaskRunnerFile::try_from(path).unwrap();
        let detection = TaskRunnerDetection::from(file);

        assert_eq!(detection.task_runner, TaskRunner::Nx);
        assert_eq!(detection.commands.build.len(), 1);
        assert_eq!(detection.commands.test.len(), 1);
        assert_eq!(detection.commands.other.len(), 2);
    }

    #[test]
    fn test_extract_tox_commands_basic() {
        let content = r#"
[testenv]
commands = pytest tests/
"#;
        let commands = extract_tox_commands(content);
        assert_eq!(commands.test.len(), 1);
        assert_eq!(commands.test[0].name, "default");
        assert!(matches!(
            &commands.test[0].executable,
            CommandExecutable::Direct { command } if command == "tox"
        ));
        assert_eq!(commands.test[0].description, None);
    }

    #[test]
    fn test_extract_tox_commands_multiple_envs() {
        let content = r#"
[testenv:py39]
commands = pytest tests/

[testenv:py310]
commands = pytest --cov tests/

[testenv:lint]
commands =
    flake8 src/
    mypy src/
"#;
        let commands = extract_tox_commands(content);
        assert_eq!(commands.test.len(), 2);
        assert_eq!(commands.other.len(), 1);

        let py39_cmd = commands.test.iter().find(|c| c.name == "py39").unwrap();
        assert!(matches!(
            &py39_cmd.executable,
            CommandExecutable::Direct { command } if command == "tox -e py39"
        ));
        assert_eq!(py39_cmd.description, None);

        let py310_cmd = commands.test.iter().find(|c| c.name == "py310").unwrap();
        assert!(matches!(
            &py310_cmd.executable,
            CommandExecutable::Direct { command } if command == "tox -e py310"
        ));
        assert_eq!(py310_cmd.description, None);

        let lint_cmd = commands.other.iter().find(|c| c.name == "lint").unwrap();
        assert!(matches!(
            &lint_cmd.executable,
            CommandExecutable::Direct { command } if command == "tox -e lint"
        ));
        assert_eq!(lint_cmd.description, None);
    }

    #[test]
    fn test_extract_tox_commands_multiline() {
        let content = r#"
[testenv]
commands =
    pytest --verbose
    coverage report
    mypy src/
"#;
        let commands = extract_tox_commands(content);
        assert_eq!(commands.test.len(), 1);
        assert_eq!(commands.test[0].name, "default");
        assert_eq!(commands.test[0].description, None);
    }

    #[test]
    fn test_extract_tox_commands_empty() {
        let content = r#"
[tox]
envlist = py39,py310

[testenv]
deps = pytest
"#;
        let commands = extract_tox_commands(content);
        assert_eq!(commands.test.len(), 1);
        assert_eq!(commands.test[0].name, "default");
        assert!(matches!(
            &commands.test[0].executable,
            CommandExecutable::Direct { command } if command == "tox"
        ));
    }

    #[test]
    fn test_extract_tox_commands_with_comments() {
        let content = r#"
# This is a comment
[testenv:py39]
# Another comment
commands = pytest tests/
    # Inline comment
    coverage report
"#;
        let commands = extract_tox_commands(content);
        assert_eq!(commands.test.len(), 1);
        assert_eq!(commands.test[0].name, "py39");
        assert_eq!(commands.test[0].description, None);
    }

    #[test]
    fn test_extract_tox_commands_build_env() {
        let content = r#"
[testenv:build]
commands = python -m build

[testenv:package]
commands =
    python setup.py sdist
    python setup.py bdist_wheel
"#;
        let commands = extract_tox_commands(content);
        assert_eq!(commands.build.len(), 2);

        let build_cmd = commands.build.iter().find(|c| c.name == "build").unwrap();
        assert!(matches!(
            &build_cmd.executable,
            CommandExecutable::Direct { command } if command == "tox -e build"
        ));
        assert_eq!(build_cmd.description, None);

        let package_cmd = commands.build.iter().find(|c| c.name == "package").unwrap();
        assert!(matches!(
            &package_cmd.executable,
            CommandExecutable::Direct { command } if command == "tox -e package"
        ));
        assert_eq!(package_cmd.description, None);
    }

    #[test]
    fn test_try_from_tox_ini() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(
            &dir,
            "tox.ini",
            r#"
[testenv]
commands = pytest tests/
"#,
        );
        let result = TaskRunnerFile::try_from(path);
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.task_runner, TaskRunner::Tox);
        assert_eq!(file.source, TaskRunnerSource::ToxIni);
    }

    #[test]
    fn test_tox_file_to_detection() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(
            &dir,
            "tox.ini",
            r#"
[testenv:py39]
commands = pytest tests/

[testenv:py310]
commands = pytest --cov tests/

[testenv:lint]
commands = flake8 src/
"#,
        );
        let file = TaskRunnerFile::try_from(path).unwrap();
        let detection = TaskRunnerDetection::from(file);

        assert_eq!(detection.task_runner, TaskRunner::Tox);
        assert_eq!(detection.commands.test.len(), 2);
        assert_eq!(detection.commands.other.len(), 1);

        assert!(detection.commands.test.iter().any(|c| c.name == "py39"
            && matches!(
                &c.executable,
                CommandExecutable::Direct { command } if command == "tox -e py39"
            )));
        assert!(detection.commands.test.iter().any(|c| c.name == "py310"
            && matches!(
                &c.executable,
                CommandExecutable::Direct { command } if command == "tox -e py310"
            )));
        assert!(detection.commands.other.iter().any(|c| c.name == "lint"
            && matches!(
                &c.executable,
                CommandExecutable::Direct { command } if command == "tox -e lint"
            )));
    }

    #[test]
    fn test_extract_nox_commands_basic() {
        let content = r#"
import nox

@nox.session
def test(session):
    session.install("pytest")
    session.run("pytest", "tests/")
"#;
        let commands = extract_nox_commands(content);
        assert_eq!(commands.test.len(), 1);
        assert_eq!(commands.test[0].name, "test");
        assert!(matches!(
            &commands.test[0].executable,
            CommandExecutable::Direct { command } if command == "nox -s test"
        ));
    }

    #[test]
    fn test_extract_nox_commands_multiple_sessions() {
        let content = r#"
import nox

@nox.session
def test(session):
    session.run("pytest")

@nox.session
def lint(session):
    session.run("flake8")

@nox.session
def build(session):
    session.run("python", "-m", "build")
"#;
        let commands = extract_nox_commands(content);
        assert_eq!(commands.test.len(), 1);
        assert_eq!(commands.build.len(), 1);
        assert_eq!(commands.other.len(), 1);

        assert_eq!(commands.test[0].name, "test");
        assert!(matches!(
            &commands.test[0].executable,
            CommandExecutable::Direct { command } if command == "nox -s test"
        ));

        let build_cmd = commands.build.iter().find(|c| c.name == "build").unwrap();
        assert!(matches!(
            &build_cmd.executable,
            CommandExecutable::Direct { command } if command == "nox -s build"
        ));

        let lint_cmd = commands.other.iter().find(|c| c.name == "lint").unwrap();
        assert!(matches!(
            &lint_cmd.executable,
            CommandExecutable::Direct { command } if command == "nox -s lint"
        ));
    }

    #[test]
    fn test_extract_nox_commands_with_explicit_names() {
        let content = r#"
import nox

@nox.session(name="unit-tests")
def unit_tests(session):
    session.run("pytest", "tests/unit/")

@nox.session(name="integration-tests")
def integration_tests(session):
    session.run("pytest", "tests/integration/")
"#;
        let commands = extract_nox_commands(content);
        assert_eq!(commands.test.len(), 2);

        assert!(commands.test.iter().any(|c| c.name == "unit-tests"
            && matches!(
                &c.executable,
                CommandExecutable::Direct { command } if command == "nox -s unit-tests"
            )));
        assert!(commands.test.iter().any(|c| c.name == "integration-tests"
            && matches!(
                &c.executable,
                CommandExecutable::Direct { command } if command == "nox -s integration-tests"
            )));
    }

    #[test]
    fn test_extract_nox_commands_python_versions() {
        let content = r#"
import nox

@nox.session
def py39(session):
    session.run("pytest")

@nox.session
def py310(session):
    session.run("pytest")

@nox.session
def py311(session):
    session.run("pytest")
"#;
        let commands = extract_nox_commands(content);
        assert_eq!(commands.test.len(), 3);

        assert!(commands.test.iter().any(|c| c.name == "py39"
            && matches!(
                &c.executable,
                CommandExecutable::Direct { command } if command == "nox -s py39"
            )));
        assert!(commands.test.iter().any(|c| c.name == "py310"
            && matches!(
                &c.executable,
                CommandExecutable::Direct { command } if command == "nox -s py310"
            )));
        assert!(commands.test.iter().any(|c| c.name == "py311"
            && matches!(
                &c.executable,
                CommandExecutable::Direct { command } if command == "nox -s py311"
            )));
    }

    #[test]
    fn test_extract_nox_commands_with_comments() {
        let content = r#"
import nox

# This is a test session
@nox.session
def test(session):
    """Run the test suite."""
    session.run("pytest")

# Linting session
@nox.session
def lint(session):
    session.run("flake8")
"#;
        let commands = extract_nox_commands(content);
        assert_eq!(commands.test.len(), 1);
        assert_eq!(commands.other.len(), 1);
    }

    #[test]
    fn test_extract_nox_commands_empty() {
        let content = r#"
import nox

# No sessions defined yet
"#;
        let commands = extract_nox_commands(content);
        assert_eq!(commands.test.len(), 1);
        assert_eq!(commands.test[0].name, "default");
        assert!(matches!(
            &commands.test[0].executable,
            CommandExecutable::Direct { command } if command == "nox"
        ));
    }

    #[test]
    fn test_extract_nox_commands_with_parameters() {
        let content = r#"
import nox

@nox.session(python=["3.9", "3.10", "3.11"])
def tests(session):
    session.run("pytest")

@nox.session(python="3.10", name="coverage")
def run_coverage(session):
    session.run("pytest", "--cov")
"#;
        let commands = extract_nox_commands(content);
        assert_eq!(commands.test.len(), 1);
        assert_eq!(commands.other.len(), 1);

        assert!(commands.test.iter().any(|c| c.name == "tests"
            && matches!(
                &c.executable,
                CommandExecutable::Direct { command } if command == "nox -s tests"
            )));
        assert!(commands.other.iter().any(|c| c.name == "coverage"
            && matches!(
                &c.executable,
                CommandExecutable::Direct { command } if command == "nox -s coverage"
            )));
    }

    #[test]
    fn test_extract_nox_commands_complex_decorator() {
        let content = r#"
import nox

@nox.session(
    python=["3.9", "3.10", "3.11"],
    name="test-all",
    reuse_venv=True
)
def test_all_pythons(session):
    session.run("pytest")
"#;
        let commands = extract_nox_commands(content);
        assert_eq!(commands.test.len(), 1);
        assert_eq!(commands.test[0].name, "test-all");
        assert!(matches!(
            &commands.test[0].executable,
            CommandExecutable::Direct { command } if command == "nox -s test-all"
        ));
    }

    #[test]
    fn test_try_from_nox_py() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(
            &dir,
            "nox.py",
            r#"
import nox

@nox.session
def test(session):
    session.run("pytest")
"#,
        );
        let result = TaskRunnerFile::try_from(path);
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.task_runner, TaskRunner::Nox);
        assert_eq!(file.source, TaskRunnerSource::NoxPy);
    }

    #[test]
    fn test_try_from_noxfile() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(
            &dir,
            "noxfile.py",
            r#"
import nox

@nox.session
def test(session):
    session.run("pytest")
"#,
        );
        let result = TaskRunnerFile::try_from(path);
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.task_runner, TaskRunner::Nox);
        assert_eq!(file.source, TaskRunnerSource::Noxfile);
    }

    #[test]
    fn test_nox_file_to_detection() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(
            &dir,
            "noxfile.py",
            r#"
import nox

@nox.session
def test(session):
    session.run("pytest")

@nox.session
def lint(session):
    session.run("flake8")

@nox.session
def build(session):
    session.run("python", "-m", "build")
"#,
        );
        let file = TaskRunnerFile::try_from(path).unwrap();
        let detection = TaskRunnerDetection::from(file);

        assert_eq!(detection.task_runner, TaskRunner::Nox);
        assert_eq!(detection.commands.test.len(), 1);
        assert_eq!(detection.commands.build.len(), 1);
        assert_eq!(detection.commands.other.len(), 1);

        assert!(detection.commands.test.iter().any(|c| c.name == "test"
            && matches!(
                &c.executable,
                CommandExecutable::Direct { command } if command == "nox -s test"
            )));
        assert!(detection.commands.build.iter().any(|c| c.name == "build"
            && matches!(
                &c.executable,
                CommandExecutable::Direct { command } if command == "nox -s build"
            )));
        assert!(detection.commands.other.iter().any(|c| c.name == "lint"
            && matches!(
                &c.executable,
                CommandExecutable::Direct { command } if command == "nox -s lint"
            )));
    }

    #[test]
    fn test_extract_invoke_commands_basic() {
        let content = r#"
from invoke import task

@task
def test(c):
    c.run("pytest tests/")
"#;
        let commands = extract_invoke_commands(content);
        assert_eq!(commands.test.len(), 1);
        assert_eq!(commands.test[0].name, "test");
        assert!(matches!(
            &commands.test[0].executable,
            CommandExecutable::Direct { command } if command == "invoke test"
        ));
    }

    #[test]
    fn test_extract_invoke_commands_multiple_tasks() {
        let content = r#"
from invoke import task

@task
def test(c):
    c.run("pytest")

@task
def lint(c):
    c.run("flake8 .")

@task
def build(c):
    c.run("python -m build")

@task
def clean(c):
    c.run("rm -rf dist/")
"#;
        let commands = extract_invoke_commands(content);
        assert_eq!(commands.test.len(), 1);
        assert_eq!(commands.build.len(), 1);
        assert_eq!(commands.other.len(), 2);

        assert_eq!(commands.test[0].name, "test");
        assert!(matches!(
            &commands.test[0].executable,
            CommandExecutable::Direct { command } if command == "invoke test"
        ));

        let build_cmd = commands.build.iter().find(|c| c.name == "build").unwrap();
        assert!(matches!(
            &build_cmd.executable,
            CommandExecutable::Direct { command } if command == "invoke build"
        ));

        let lint_cmd = commands.other.iter().find(|c| c.name == "lint").unwrap();
        assert!(matches!(
            &lint_cmd.executable,
            CommandExecutable::Direct { command } if command == "invoke lint"
        ));

        let clean_cmd = commands.other.iter().find(|c| c.name == "clean").unwrap();
        assert!(matches!(
            &clean_cmd.executable,
            CommandExecutable::Direct { command } if command == "invoke clean"
        ));
    }

    #[test]
    fn test_extract_invoke_commands_with_parameters() {
        let content = r#"
from invoke import task

@task(pre=[clean])
def build(c):
    c.run("python setup.py build")

@task
def clean(c):
    c.run("rm -rf build/")
"#;
        let commands = extract_invoke_commands(content);
        assert_eq!(commands.build.len(), 1);
        assert_eq!(commands.other.len(), 1);

        let build_cmd = commands.build.iter().find(|c| c.name == "build").unwrap();
        assert!(matches!(
            &build_cmd.executable,
            CommandExecutable::Direct { command } if command == "invoke build"
        ));
    }

    #[test]
    fn test_extract_invoke_commands_empty() {
        let content = r#"
from invoke import task

# No tasks defined yet
"#;
        let commands = extract_invoke_commands(content);
        assert_eq!(commands.test.len(), 0);
        assert_eq!(commands.build.len(), 0);
        assert_eq!(commands.other.len(), 0);
    }

    #[test]
    fn test_extract_invoke_commands_with_docstrings() {
        let content = r#"
from invoke import task

@task
def test(c):
    """Run the test suite."""
    c.run("pytest")

@task
def coverage(c):
    """Run tests with coverage."""
    c.run("pytest --cov")
"#;
        let commands = extract_invoke_commands(content);
        assert_eq!(commands.test.len(), 1);
        assert_eq!(commands.other.len(), 1);

        let test_cmd = commands.test.iter().find(|c| c.name == "test").unwrap();
        assert!(matches!(
            &test_cmd.executable,
            CommandExecutable::Direct { command } if command == "invoke test"
        ));

        let coverage_cmd = commands
            .other
            .iter()
            .find(|c| c.name == "coverage")
            .unwrap();
        assert!(matches!(
            &coverage_cmd.executable,
            CommandExecutable::Direct { command } if command == "invoke coverage"
        ));
    }

    #[test]
    fn test_extract_invoke_yaml_commands() {
        let content = r#"
tasks:
  test:
    command: pytest tests/
  build:
    command: python -m build
  lint:
    command: flake8 .
"#;
        let commands = extract_invoke_yaml_commands(content);
        assert_eq!(commands.test.len(), 1);
        assert_eq!(commands.build.len(), 1);
        assert_eq!(commands.other.len(), 1);

        assert_eq!(commands.test[0].name, "test");
        assert!(matches!(
            &commands.test[0].executable,
            CommandExecutable::Direct { command } if command == "invoke test"
        ));

        let build_cmd = commands.build.iter().find(|c| c.name == "build").unwrap();
        assert!(matches!(
            &build_cmd.executable,
            CommandExecutable::Direct { command } if command == "invoke build"
        ));

        let lint_cmd = commands.other.iter().find(|c| c.name == "lint").unwrap();
        assert!(matches!(
            &lint_cmd.executable,
            CommandExecutable::Direct { command } if command == "invoke lint"
        ));
    }

    #[test]
    fn test_extract_invoke_yaml_commands_empty() {
        let content = r#"
# No tasks defined
other_config: value
"#;
        let commands = extract_invoke_yaml_commands(content);
        assert!(commands.test.is_empty());
        assert!(commands.build.is_empty());
        assert!(commands.other.is_empty());
    }

    #[test]
    fn test_extract_invoke_yaml_commands_invalid() {
        let content = "not valid yaml: [";
        let commands = extract_invoke_yaml_commands(content);
        assert!(commands.test.is_empty());
        assert!(commands.build.is_empty());
        assert!(commands.other.is_empty());
    }

    #[test]
    fn test_try_from_tasks_py() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(
            &dir,
            "tasks.py",
            r#"
from invoke import task

@task
def test(c):
    c.run("pytest")
"#,
        );
        let result = TaskRunnerFile::try_from(path);
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.task_runner, TaskRunner::Invoke);
        assert_eq!(file.source, TaskRunnerSource::TasksPy);
    }

    #[test]
    fn test_try_from_invoke_yaml() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(
            &dir,
            "invoke.yaml",
            r#"
tasks:
  test:
    command: pytest
"#,
        );
        let result = TaskRunnerFile::try_from(path);
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.task_runner, TaskRunner::Invoke);
        assert_eq!(file.source, TaskRunnerSource::InvokeYaml);
    }

    #[test]
    fn test_invoke_file_to_detection() {
        let dir = TempDir::new().unwrap();
        let path = create_temp_file(
            &dir,
            "tasks.py",
            r#"
from invoke import task

@task
def test(c):
    c.run("pytest")

@task
def lint(c):
    c.run("flake8 .")

@task
def build(c):
    c.run("python -m build")
"#,
        );
        let file = TaskRunnerFile::try_from(path).unwrap();
        let detection = TaskRunnerDetection::from(file);

        assert_eq!(detection.task_runner, TaskRunner::Invoke);
        assert_eq!(detection.commands.test.len(), 1);
        assert_eq!(detection.commands.build.len(), 1);
        assert_eq!(detection.commands.other.len(), 1);

        assert!(detection.commands.test.iter().any(|c| c.name == "test"
            && matches!(
                &c.executable,
                CommandExecutable::Direct { command } if command == "invoke test"
            )));
        assert!(detection.commands.build.iter().any(|c| c.name == "build"
            && matches!(
                &c.executable,
                CommandExecutable::Direct { command } if command == "invoke build"
            )));
        assert!(detection.commands.other.iter().any(|c| c.name == "lint"
            && matches!(
                &c.executable,
                CommandExecutable::Direct { command } if command == "invoke lint"
            )));
    }
}
