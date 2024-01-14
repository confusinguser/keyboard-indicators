use std::process::Stdio;

use tokio::process::{self, ChildStdout};

pub(crate) fn run_command_async(command: &str) -> Option<ChildStdout> {
    let mut command_array = command.split(' ');
    if command_array.clone().count() == 0 {
        return None;
    }
    println!("Running command: {}", command);

    let mut cmd = process::Command::new(command_array.next().unwrap())
        .stdout(Stdio::piped())
        .args(command_array)
        .spawn()
        .unwrap();
    cmd.stdout.take()
}

pub(crate) fn run_command(command: &str) -> Option<String> {
    let mut command_array = command.split(' ');
    if command_array.clone().count() == 0 {
        return Default::default();
    }

    let cmd = std::process::Command::new(command_array.next().unwrap())
        .stdout(Stdio::piped())
        .args(command_array)
        .output()
        .unwrap();

    String::from_utf8(cmd.stdout)
        .ok()
        .map(|e| e.trim().to_owned())
}
