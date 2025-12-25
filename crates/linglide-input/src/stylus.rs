//! Virtual stylus/pen handling with pressure and tilt support

use crate::VirtualDevice;
use evdev::{AbsoluteAxisCode, EventType, InputEvent, KeyCode};
use linglide_core::protocol::PenButton;
use linglide_core::Result;
use tracing::debug;

/// Maximum pressure level (4096 levels, 0-4095)
const MAX_PRESSURE: i32 = 4095;

/// Resolution multiplier for sub-pixel precision
const RESOLUTION: i32 = 10;

/// Virtual stylus with pressure, tilt, and button support
/// Compatible with Wacom tablet protocol for drawing applications
pub struct VirtualStylus {
    device: VirtualDevice,
    width: u32,
    height: u32,
    /// Offset for coordinate translation (virtual display position)
    offset_x: i32,
    offset_y: i32,
    /// Current pen state
    in_range: bool,
    tip_down: bool,
    eraser_mode: bool,
    /// Current button states
    stylus_button1: bool,
    stylus_button2: bool,
}

impl VirtualStylus {
    /// Create a new virtual stylus
    pub fn new(width: u32, height: u32, offset_x: i32, offset_y: i32) -> Result<Self> {
        let device = VirtualDevice::new_stylus_with_offset(
            "LinGlide Stylus",
            width,
            height,
            offset_x,
            offset_y,
        )?;

        Ok(Self {
            device,
            width,
            height,
            offset_x,
            offset_y,
            in_range: false,
            tip_down: false,
            eraser_mode: false,
            stylus_button1: false,
            stylus_button2: false,
        })
    }

    /// Convert normalized coordinates (0.0-1.0) to absolute device coordinates
    fn to_absolute(&self, x: f64, y: f64) -> (i32, i32) {
        let abs_x = ((x * self.width as f64) as i32 + self.offset_x) * RESOLUTION;
        let abs_y = ((y * self.height as f64) as i32 + self.offset_y) * RESOLUTION;
        (abs_x, abs_y)
    }

    /// Convert normalized pressure (0.0-1.0) to device pressure level
    fn to_pressure(&self, pressure: f64) -> i32 {
        ((pressure.clamp(0.0, 1.0) * MAX_PRESSURE as f64) as i32).clamp(0, MAX_PRESSURE)
    }

    /// Convert tilt angle in degrees to device tilt value
    fn to_tilt(&self, tilt: f64) -> i32 {
        (tilt.clamp(-90.0, 90.0) as i32).clamp(-90, 90)
    }

    /// Handle pen hover event (pen in range but not touching)
    pub fn pen_hover(
        &mut self,
        x: f64,
        y: f64,
        _pressure: f64,
        tilt_x: f64,
        tilt_y: f64,
    ) -> Result<()> {
        let (abs_x, abs_y) = self.to_absolute(x, y);
        let tilt_x_val = self.to_tilt(tilt_x);
        let tilt_y_val = self.to_tilt(tilt_y);

        debug!(
            "Pen hover: pos=({}, {}), tilt=({}, {})",
            abs_x, abs_y, tilt_x_val, tilt_y_val
        );

        let mut events = Vec::new();

        // Enter range if not already
        if !self.in_range {
            self.in_range = true;
            // Set tool type
            if self.eraser_mode {
                events.push(InputEvent::new(
                    EventType::KEY.0,
                    KeyCode::BTN_TOOL_RUBBER.0,
                    1,
                ));
            } else {
                events.push(InputEvent::new(
                    EventType::KEY.0,
                    KeyCode::BTN_TOOL_PEN.0,
                    1,
                ));
            }
        }

        // Position
        events.push(InputEvent::new(
            EventType::ABSOLUTE.0,
            AbsoluteAxisCode::ABS_X.0,
            abs_x,
        ));
        events.push(InputEvent::new(
            EventType::ABSOLUTE.0,
            AbsoluteAxisCode::ABS_Y.0,
            abs_y,
        ));
        // Pressure (0 for hover)
        events.push(InputEvent::new(
            EventType::ABSOLUTE.0,
            AbsoluteAxisCode::ABS_PRESSURE.0,
            0,
        ));
        // Tilt
        events.push(InputEvent::new(
            EventType::ABSOLUTE.0,
            AbsoluteAxisCode::ABS_TILT_X.0,
            tilt_x_val,
        ));
        events.push(InputEvent::new(
            EventType::ABSOLUTE.0,
            AbsoluteAxisCode::ABS_TILT_Y.0,
            tilt_y_val,
        ));
        // Distance (hovering close to surface)
        events.push(InputEvent::new(
            EventType::ABSOLUTE.0,
            AbsoluteAxisCode::ABS_DISTANCE.0,
            50,
        ));
        // Sync
        events.push(InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0));

        self.device.emit(&events)
    }

    /// Handle pen down event (pen touching surface)
    pub fn pen_down(
        &mut self,
        x: f64,
        y: f64,
        pressure: f64,
        tilt_x: f64,
        tilt_y: f64,
        button: PenButton,
    ) -> Result<()> {
        let (abs_x, abs_y) = self.to_absolute(x, y);
        let pressure_val = self.to_pressure(pressure);
        let tilt_x_val = self.to_tilt(tilt_x);
        let tilt_y_val = self.to_tilt(tilt_y);

        debug!(
            "Pen down: pos=({}, {}), pressure={}, tilt=({}, {}), button={:?}",
            abs_x, abs_y, pressure_val, tilt_x_val, tilt_y_val, button
        );

        let mut events = Vec::new();

        // Set eraser mode based on button
        let new_eraser_mode = matches!(button, PenButton::Eraser);
        if new_eraser_mode != self.eraser_mode {
            // Switch tool type
            if self.in_range {
                // Exit current tool
                if self.eraser_mode {
                    events.push(InputEvent::new(
                        EventType::KEY.0,
                        KeyCode::BTN_TOOL_RUBBER.0,
                        0,
                    ));
                } else {
                    events.push(InputEvent::new(
                        EventType::KEY.0,
                        KeyCode::BTN_TOOL_PEN.0,
                        0,
                    ));
                }
            }
            self.eraser_mode = new_eraser_mode;
        }

        // Enter range if not already
        if !self.in_range {
            self.in_range = true;
        }
        // Set tool type
        if self.eraser_mode {
            events.push(InputEvent::new(
                EventType::KEY.0,
                KeyCode::BTN_TOOL_RUBBER.0,
                1,
            ));
        } else {
            events.push(InputEvent::new(
                EventType::KEY.0,
                KeyCode::BTN_TOOL_PEN.0,
                1,
            ));
        }

        // Position
        events.push(InputEvent::new(
            EventType::ABSOLUTE.0,
            AbsoluteAxisCode::ABS_X.0,
            abs_x,
        ));
        events.push(InputEvent::new(
            EventType::ABSOLUTE.0,
            AbsoluteAxisCode::ABS_Y.0,
            abs_y,
        ));
        // Pressure
        events.push(InputEvent::new(
            EventType::ABSOLUTE.0,
            AbsoluteAxisCode::ABS_PRESSURE.0,
            pressure_val,
        ));
        // Tilt
        events.push(InputEvent::new(
            EventType::ABSOLUTE.0,
            AbsoluteAxisCode::ABS_TILT_X.0,
            tilt_x_val,
        ));
        events.push(InputEvent::new(
            EventType::ABSOLUTE.0,
            AbsoluteAxisCode::ABS_TILT_Y.0,
            tilt_y_val,
        ));
        // Distance (touching surface)
        events.push(InputEvent::new(
            EventType::ABSOLUTE.0,
            AbsoluteAxisCode::ABS_DISTANCE.0,
            0,
        ));
        // Touch down
        events.push(InputEvent::new(EventType::KEY.0, KeyCode::BTN_TOUCH.0, 1));
        // Sync
        events.push(InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0));

        self.tip_down = true;
        self.device.emit(&events)
    }

    /// Handle pen move event (while touching)
    pub fn pen_move(
        &mut self,
        x: f64,
        y: f64,
        pressure: f64,
        tilt_x: f64,
        tilt_y: f64,
    ) -> Result<()> {
        if !self.tip_down {
            // If not touching, treat as hover
            return self.pen_hover(x, y, pressure, tilt_x, tilt_y);
        }

        let (abs_x, abs_y) = self.to_absolute(x, y);
        let pressure_val = self.to_pressure(pressure);
        let tilt_x_val = self.to_tilt(tilt_x);
        let tilt_y_val = self.to_tilt(tilt_y);

        debug!(
            "Pen move: pos=({}, {}), pressure={}, tilt=({}, {})",
            abs_x, abs_y, pressure_val, tilt_x_val, tilt_y_val
        );

        let events = [
            // Position
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_X.0, abs_x),
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_Y.0, abs_y),
            // Pressure
            InputEvent::new(
                EventType::ABSOLUTE.0,
                AbsoluteAxisCode::ABS_PRESSURE.0,
                pressure_val,
            ),
            // Tilt
            InputEvent::new(
                EventType::ABSOLUTE.0,
                AbsoluteAxisCode::ABS_TILT_X.0,
                tilt_x_val,
            ),
            InputEvent::new(
                EventType::ABSOLUTE.0,
                AbsoluteAxisCode::ABS_TILT_Y.0,
                tilt_y_val,
            ),
            // Sync
            InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
        ];

        self.device.emit(&events)
    }

    /// Handle pen up event (lifted from surface)
    pub fn pen_up(&mut self, x: f64, y: f64) -> Result<()> {
        if !self.tip_down {
            return Ok(());
        }

        let (abs_x, abs_y) = self.to_absolute(x, y);

        debug!("Pen up: pos=({}, {})", abs_x, abs_y);

        let events = [
            // Final position
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_X.0, abs_x),
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_Y.0, abs_y),
            // Pressure zero
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_PRESSURE.0, 0),
            // Distance (now hovering)
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_DISTANCE.0, 50),
            // Touch up
            InputEvent::new(EventType::KEY.0, KeyCode::BTN_TOUCH.0, 0),
            // Sync
            InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
        ];

        self.tip_down = false;
        self.device.emit(&events)
    }

    /// Handle pen leaving proximity (out of range)
    pub fn pen_leave(&mut self) -> Result<()> {
        if !self.in_range {
            return Ok(());
        }

        debug!("Pen leave");

        let mut events = Vec::new();

        // If still touching, lift first
        if self.tip_down {
            events.push(InputEvent::new(EventType::KEY.0, KeyCode::BTN_TOUCH.0, 0));
            self.tip_down = false;
        }

        // Release any buttons
        if self.stylus_button1 {
            events.push(InputEvent::new(EventType::KEY.0, KeyCode::BTN_STYLUS.0, 0));
            self.stylus_button1 = false;
        }
        if self.stylus_button2 {
            events.push(InputEvent::new(EventType::KEY.0, KeyCode::BTN_STYLUS2.0, 0));
            self.stylus_button2 = false;
        }

        // Exit tool
        if self.eraser_mode {
            events.push(InputEvent::new(
                EventType::KEY.0,
                KeyCode::BTN_TOOL_RUBBER.0,
                0,
            ));
        } else {
            events.push(InputEvent::new(
                EventType::KEY.0,
                KeyCode::BTN_TOOL_PEN.0,
                0,
            ));
        }

        // Sync
        events.push(InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0));

        self.in_range = false;
        self.device.emit(&events)
    }

    /// Handle stylus button press/release
    pub fn pen_button(&mut self, button: PenButton, pressed: bool) -> Result<()> {
        debug!("Pen button: {:?} = {}", button, pressed);

        let (key_code, state_ref) = match button {
            PenButton::Secondary => (KeyCode::BTN_STYLUS, &mut self.stylus_button1),
            PenButton::Tertiary => (KeyCode::BTN_STYLUS2, &mut self.stylus_button2),
            PenButton::Primary | PenButton::Eraser => {
                // Primary and eraser are handled via pen_down/pen_up
                return Ok(());
            }
        };

        // Only emit if state changed
        if *state_ref == pressed {
            return Ok(());
        }

        *state_ref = pressed;

        let events = [
            InputEvent::new(EventType::KEY.0, key_code.0, if pressed { 1 } else { 0 }),
            InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
        ];

        self.device.emit(&events)
    }

    /// Check if pen is currently in range
    pub fn is_in_range(&self) -> bool {
        self.in_range
    }

    /// Check if pen tip is currently down
    pub fn is_tip_down(&self) -> bool {
        self.tip_down
    }
}
