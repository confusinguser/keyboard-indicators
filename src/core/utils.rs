use std::process::Stdio;

use rgb::{ComponentMap, RGB8, RGBA8};
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

pub(crate) fn overlay(color1: RGBA8, color2: RGB8) -> RGB8 {
    (color1
        .rgb()
        .map(|comp| comp as f32 * color1.a as f32 / 255.)
        + color2.map(|comp| comp as f32 * (1. - color1.a as f32 / 255.)))
    .map(|comp| comp as u8)
}
