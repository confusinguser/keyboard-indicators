use std::io;

use crossterm::event::*;
use openrgb::data::Color;

use crate::core::config_manager::Configuration;

use super::keyboard_controller::KeyboardController;

pub(crate) async fn start_config_creator(
    keyboard_controller: &KeyboardController,
) -> anyhow::Result<Configuration> {
    let mut config = Configuration::default();
    println!("Press every key as it lights up. If no key lights up, press LMB. If no reaction is given when key is pressed, press RMB");

    prepare_terminal_event_capture();
    build_key_led_map(keyboard_controller, &mut config).await?;
    println!("Press the first keys of every row. In order");
    Ok(config)
}

async fn build_key_led_map(
    keyboard_controller: &KeyboardController,
    config: &mut Configuration,
) -> anyhow::Result<()> {
    for index in 0..keyboard_controller.num_leds().await {
        if index != 0 {
            keyboard_controller
                .set_led_by_index(index - 1, Color::new(0, 0, 0))
                .await?;
        }
        keyboard_controller
            .set_led_by_index(index, Color::new(255, 255, 255))
            .await?;
        loop {
            let event = crossterm::event::read().unwrap();
            match event {
                Event::Key(event) => {
                    if event.kind != KeyEventKind::Press {
                        continue;
                    }
                    if event.modifiers.intersects(KeyModifiers::CONTROL)
                        && event.code == KeyCode::Char('c')
                    {
                        crossterm::terminal::disable_raw_mode()?;
                        panic!("Interrupted by user");
                    }
                    config.key_led_map.insert(event.code, index);
                    crossterm::terminal::disable_raw_mode()?;
                    dbg!(&config.key_led_map);
                    crossterm::terminal::enable_raw_mode()?;
                    break;
                }

                Event::Mouse(event) => match event.kind {
                    MouseEventKind::Down(mouse_button) => {
                        if mouse_button == MouseButton::Left {
                            config.skip_indicies.insert(index);
                        }
                        break;
                    }
                    _ => continue,
                },
                _ => continue,
            }
        }
    }
    Ok(())
}
fn prepare_terminal_event_capture() -> anyhow::Result<()> {
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
