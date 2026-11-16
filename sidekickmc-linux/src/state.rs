use std::time::{Duration, Instant};
use crate::config::{Action, ChordAction, Config};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideButton {
    Back = 1,
    Forward = 2,
}

#[derive(Debug)]
pub enum GestureResult {
    /// Forward event unmodified to virtual mouse
    PassThrough,
    /// Event was consumed by gesture engine (do not forward)
    Consumed,
    /// Adjust volume or brightness
    TriggerAction(Action, i32),
    /// Trigger screen snipping overlay
    TriggerChord,
    /// Re-emit a single tap click for Back or Forward
    EmitTap(SideButton),
}

pub struct GestureEngine {
    config: Config,
    held: u8,
    started: Option<Instant>,
    scrolled: bool,
    chord: bool,
    snip_on_release: bool,
    wheel_remainder: i32,
}

impl GestureEngine {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            held: 0,
            started: None,
            scrolled: false,
            chord: false,
            snip_on_release: false,
            wheel_remainder: 0,
        }
    }

    pub fn update_config(&mut self, config: Config) {
        self.config = config;
    }

    pub fn handle_button(&mut self, button: SideButton, is_press: bool) -> GestureResult {
        let bit = button as u8;

        if is_press {
            let now = Instant::now();
            if self.held == 0 {
                self.started = Some(now);
                self.scrolled = false;
                self.wheel_remainder = 0;
            }

            // If already holding one button and the other is pressed within the chord window
            if self.held != 0 && (self.held & bit) == 0 && !self.chord {
                self.chord = true;
                let elapsed = self.started.map(|s| now.duration_since(s)).unwrap_or(Duration::ZERO);
                let chord_timeout = Duration::from_millis(self.config.chord_window_ms);
                self.snip_on_release = self.config.chord_action == ChordAction::SnippingTool && elapsed <= chord_timeout;
            }

            self.held |= bit;
            GestureResult::Consumed
        } else {
            // Button release
            if (self.held & bit) != 0 {
                let is_tap = !self.chord && !self.scrolled;
                self.held &= !bit;

                let mut result = GestureResult::Consumed;

                if is_tap {
                    result = GestureResult::EmitTap(button);
                }

                if self.held == 0 {
                    self.chord = false;
                    self.started = None;
                    if self.snip_on_release {
                        self.snip_on_release = false;
                        return GestureResult::TriggerChord;
                    }
                }

                result
            } else {
                GestureResult::PassThrough
            }
        }
    }

    pub fn handle_wheel(&mut self, delta: i32) -> GestureResult {
        if !self.chord && self.held != 0 {
            self.scrolled = true;
            self.wheel_remainder += delta;

            // In Linux evdev, standard REL_WHEEL is usually 1 or -1 per tick.
            // If delta is larger (e.g. 120 from high-res mice), we step by 120.
            let steps = if delta.abs() >= 120 {
                let s = self.wheel_remainder / 120;
                self.wheel_remainder %= 120;
                s
            } else {
                let s = self.wheel_remainder;
                self.wheel_remainder = 0;
                s
            };

            if steps != 0 {
                let action = if (self.held & (SideButton::Back as u8)) != 0 {
                    &self.config.back_action
                } else {
                    &self.config.forward_action
                };
                return GestureResult::TriggerAction(action.clone(), steps);
            }

            GestureResult::Consumed
        } else {
            GestureResult::PassThrough
        }
    }

    pub fn is_holding(&self) -> bool {
        self.held != 0
    }
}
