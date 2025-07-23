use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

/// Supported IDE types with their detection logic
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum IdeType {
    VSCode,
    IntellijIdea,
    AndroidStudio,
    WebStorm,
    PyCharm,
    Vim,
    Emacs,
    SublimeText,
    Atom,
    Cursor,
}

impl IdeType {
    /// Get possible commands to launch this IDE (in order of preference)
    pub fn commands(&self) -> &'static [&'static str] {
        match self {
            IdeType::VSCode => &["code", "code-insiders"],
            IdeType::IntellijIdea => &["idea", "intellij-idea-ultimate", "intellij-idea-community"],
            IdeType::AndroidStudio => &["studio", "android-studio"],
            IdeType::WebStorm => &["webstorm"],
            IdeType::PyCharm => &["pycharm", "pycharm-professional", "pycharm-community"],
            IdeType::Vim => &["nvim", "vim"],
            IdeType::Emacs => &["emacs"],
            IdeType::SublimeText => &["subl", "sublime_text"],
            IdeType::Atom => &["atom"],
            IdeType::Cursor => &["cursor"],
        }
    }

    /// Get display name for this IDE
    pub fn display_name(&self) -> &'static str {
        match self {
            IdeType::VSCode => "Visual Studio Code",
            IdeType::IntellijIdea => "IntelliJ IDEA",
            IdeType::AndroidStudio => "Android Studio",
            IdeType::WebStorm => "WebStorm",
            IdeType::PyCharm => "PyCharm",
            IdeType::Vim => "Vim/Neovim",
            IdeType::Emacs => "Emacs",
            IdeType::SublimeText => "Sublime Text",
            IdeType::Atom => "Atom",
            IdeType::Cursor => "Cursor",
        }
    }

    /// Get all IDE types
    pub fn all() -> &'static [IdeType] {
        &[
            IdeType::VSCode,
            IdeType::Cursor,
            IdeType::IntellijIdea,
            IdeType::AndroidStudio,
            IdeType::WebStorm,
            IdeType::PyCharm,
            IdeType::SublimeText,
            IdeType::Vim,
            IdeType::Emacs,
            IdeType::Atom,
        ]
    }
}

/// Global cache for available IDEs
static AVAILABLE_IDES: OnceLock<HashMap<IdeType, String>> = OnceLock::new();

/// Initialize and cache available IDEs on the system
fn get_available_ides() -> &'static HashMap<IdeType, String> {
    AVAILABLE_IDES.get_or_init(|| {
        let mut available = HashMap::new();

        for ide_type in IdeType::all() {
            for &command in ide_type.commands() {
                if is_command_available(command) {
                    available.insert(ide_type.clone(), command.to_string());
                    break; // Use first available command
                }
            }
        }

        available
    })
}

/// Check if a command is available in the system PATH
fn is_command_available(command: &str) -> bool {
    Command::new("which")
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Get available IDEs for a project, filtered by what's installed
pub fn get_available_ides_for_project<P: AsRef<Path>>(project_path: P) -> Vec<(IdeType, String)> {
    let available_ides = get_available_ides();
    let suggested = detect_ide_type(&project_path);
    let mut result = Vec::new();

    // Add suggested IDEs first (if available)
    for ide_type in suggested {
        if let Some(command) = available_ides.get(&ide_type) {
            result.push((ide_type, command.clone()));
        }
    }

    // Add other available IDEs
    for (ide_type, command) in available_ides {
        if !result
            .iter()
            .any(|(existing_type, _)| existing_type == ide_type)
        {
            result.push((ide_type.clone(), command.clone()));
        }
    }

    result
}

/// Detect the most appropriate IDE for a project path
pub fn detect_ide_type<P: AsRef<Path>>(project_path: P) -> Vec<IdeType> {
    let path = project_path.as_ref();
    let mut ides = Vec::new();

    // Android Studio detection (highest priority for Android projects)
    if is_android_project(path) {
        ides.push(IdeType::AndroidStudio);
    }

    // IntelliJ IDEA detection (but not if it's clearly an Android project)
    if is_intellij_project(path) && !is_android_project(path) {
        ides.push(IdeType::IntellijIdea);
    }

    // WebStorm detection
    if is_web_project(path) {
        ides.push(IdeType::WebStorm);
    }

    // PyCharm detection
    if is_python_project(path) {
        ides.push(IdeType::PyCharm);
    }

    // VSCode and Cursor are good general-purpose editors
    ides.push(IdeType::VSCode);
    ides.push(IdeType::Cursor);

    ides
}

/// Launch IDE for the given project path with specific command
pub fn launch_ide_with_command<P: AsRef<Path>>(project_path: P, command: &str) -> Result<()> {
    let path = project_path.as_ref();
    let path_str = path.to_string_lossy();

    // Use nohup and redirect to /dev/null to fully detach from terminal
    let mut cmd = Command::new("nohup");
    cmd.arg(command)
        .arg(&*path_str)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null());

    match cmd.spawn() {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(
            "Failed to launch {} with command '{}': {}",
            path_str,
            command,
            e
        )),
    }
}

// Project type detection functions

fn is_android_project<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();

    // Check for Android-specific files/directories (more specific patterns)
    path.join("android").exists()
        || path.join("app/build.gradle").exists()
        || (path.join("settings.gradle").exists() && path.join("app").exists())
        || (path.join("build.gradle").exists()
            && (path.join("app/src/main/AndroidManifest.xml").exists()
                || path.join("app/src/main/java").exists()
                || path.join("app/src/main/kotlin").exists()))
}

fn is_intellij_project<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();

    // Check for IntelliJ IDEA project markers
    path.join(".idea").exists()
        || path.join("pom.xml").exists()
        || path.join("build.gradle").exists()
        || path.join("build.gradle.kts").exists()
}

fn is_web_project<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();

    // Check for web project markers
    path.join("package.json").exists()
        || path.join("yarn.lock").exists()
        || path.join("webpack.config.js").exists()
        || path.join("next.config.js").exists()
        || path.join("vue.config.js").exists()
        || path.join("angular.json").exists()
}

fn is_python_project<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();

    // Check for Python project markers
    path.join("requirements.txt").exists()
        || path.join("pyproject.toml").exists()
        || path.join("setup.py").exists()
        || path.join("Pipfile").exists()
        || path.join("poetry.lock").exists()
        || path.join("manage.py").exists() // Django project
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_detect_android_project() {
        // Use system temp directory instead of tempfile crate
        let path = std::env::temp_dir().join("test_android_project");

        // Create test directory and android subdirectory
        fs::create_dir_all(&path).unwrap();
        fs::create_dir_all(path.join("android")).unwrap();

        assert!(is_android_project(&path));

        let ides = detect_ide_type(&path);
        assert!(ides.contains(&IdeType::AndroidStudio));

        // Cleanup
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn test_detect_web_project() {
        let path = std::env::temp_dir().join("test_web_project");

        // Create test directory and package.json
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("package.json"), "{}").unwrap();

        assert!(is_web_project(&path));

        let ides = detect_ide_type(&path);
        assert!(ides.contains(&IdeType::WebStorm));

        // Cleanup
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn test_vscode_always_available() {
        let path = std::env::temp_dir().join("test_generic_project");

        let ides = detect_ide_type(&path);
        assert!(ides.contains(&IdeType::VSCode));
    }
}
