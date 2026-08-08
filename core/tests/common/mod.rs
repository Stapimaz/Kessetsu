#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_WORKSPACE_ID: AtomicU64 = AtomicU64::new(0);

pub fn fixture_path(relative: impl AsRef<Path>) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(relative)
}

pub fn read_fixture(relative: impl AsRef<Path>) -> String {
    let path = fixture_path(relative);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("could not read fixture '{}': {error}", path.display()))
}

pub fn normalize_text(input: &str) -> String {
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    format!("{}\n", normalized.trim_end())
}

pub struct TestWorkspace {
    root: PathBuf,
}

impl TestWorkspace {
    pub fn new(label: &str) -> Self {
        let safe_label: String = label
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character
                } else {
                    '-'
                }
            })
            .collect();
        let sequence = NEXT_WORKSPACE_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "netlang-test-{safe_label}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&root)
            .unwrap_or_else(|error| panic!("could not create '{}': {error}", root.display()));
        Self { root }
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    pub fn write(&self, relative: impl AsRef<Path>, contents: &str) -> PathBuf {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|error| panic!("could not create '{}': {error}", parent.display()));
        }
        fs::write(&path, contents)
            .unwrap_or_else(|error| panic!("could not write '{}': {error}", path.display()));
        path
    }

    pub fn run_cli(&self, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_netlang"))
            .args(arguments)
            .current_dir(&self.root)
            .output()
            .unwrap_or_else(|error| panic!("could not run NetLang CLI: {error}"))
    }

    pub fn run_cli_with_env(&self, arguments: &[&str], key: &str, value: &Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_netlang"))
            .args(arguments)
            .env(key, value)
            .current_dir(&self.root)
            .output()
            .unwrap_or_else(|error| panic!("could not run NetLang CLI: {error}"))
    }

    pub fn write_fake_simulator(
        &self,
        name: &str,
        stdout: &str,
        stderr: &str,
        code: i32,
    ) -> PathBuf {
        #[cfg(windows)]
        let (file_name, contents) = (
            format!("{name}.cmd"),
            format!(
                "@echo off\r\n{}{}exit /b {code}\r\n",
                stdout
                    .lines()
                    .map(|line| format!("echo {line}\r\n"))
                    .collect::<String>(),
                stderr
                    .lines()
                    .map(|line| format!("echo {line} 1>&2\r\n"))
                    .collect::<String>()
            ),
        );

        #[cfg(not(windows))]
        let (file_name, contents) = (
            format!("{name}.sh"),
            format!(
                "#!/bin/sh\n{}{}exit {code}\n",
                stdout
                    .lines()
                    .map(|line| format!("printf '%s\\n' '{line}'\n"))
                    .collect::<String>(),
                stderr
                    .lines()
                    .map(|line| format!("printf '%s\\n' '{line}' >&2\n"))
                    .collect::<String>()
            ),
        );

        let path = self.write(file_name, &contents);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&path)
                .expect("fake simulator metadata should be readable")
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions).expect("fake simulator should be executable");
        }
        path
    }

    pub fn normalize_cli_text(&self, bytes: &[u8]) -> String {
        let text = String::from_utf8_lossy(bytes);
        let root = self.root.to_string_lossy();
        let json_escaped_root = root.replace('\\', "\\\\");
        let normalized = text
            .replace(&json_escaped_root, "<TEMP>")
            .replace(root.as_ref(), "<TEMP>")
            .replace(&root.replace('\\', "/"), "<TEMP>")
            .replace('\\', "/")
            .replace("<TEMP>//", "<TEMP>/");
        normalize_text(&normalized)
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let temp_root = std::env::temp_dir();
        let is_owned_workspace = self.root.parent() == Some(temp_root.as_path())
            && self
                .root
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("netlang-test-"));

        if is_owned_workspace && self.root.exists() {
            fs::remove_dir_all(&self.root).unwrap_or_else(|error| {
                panic!(
                    "could not remove test workspace '{}': {error}",
                    self.root.display()
                )
            });
        }
    }
}
