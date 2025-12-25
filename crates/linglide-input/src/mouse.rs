//! Virtual mouse emulation

use crate::VirtualDevice;
use evdev::{AbsoluteAxisCode, EventType, InputEvent, KeyCode, RelativeAxisCode};
use linglide_core::{Error, Result};
use tracing::debug;

/// Virtual mouse for desktop control
pub struct VirtualMouse {
    device: VirtualDevice,
    width: u32,
    height: u32,
    offset_x: i32,
    offset_y: i32,
    /// Current button states
    button_states: [bool; 3],
}

impl VirtualMouse {
    /// Create a new virtual mouse
    pub fn new(width: u32, height: u32, offset_x: i32, offset_y: i32) -> Result<Self> {
        let device = VirtualDevice::new_absolute_pointer_with_offset(
            "LinGlide Mouse",
            width,
            height,
            offset_x,
            offset_y
        )?;

        Ok(Self {
            device,
            width,
            height,
            offset_x,
            offset_y,
            button_states: [false; 3],
        })
    }

    /// Convert normalized coordinates to absolute coordinates
    fn to_absolute(&self, x: f64, y: f64) -> (i32, i32) {
        let abs_x = (x * self.width as f64) as i32 + self.offset_x;
        let abs_y = (y * self.height as f64) as i32 + self.offset_y;
        (abs_x, abs_y)
    }

    /// Get key code for button index
    fn button_key(button: u8) -> Option<KeyCode> {
        match button {
            0 => Some(KeyCode::BTN_LEFT),
            1 => Some(KeyCode::BTN_MIDDLE),
            2 => Some(KeyCode::BTN_RIGHT),
            _ => None,
        }
    }

    /// Handle mouse move event
    pub fn mouse_move(&mut self, x: f64, y: f64) -> Result<()> {
        let (abs_x, abs_y) = self.to_absolute(x, y);

        debug!("Mouse move: pos=({}, {})", abs_x, abs_y);

        let events = [
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_X.0, abs_x),
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_Y.0, abs_y),
            InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
        ];

        self.device.emit(&events)
    }

    /// Handle mouse button down event
    pub fn mouse_down(&mut self, button: u8, x: f64, y: f64) -> Result<()> {
        let key = Self::button_key(button)
            .ok_or_else(|| Error::InputError(format!("Invalid button: {}", button)))?;

        if button < 3 {
            self.button_states[button as usize] = true;
        }

        let (abs_x, abs_y) = self.to_absolute(x, y);

        debug!("Mouse down: button={}, pos=({}, {})", button, abs_x, abs_y);

        let events = [
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_X.0, abs_x),
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_Y.0, abs_y),
            InputEvent::new(EventType::KEY.0, key.0, 1),
            InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
        ];

        self.device.emit(&events)
    }

    /// Handle mouse button up event
    pub fn mouse_up(&mut self, button: u8, x: f64, y: f64) -> Result<()> {
        let key = Self::button_key(button)
            .ok_or_else(|| Error::InputError(format!("Invalid button: {}", button)))?;

        if button < 3 {
            self.button_states[button as usize] = false;
        }

        let (abs_x, abs_y) = self.to_absolute(x, y);

        debug!("Mouse up: button={}, pos=({}, {})", button, abs_x, abs_y);

        let events = [
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_X.0, abs_x),
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_Y.0, abs_y),
            InputEvent::new(EventType::KEY.0, key.0, 0),
            InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
        ];

        self.device.emit(&events)
    }

    /// Handle scroll event
    pub fn scroll(&mut self, _dx: f64, _dy: f64) -> Result<()> {
        // Scroll not supported by absolute pointer device
        // Use RelativeMouse for scroll
        Ok(())
    }

    /// Click at position (down then up)
    pub fn click(&mut self, button: u8, x: f64, y: f64) -> Result<()> {
        self.mouse_down(button, x, y)?;
        self.mouse_up(button, x, y)
    }

    /// Get button state
    pub fn is_button_pressed(&self, button: u8) -> bool {
        if button < 3 {
            self.button_states[button as usize]
        } else {
            false
        }
    }
}

/// Relative mouse for scroll support
pub struct RelativeMouse {
    device: VirtualDevice,
}

impl RelativeMouse {
    /// Create a new relative mouse (for scroll events)
    pub fn new() -> Result<Self> {
        let device = VirtualDevice::new_mouse("LinGlide Scroll")?;
        Ok(Self { device })
    }

    /// Emit scroll event
    pub fn scroll(&mut self, dx: f64, dy: f64) -> Result<()> {
        let scroll_x = -(dx / 15.0) as i32;
        let scroll_y = -(dy / 15.0) as i32;

        if scroll_x == 0 && scroll_y == 0 {
            return Ok(());
        }

        debug!("Scroll: x={}, y={}", scroll_x, scroll_y);

        let mut events = Vec::new();

        if scroll_y != 0 {
            events.push(InputEvent::new(
                EventType::RELATIVE.0,
                RelativeAxisCode::REL_WHEEL.0,
                scroll_y,
            ));
        }

        if scroll_x != 0 {
            events.push(InputEvent::new(
                EventType::RELATIVE.0,
                RelativeAxisCode::REL_HWHEEL.0,
                scroll_x,
            ));
        }

        events.push(InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0));

        self.device.emit(&events)
    }
}
