use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDefinition {
    id: String,
    label: String,
    command: String,
    cwd: String,
    source: String,
    group: String,
    problem_matcher: Option<String>,
}

fn group_for_name(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if matches!(lower.as_str(), "build" | "compile" | "bundle") {
        "build"
    } else if matches!(lower.as_str(), "test" | "check" | "lint") || lower.contains("test") {
        "test"
    } else if matches!(lower.as_str(), "dev" | "start" | "serve" | "run") {
        "run"
    } else {
        "custom"
    }
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | ':' | '='))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn push_task(
    out: &mut Vec<TaskDefinition>,
    root: &Path,
    source: &str,
    label: impl Into<String>,
    command: impl Into<String>,
    group: impl Into<String>,
    problem_matcher: Option<String>,
) {
    let label = label.into();
    let source_id = source.to_lowercase().replace(' ', "-");
    let task_id = label
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>();
    out.push(TaskDefinition {
        id: format!("{source_id}:{task_id}"),
        label,
        command: command.into(),
        cwd: root.to_string_lossy().to_string(),
        source: source.to_string(),
        group: group.into(),
        problem_matcher,
    });
}

fn add_package_tasks(root: &Path, out: &mut Vec<TaskDefinition>) {
    let path = root.join("package.json");
    let Ok(raw) = std::fs::read_to_string(path) else {
        return;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return;
    };
    let Some(scripts) = json.get("scripts").and_then(|v| v.as_object()) else {
        return;
    };
    let mut names: Vec<&String> = scripts.keys().collect();
    names.sort();
    for name in names {
        let group = group_for_name(name);
        let matcher = match group {
            "build" | "test" => Some("$tsc".to_string()),
            _ => None,
        };
        push_task(
            out,
            root,
            "npm",
            format!("npm: {name}"),
            format!("npm run {}", shell_quote(name)),
            group,
            matcher,
        );
    }
}

fn add_cargo_tasks(root: &Path, out: &mut Vec<TaskDefinition>) {
    if !root.join("Cargo.toml").is_file() {
        return;
    }
    for (label, command, group) in [
        ("cargo: check", "cargo check", "test"),
        ("cargo: build", "cargo build", "build"),
        ("cargo: test", "cargo test", "test"),
        ("cargo: run", "cargo run", "run"),
    ] {
        push_task(out, root, "cargo", label, command, group, Some("$rustc".into()));
    }
}

fn add_make_tasks(root: &Path, out: &mut Vec<TaskDefinition>) {
    if root.join("Makefile").is_file() || root.join("makefile").is_file() {
        push_task(out, root, "make", "make", "make", "build", None);
    }
}

fn json_array_strings(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn add_configured_tasks(root: &Path, rel: &str, source: &str, out: &mut Vec<TaskDefinition>) {
    let path = root.join(rel);
    let Ok(raw) = std::fs::read_to_string(path) else {
        return;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return;
    };
    let Some(tasks) = json.get("tasks").and_then(|v| v.as_array()) else {
        return;
    };
    for task in tasks {
        let label = task
            .get("label")
            .and_then(|v| v.as_str())
            .or_else(|| task.get("taskName").and_then(|v| v.as_str()));
        let command = task.get("command").and_then(|v| v.as_str());
        let (Some(label), Some(command)) = (label, command) else {
            continue;
        };
        let args = json_array_strings(task.get("args").unwrap_or(&serde_json::Value::Null));
        let mut full_command = command.to_string();
        for arg in args {
            full_command.push(' ');
            full_command.push_str(&shell_quote(&arg));
        }
        let group = task
            .get("group")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| group_for_name(label));
        let matcher = task
            .get("problemMatcher")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        push_task(out, root, source, label, full_command, group, matcher);
    }
}

fn discover_tasks(root: &Path) -> Vec<TaskDefinition> {
    let mut out = Vec::new();
    add_configured_tasks(root, ".michael/tasks.json", "Michael", &mut out);
    add_configured_tasks(root, ".vscode/tasks.json", "VS Code", &mut out);
    add_package_tasks(root, &mut out);
    add_cargo_tasks(root, &mut out);
    add_make_tasks(root, &mut out);
    out.sort_by(|a, b| a.source.cmp(&b.source).then(a.label.cmp(&b.label)));
    out
}

#[tauri::command]
pub fn tasks_list(root: String) -> Result<Vec<TaskDefinition>, String> {
    let root = PathBuf::from(root);
    if !root.is_dir() {
        return Err("workspace root is not a directory".into());
    }
    Ok(discover_tasks(&root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("michael-ide-{name}-{suffix}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn discovers_package_scripts() {
        let root = temp_root("npm");
        std::fs::write(
            root.join("package.json"),
            r#"{"scripts":{"dev":"vite","build":"vite build","test":"vitest"}}"#,
        )
        .unwrap();

        let tasks = discover_tasks(&root);
        let labels: Vec<&str> = tasks.iter().map(|task| task.label.as_str()).collect();
        assert!(labels.contains(&"npm: build"));
        assert!(labels.contains(&"npm: dev"));
        assert!(labels.contains(&"npm: test"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn discovers_cargo_tasks() {
        let root = temp_root("cargo");
        std::fs::write(root.join("Cargo.toml"), "[package]\nname='demo'\nversion='0.1.0'\n").unwrap();

        let tasks = discover_tasks(&root);
        assert!(tasks.iter().any(|task| task.command == "cargo check"));
        assert!(tasks.iter().any(|task| task.command == "cargo test"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn discovers_configured_tasks_with_args() {
        let root = temp_root("configured");
        std::fs::create_dir_all(root.join(".michael")).unwrap();
        std::fs::write(
            root.join(".michael/tasks.json"),
            r#"{"tasks":[{"label":"Type Check","command":"npm","args":["run","typecheck"],"group":"test","problemMatcher":"$tsc"}]}"#,
        )
        .unwrap();

        let tasks = discover_tasks(&root);
        let task = tasks.iter().find(|task| task.label == "Type Check").unwrap();
        assert_eq!(task.command, "npm run typecheck");
        assert_eq!(task.problem_matcher.as_deref(), Some("$tsc"));
        let _ = std::fs::remove_dir_all(root);
    }
}
