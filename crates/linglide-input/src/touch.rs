//! Virtual touchscreen handling with multitouch support

use crate::VirtualDevice;
use evdev::{AbsoluteAxisCode, EventType, InputEvent, KeyCode};
use linglide_core::{Error, Result};
use std::collections::HashMap;
use tracing::{debug, info};

/// Virtual touchscreen with multitouch protocol type B support
pub struct VirtualTouchscreen {
    device: VirtualDevice,
    width: u32,
    height: u32,
    /// Active touch points (id -> slot)
    active_touches: HashMap<u32, u32>,
    /// Available slots
    max_slots: u32,
    /// Next tracking ID
    next_tracking_id: u32,
    /// Offset for coordinate translation
    offset_x: i32,
    offset_y: i32,
}

impl VirtualTouchscreen {
    /// Create a new virtual touchscreen
    pub fn new(width: u32, height: u32, offset_x: i32, offset_y: i32) -> Result<Self> {
        let max_slots = 10;
        let device = VirtualDevice::new_multitouch_with_offset(
            "LinGlide Touchscreen",
            width,
            height,
            offset_x,
            offset_y,
            max_slots
        )?;

        Ok(Self {
            device,
            width,
            height,
            active_touches: HashMap::new(),
            max_slots,
            next_tracking_id: 0,
            offset_x,
            offset_y,
        })
    }

    /// Find an available slot for a new touch
    fn find_free_slot(&self) -> Option<u32> {
        let used_slots: std::collections::HashSet<_> = self.active_touches.values().copied().collect();
        (0..self.max_slots).find(|slot| !used_slots.contains(slot))
    }

    /// Convert normalized coordinates to absolute coordinates
    fn to_absolute(&self, x: f64, y: f64) -> (i32, i32) {
        let abs_x = (x * self.width as f64) as i32 + self.offset_x;
        let abs_y = (y * self.height as f64) as i32 + self.offset_y;
        (abs_x, abs_y)
    }

    /// Handle touch start event
    pub fn touch_start(&mut self, id: u32, x: f64, y: f64) -> Result<()> {
        let slot = self.find_free_slot()
            .ok_or_else(|| Error::InputError("No available touch slots".to_string()))?;

        let tracking_id = self.next_tracking_id;
        self.next_tracking_id = self.next_tracking_id.wrapping_add(1);
        self.active_touches.insert(id, slot);

        let (abs_x, abs_y) = self.to_absolute(x, y);

        info!("Touch start: id={}, slot={}, norm=({:.3}, {:.3}), abs=({}, {}), offset=({}, {})",
              id, slot, x, y, abs_x, abs_y, self.offset_x, self.offset_y);

        let events = [
            // Select slot
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_MT_SLOT.0, slot as i32),
            // Set tracking ID (new touch)
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_MT_TRACKING_ID.0, tracking_id as i32),
            // Set position
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_MT_POSITION_X.0, abs_x),
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_MT_POSITION_Y.0, abs_y),
            // Also update single-touch axes for compatibility
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_X.0, abs_x),
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_Y.0, abs_y),
            // Touch down
            InputEvent::new(EventType::KEY.0, KeyCode::BTN_TOUCH.0, 1),
            // Sync
            InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
        ];

        let result = self.device.emit(&events);
        if let Err(ref e) = result {
            info!("Touch emit error: {:?}", e);
        }
        result
    }

    /// Handle touch move event
    pub fn touch_move(&mut self, id: u32, x: f64, y: f64) -> Result<()> {
        let slot = *self.active_touches.get(&id)
            .ok_or_else(|| Error::InputError(format!("Unknown touch id: {}", id)))?;

        let (abs_x, abs_y) = self.to_absolute(x, y);

        debug!("Touch move: id={}, slot={}, pos=({}, {})", id, slot, abs_x, abs_y);

        let events = [
            // Select slot
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_MT_SLOT.0, slot as i32),
            // Update position
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_MT_POSITION_X.0, abs_x),
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_MT_POSITION_Y.0, abs_y),
            // Also update single-touch axes
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_X.0, abs_x),
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_Y.0, abs_y),
            // Sync
            InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
        ];

        self.device.emit(&events)
    }

    /// Handle touch end event
    pub fn touch_end(&mut self, id: u32) -> Result<()> {
        let slot = self.active_touches.remove(&id)
            .ok_or_else(|| Error::InputError(format!("Unknown touch id: {}", id)))?;

        debug!("Touch end: id={}, slot={}", id, slot);

        let mut events = vec![
            // Select slot
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_MT_SLOT.0, slot as i32),
            // Set tracking ID to -1 (touch lifted)
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_MT_TRACKING_ID.0, -1),
        ];

        // If no more touches, send BTN_TOUCH up
        if self.active_touches.is_empty() {
            events.push(InputEvent::new(EventType::KEY.0, KeyCode::BTN_TOUCH.0, 0));
        }

        events.push(InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0));

        self.device.emit(&events)
    }

    /// Handle touch cancel (same as end)
    pub fn touch_cancel(&mut self, id: u32) -> Result<()> {
        self.touch_end(id)
    }

    /// Get the number of active touches
    pub fn active_touch_count(&self) -> usize {
        self.active_touches.len()
    }
}
