use anyhow::{Result, anyhow};
use escargot::CargoBuild;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env, fmt,
    fs::{self, DirBuilder},
    path::{Path, PathBuf},
};
use tempfile::TempDir;
use tokio::process::Command;

pub use commandeer_macros::commandeer;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommandInvocation {
    pub binary_name: String,
    pub args: Vec<String>,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RecordedCommands {
    commands: HashMap<String, Vec<CommandInvocation>>,
}

impl RecordedCommands {
    fn generate_key(binary_name: &str, args: &[String]) -> String {
        format!("{binary_name}:{}", args.join(" "))
    }

    pub fn add_invocation(&mut self, invocation: CommandInvocation) {
        let key = Self::generate_key(&invocation.binary_name, &invocation.args);

        self.commands.entry(key).or_default().push(invocation);
    }

    pub fn find_invocation(
        &self,
        binary_name: &str,
        args: &[String],
    ) -> Option<&CommandInvocation> {
        let key = Self::generate_key(binary_name, args);

        self.commands.get(&key)?.first()
    }
}

pub async fn load_recordings(file_path: &PathBuf) -> Result<RecordedCommands> {
    if !file_path.exists() {
        tokio::fs::create_dir_all(
            file_path.parent().ok_or_else(|| {
                anyhow!("Couldn't get parent of recording {}", file_path.display())
            })?,
        )
        .await?;

        return Ok(RecordedCommands::default());
    }

    let contents = tokio::fs::read_to_string(file_path).await?;

    if contents.trim().is_empty() {
        return Ok(RecordedCommands::default());
    }

    let recordings: RecordedCommands = serde_json::from_str(&contents)?;

    Ok(recordings)
}

pub async fn save_recordings(file_path: &PathBuf, recordings: &RecordedCommands) -> Result<()> {
    let json = serde_json::to_string_pretty(recordings)?;

    tokio::fs::write(file_path, json.as_bytes()).await?;

    Ok(())
}

pub async fn record_command(
    file_path: PathBuf,
    command: String,
    args: Vec<String>,
) -> Result<CommandInvocation> {
    let mut recordings = load_recordings(&file_path).await?;

    let output = Command::new(&command).args(&args).output().await?;

    let invocation = CommandInvocation {
        binary_name: command,
        args,
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    };

    recordings.add_invocation(invocation.clone());
    save_recordings(&file_path, &recordings).await?;

    Ok(invocation)
}

pub async fn replay_command(
    file_path: PathBuf,
    command: String,
    args: Vec<String>,
) -> Result<Option<CommandInvocation>> {
    let recordings = load_recordings(&file_path).await?;

    Ok(recordings.find_invocation(&command, &args).cloned())
}

pub fn output_invocation(invocation: &CommandInvocation) {
    print!("{}", invocation.stdout);
    eprint!("{}", invocation.stderr);
}

pub fn exit_with_code(code: i32) -> ! {
    std::process::exit(code);
}

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Record,
    Replay,
}

pub struct Commandeer {
    mock_runner: escargot::CargoRun,
    temp_dir: TempDir,
    fixture: PathBuf,
    mode: Mode,
    original_path: String,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mode::Record => write!(f, "record"),
            Mode::Replay => write!(f, "replay"),
        }
    }
}

impl Commandeer {
    pub fn new(test_name: impl AsRef<Path>, mode: Mode) -> Self {
        let dir = PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR").expect("Failed to get crate directory."),
        );

        DirBuilder::new()
            .recursive(true)
            .create(&dir)
            .expect("Failed to create testcmds dir");

        let fixture = dir.join("testcmds").join(test_name);

        let mock_runner = CargoBuild::new()
            .manifest_path(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .package("commandeer-cli")
            .bin("commandeer")
            .run()
            .expect("Failed to build mock binary");

        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let original_path = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{original_path}", temp_dir.path().display());

        unsafe {
            std::env::set_var("PATH", new_path);
        }

        Self {
            mock_runner,
            temp_dir,
            fixture,
            mode,
            original_path,
        }
    }
    pub fn mock_command(&self, command_name: &str) -> PathBuf {
        let mock_path = self.temp_dir.path().join(command_name);

        let wrapper = format!(
            r#"#!/usr/bin/env bash
exec env PATH="{}" {} {} --file {} --command {command_name} "$@"
"#,
            self.original_path,
            self.mock_runner.path().display(),
            self.mode,
            self.fixture.display(),
        );

        fs::write(&mock_path, wrapper).expect("Failed to write mock wrapper script");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;

            let mut perms = fs::metadata(&mock_path)
                .expect("Could not get permissions")
                .permissions();

            perms.set_mode(0o755);

            fs::set_permissions(&mock_path, perms).expect("Could not set permissions");
        }

        mock_path
    }
}

#[cfg(test)]
mod tests {
    use crate::{Commandeer, Mode, commandeer};

    #[serial_test::serial]
    fn test_mock_cmd() {
        let commandeer = Commandeer::new("test_recordings.json", Mode::Replay);
        let mock_path = commandeer.mock_command("echo");

        let status = std::process::Command::new("echo")
            .arg("foo")
            .status()
            .unwrap();

        assert!(status.success());

        assert!(mock_path.exists());
    }

    #[commandeer(Replay, "echo")]
    #[serial_test::serial]
    fn my_test() {
        let output = std::process::Command::new("echo")
            .arg("hello")
            .output()
            .unwrap();

        assert!(output.status.success());
    }

    #[commandeer(Replay, "date")]
    #[tokio::test]
    #[serial_test::serial]
    async fn async_replay() {
        let output = std::process::Command::new("date").output().unwrap();

        assert!(output.status.success());
    }

    #[commandeer(Record, "git")]
    #[test]
    #[serial_test::serial]
    fn test_flag_args() {
        let output = std::process::Command::new("git")
            .arg("--version")
            .output()
            .unwrap();

        assert!(output.status.success());
    }
}
