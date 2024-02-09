use std::io;
use std::path::PathBuf;
use std::process::Stdio;

use anyhow::bail;
use clap::ArgMatches;
use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use hsl::HSL;
use rand::Rng;
use rgb::{ComponentMap, RGB, RGB8, RGBA8};
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

pub(crate) fn progress_bar(progress: f32, num_leds: u32, only_show_cursor: bool) -> Vec<f32> {
    // We offset to make the bar start att all off and end at all on. Otherwise it will start at
    // one LED on.
    let offset = if !only_show_cursor { 1 } else { 0 };
    let progress_to_next_led = progress % (1. / (num_leds as f32)) * num_leds as f32;
    // The LED that is the furthest back in the "cursor"
    let led_at_back = (progress / (1. / num_leds as f32)).floor() as u32;
    let mut out = Vec::with_capacity(num_leds as usize);
    for i in offset..num_leds + offset {
        out.push(if i < led_at_back {
            if only_show_cursor {
                0.
            } else {
                1.
            }
        } else if i == led_at_back {
            if only_show_cursor {
                1. - progress_to_next_led
            } else {
                1.
            }
        } else if i == led_at_back + 1 {
            progress_to_next_led
        } else {
            0.
        })
    }
    out
}

/// Returns a vector with "orders". Each order is a tuple where the first element is the LED index
/// and the second is the brightness it should have as a float from 0.0 to 1.0
pub(crate) fn progress_bar_diff(
    progress: f32,
    last_progress: Option<f32>,
    num_leds: u32,
    only_show_cursor: bool,
) -> Vec<(u32, f32)> {
    let last_progress_bar =
        last_progress.map(|last_progress| progress_bar(last_progress, num_leds, only_show_cursor));
    let mut out = Vec::new();
    let progress_bar = progress_bar(progress, num_leds, only_show_cursor);
    if let Some(last_progress_bar) = last_progress_bar {
        for (i, &led_value) in progress_bar.iter().enumerate() {
            if last_progress_bar[i] != led_value {
                out.push((i as u32, led_value));
            }
        }
    } else {
        out = progress_bar
            .iter()
            .enumerate()
            .map(|(i, led_value)| (i as u32, *led_value))
            .collect();
    }
    out
}

pub(crate) fn prepare_terminal_event_capture() -> anyhow::Result<()> {
    let supports_keyboard_enhancement = matches!(
        crossterm::terminal::supports_keyboard_enhancement(),
        Ok(true)
    );
    let mut stdout = io::stdout();
    crossterm::terminal::enable_raw_mode()?;

    if supports_keyboard_enhancement {
        crossterm::queue!(
            stdout,
            PushKeyboardEnhancementFlags(
                // KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                // | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
                 | KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
            )
        )
        .unwrap();
    }
    crossterm::execute!(
        stdout,
        // EnableBracketedPaste,
        // EnableFocusChange,
        EnableMouseCapture,
    )
    .unwrap();
    Ok(())
}

pub(crate) fn default_terminal_settings() -> anyhow::Result<()> {
    let supports_keyboard_enhancement = matches!(
        crossterm::terminal::supports_keyboard_enhancement(),
        Ok(true)
    );
    let mut stdout = io::stdout();
    crossterm::terminal::disable_raw_mode()?;

    if supports_keyboard_enhancement {
        crossterm::queue!(stdout, PopKeyboardEnhancementFlags).unwrap();
    }
    crossterm::execute!(
        stdout,
        // EnableBracketedPaste,
        // EnableFocusChange,
        DisableMouseCapture,
    )
    .unwrap();
    Ok(())
}

pub(crate) fn get_config_path(args: &ArgMatches) -> anyhow::Result<PathBuf> {
    let keymap_path = match args.get_one::<PathBuf>("config") {
        None => dirs::config_dir().map(|pathbuf| pathbuf.join("keyboard-indicators/keymap.yaml")),
        Some(pathbuf) => Some(pathbuf.clone()),
    };

    let Some(config_path) = keymap_path else {
        bail!("Could not find a path for config. Please specify your own");
    };

    Ok(config_path)
}

/// Creates a color list with `len` items and evenly divided around the color circle
pub(crate) fn color_list(len: usize, saturation: f32, lightness: f32) -> Vec<openrgb::data::Color> {
    // let mut curr_hue = rand::thread_rng().gen_range(0.0..360.);
    let mut curr_hue = 0.;
    let hue_step = 360. / len as f64;
    let mut out = Vec::new();
    for _ in 0..len {
        let color = HSL {
            h: curr_hue,
            s: saturation as f64,
            l: lightness as f64,
        };
        let color_rgb = RGB::from(color.to_rgb());
        out.push(color_rgb);
        curr_hue += hue_step;
        if curr_hue >= 360. {
            curr_hue -= 360.
        }
    }
    out
}
