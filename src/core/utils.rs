use std::collections::HashMap;
use std::io::{self, BufRead, Read, Write};
use std::path::PathBuf;
use std::process::Stdio;

use anyhow::bail;
use anyhow::Result;
use clap::ArgMatches;
use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, Event, KeyboardEnhancementFlags, KeyCode,
    KeyModifiers, MouseButton, MouseEventKind, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use rgb::{ComponentMap, RGB, RGB8, RGBA8};
use tokio::process::{self, ChildStdout};
use tokio::sync::mpsc::Sender;

use super::config_manager::Configuration;
use super::keyboard_controller::{KeyboardController, KeyboardControllerMessage};
use super::module::Module;

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

pub(crate) fn prepare_terminal_event_capture() -> Result<()> {
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

pub(crate) fn default_terminal_settings() -> Result<()> {
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

pub(crate) fn get_keymap_path(args: &ArgMatches) -> Result<PathBuf> {
    let keymap_path = match args.get_one::<PathBuf>("keymap") {
        None => dirs::config_dir().map(|pathbuf| pathbuf.join("keyboard-indicators/keymap.yaml")),
        Some(pathbuf) => Some(pathbuf.clone()),
    };

    let Some(keymap_path) = keymap_path else {
        bail!("Could not find a path for keymap. Please specify your own");
    };

    Ok(keymap_path)
}

pub(crate) fn get_config_path(args: &ArgMatches) -> Result<PathBuf> {
    let keymap_path = match args.get_one::<PathBuf>("config") {
        None => dirs::config_dir().map(|pathbuf| pathbuf.join("keyboard-indicators/config.yaml")),
        Some(pathbuf) => Some(pathbuf.clone()),
    };

    let Some(config_path) = keymap_path else {
        bail!("Could not find a path for config. Please specify your own");
    };

    Ok(config_path)
}

/// Creates a color list with `len` items and evenly divided around the color circle
/// Saturation and lightness are supposed to be in the range 0..255
pub(crate) fn color_list(len: usize, saturation: f32, lightness: f32) -> Vec<openrgb::data::Color> {
    // let mut curr_hue = rand::thread_rng().gen_range(0.0..360.);
    let mut hue = 0.;
    let hue_step = 360. / len as f64;
    let mut out = Vec::new();
    for _ in 0..len {
        let color = hsv::hsv_to_rgb(hue, saturation as f64 / 100., lightness as f64 / 100.);
        let color_rgb = RGB::from(color);
        out.push(color_rgb);
        hue += hue_step;
        if hue >= 360. {
            hue -= 360.
        }
    }
    out
}

pub(crate) fn pause_until_click() -> Result<()> {
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    // We want the cursor to stay at the end of the line, so we print without a newline and flush manually.
    print!("Press any key to continue... ");
    stdout.flush()?;

    // Read a single byte and discard
    let _ = stdin.read(&mut [0u8]).unwrap();
    Ok(())
}

pub(crate) fn confirm_action(message: &str, default_value: bool) -> Result<bool> {
    print!("{}", message);
    io::stdout().flush()?;
    let stdin = io::stdin();
    let line = stdin
        .lock()
        .lines()
        .next()
        .unwrap_or(Ok(String::default()))?;
    if line.to_lowercase() == "y" {
        return Ok(true);
    }
    if line.to_lowercase() == "n" {
        return Ok(false);
    }

    Ok(default_value)
}

pub async fn highlight_all_modules(
    sender: &mut Sender<KeyboardControllerMessage>,
    config: &Configuration,
    saturation: f32,
    lightness: f32,
) -> Result<()> {
    KeyboardController::turn_all_off(sender).await?;
    let colors = color_list(config.modules.len(), saturation, lightness);
    for (i, module) in config.modules.iter().enumerate() {
        for led in &module.module_leds {
            let color = colors[i];
            if let Some(led) = led {
                KeyboardController::update_led(sender, *led, color).await?;
            }
        }
    }
    Ok(())
}

pub async fn highlight_one_module(
    sender: &mut Sender<KeyboardControllerMessage>,
    num_modules: usize,
    module_index: usize,
    module: &Module,
) -> Result<()> {
    let colors = color_list(num_modules, 100., 100.);
    for led in &module.module_leds {
        let color = colors[module_index];
        if let Some(led) = led {
            KeyboardController::update_led(sender, *led, color).await?;
        }
    }
    Ok(())
}

pub(crate) fn interpolate(from: RGB8, to: RGB8, progress: f32) -> RGB<u8> {
    let mut out = RGB8::new(0, 0, 0);
    out += from.map(|comp| (comp as f32 * (1. - progress)) as u8);
    out += to.map(|comp| (comp as f32 * progress) as u8);
    out
}

pub(crate) fn choose_option(options: &[impl AsRef<str>]) -> Result<usize> {
    for (i, option) in options.iter().enumerate() {
        println!("{}) {}", i + 1, option.as_ref());
    }

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        if let Ok(number_chosen) = line?.trim().parse::<usize>() {
            if number_chosen > 0 && number_chosen <= options.len() {
                return Ok(number_chosen - 1);
            }
        }
        println!(
            "Invalid option. Please choose a number between 1 and {}",
            options.len()
        );
    }

    Err(anyhow::anyhow!("No valid option chosen"))
}

pub(crate) fn rgb_to_hex(rgb: RGB8) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb.r, rgb.g, rgb.b)
}

pub(crate) fn get_color_input() -> Result<RGB<u8>> {
    println!("Write new color: ");
    get_input("Invalid RGB value. RGB values must be integers between 0 and 255, separated by commas or spaces, or a valid hexadecimal value (e.g., '#RRGGBB')", |input| parse_rgb(input).ok())
}

/// Takes in a closure which is applied on a line inputted by user. If the closure returns None, it
/// prints error_message and waits for more user input. This happens until the input from user
/// gives a Some value from closure, which is then returned
pub(crate) fn get_input<T, F>(error_message: &str, mut f: F) -> Result<T>
where
    F: FnMut(&str) -> Option<T>,
{
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let function_output = f(input.trim());

        if let Some(function_output) = function_output {
            return Ok(function_output);
        }
        println!("{}", error_message);
    }
}

pub(crate) fn parse_rgb(input: &str) -> Result<RGB8> {
    fn split_decimal_input(input: &str) -> Option<(&str, &str, &str)> {
        let parts: Vec<&str> = input
            .split(|c| c == ',' || c == ' ')
            .filter(|&part| !part.is_empty())
            .collect();
        match (parts.get(0), parts.get(1), parts.get(2), parts.get(3)) {
            (Some(&r), Some(&g), Some(&b), None) => Some((r, g, b)),
            _ => None,
        }
    }
    let input = input.trim();

    if input.len() == 6 || input.starts_with('#') && input.len() == 7 {
        // Remove the #
        let input = if input.len() == 7 { &input[1..] } else { input };
        // Handle hexadecimal format
        let r = u8::from_str_radix(&input[0..2], 16)?;
        let g = u8::from_str_radix(&input[2..4], 16)?;
        let b = u8::from_str_radix(&input[4..6], 16)?;

        Ok((r, g, b).into())
    } else if let Some((r_str, g_str, b_str)) = split_decimal_input(input) {
        // Assume the input is in the format "r g b", "r,g,b" or "r, g, b"
        let r = r_str.parse()?;
        let g = g_str.parse()?;
        let b = b_str.parse()?;

        Ok((r, g, b).into())
    } else {
        Err(anyhow::anyhow!("Invalid RGB value. RGB values must be integers between 0 and 255, separated by commas or spaces, or a valid hexadecimal value (e.g., '#RRGGBB')"))
    }
}

/// Highlights a module with a rainbow palette to make the order of the LEDs clear
pub async fn highlight_one_module_rainbow(
    sender: &mut Sender<KeyboardControllerMessage>,
    module: &Module,
) -> Result<()> {
    let num_leds = module.module_leds.len();
    let colors = color_list(num_leds, 100., 100.);
    for (i, led) in module.module_leds.iter().enumerate() {
        let color = colors[i];
        if let Some(led) = led {
            KeyboardController::update_led(sender, *led, color).await?;
        }
    }
    Ok(())
}

/// This light curve keeps the points (0, 0) and (255, 255) constant and curves the rest according to the value of k
pub fn compute_light_curve(k: f64, x: u8) -> u8 {
    let x = x as f64;
    let common_term = 255. + (65025. + 4. * k).sqrt();
    let denominator_1 = 2. * x - common_term;
    let denominator_2 = common_term;

    let result = 2. * k * (-1. / denominator_1 - 1. / denominator_2);

    result.round() as u8
}

pub fn calculate_k_value(led_brightness: u8) -> f64 {
    todo!()
}

pub fn compute_light_curve_for_color(k: f64, color: RGB8) -> RGB8 {
    color.map(|comp| compute_light_curve(k, comp))
}

pub async fn pick_leds(
    sender: &mut Sender<KeyboardControllerMessage>,
    key_led_map: &HashMap<KeyCode, u32>,
    color: RGB8,
) -> Result<Vec<Option<u32>>> {
    prepare_terminal_event_capture()?;
    let mut leds_picked = Vec::new();
    loop {
        let event = crossterm::event::read().unwrap();
        if let Event::Key(event) = event {
            if event.kind != crossterm::event::KeyEventKind::Press {
                continue;
            }
            if event.modifiers.intersects(KeyModifiers::CONTROL) && event.code == KeyCode::Char('c')
            {
                default_terminal_settings()?;
                panic!("Interrupted by user");
            }
            default_terminal_settings()?;
            if let Some(&index_pressed) = key_led_map.get(&event.code) {
                if leds_picked.contains(&Some(index_pressed)) {
                    println!("This LED has already been added");
                    prepare_terminal_event_capture()?;
                    continue;
                }
                leds_picked.push(Some(index_pressed));
                KeyboardController::update_led(sender, index_pressed, color)
                    .await?;
            } else {
                println!("This button is not tied to an LED, it can't be used in module");
            }
            prepare_terminal_event_capture()?;
        }
        if let Event::Mouse(event) = event {
            if event.kind == MouseEventKind::Down(MouseButton::Left) {
                default_terminal_settings()?;
                return Ok(leds_picked);
            }
            if event.kind == MouseEventKind::Down(MouseButton::Right) {
                leds_picked.push(None);
            }
        }
    }
}

pub(crate) async fn pick_led(key_led_map: &HashMap<KeyCode, u32>) -> Result<u32> {
    prepare_terminal_event_capture()?;
    loop {
        let event = crossterm::event::read().unwrap();
        if let Event::Key(event) = event {
            if event.kind != crossterm::event::KeyEventKind::Press {
                continue;
            }
            if event.modifiers.intersects(KeyModifiers::CONTROL) && event.code == KeyCode::Char('c')
            {
                default_terminal_settings()?;
                panic!("Interrupted by user");
            }
            default_terminal_settings()?;
            if let Some(&index_pressed) = key_led_map.get(&event.code) {
                break Ok(index_pressed);
            } else {
                println!("This button is not tied to an LED, it can't be used in module");
            }
            prepare_terminal_event_capture()?;
        }
    }
}

mod tests {
    #![allow(unused_imports)]

    use rgb::RGB;

    use crate::core::utils::{color_list, compute_light_curve};

    #[test]
    fn test_color_list() {
        for len in 1..=10 {
            let list = color_list(len, 100., 100.);
            assert_eq!(list.len(), len);
            assert_eq!(*list.first().unwrap(), RGB::from((255, 0, 0)));
        }
    }

    #[test]
    fn test_light_curve() {
        for k in (1..=1000000).step_by(100) {
            assert_eq!(compute_light_curve(k as f64, 255), 255)
        }
    }
}
