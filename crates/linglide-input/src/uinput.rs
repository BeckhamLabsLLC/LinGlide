//! Virtual uinput device creation

use evdev::{
    uinput::VirtualDevice as EvdevVirtualDevice, AbsInfo, AbsoluteAxisCode, AttributeSet,
    InputEvent, KeyCode, RelativeAxisCode, UinputAbsSetup,
};
use linglide_core::{Error, Result};
use tracing::info;

/// Wrapper for evdev virtual device
pub struct VirtualDevice {
    device: EvdevVirtualDevice,
    name: String,
}

impl VirtualDevice {
    /// Create a new virtual mouse device
    pub fn new_mouse(name: &str) -> Result<Self> {
        let mut keys = AttributeSet::<KeyCode>::new();
        keys.insert(KeyCode::BTN_LEFT);
        keys.insert(KeyCode::BTN_RIGHT);
        keys.insert(KeyCode::BTN_MIDDLE);

        let mut rel_axes = AttributeSet::<RelativeAxisCode>::new();
        rel_axes.insert(RelativeAxisCode::REL_X);
        rel_axes.insert(RelativeAxisCode::REL_Y);
        rel_axes.insert(RelativeAxisCode::REL_WHEEL);
        rel_axes.insert(RelativeAxisCode::REL_HWHEEL);

        let device = EvdevVirtualDevice::builder()
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .name(name)
            .with_keys(&keys)
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .with_relative_axes(&rel_axes)
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .build()
            .map_err(|e| Error::UinputCreation(e.to_string()))?;

        info!("Created virtual mouse: {}", name);

        Ok(Self {
            device,
            name: name.to_string(),
        })
    }

    /// Create a new virtual absolute pointer device with offset support
    pub fn new_absolute_pointer_with_offset(
        name: &str,
        width: u32,
        height: u32,
        offset_x: i32,
        offset_y: i32,
    ) -> Result<Self> {
        let mut keys = AttributeSet::<KeyCode>::new();
        keys.insert(KeyCode::BTN_TOUCH);
        keys.insert(KeyCode::BTN_TOOL_FINGER);
        keys.insert(KeyCode::BTN_LEFT);
        keys.insert(KeyCode::BTN_RIGHT);
        keys.insert(KeyCode::BTN_MIDDLE);

        // Extend bounds to cover offset + size
        let max_x = offset_x + width as i32;
        let max_y = offset_y + height as i32;
        let x_abs = AbsInfo::new(0, 0, max_x, 0, 0, 1);
        let y_abs = AbsInfo::new(0, 0, max_y, 0, 0, 1);

        let device = EvdevVirtualDevice::builder()
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .name(name)
            .with_keys(&keys)
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_X, x_abs))
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_Y, y_abs))
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .build()
            .map_err(|e| Error::UinputCreation(e.to_string()))?;

        info!(
            "Created virtual absolute pointer: {} ({}x{} at offset {},{})",
            name, width, height, offset_x, offset_y
        );

        Ok(Self {
            device,
            name: name.to_string(),
        })
    }

    /// Create a new virtual absolute pointer device (legacy, no offset)
    pub fn new_absolute_pointer(name: &str, width: u32, height: u32) -> Result<Self> {
        Self::new_absolute_pointer_with_offset(name, width, height, 0, 0)
    }

    /// Create a new multitouch device with offset support
    /// The device bounds cover the full desktop coordinate space
    pub fn new_multitouch_with_offset(
        name: &str,
        width: u32,
        height: u32,
        offset_x: i32,
        offset_y: i32,
        max_slots: u32,
    ) -> Result<Self> {
        let mut keys = AttributeSet::<KeyCode>::new();
        keys.insert(KeyCode::BTN_TOUCH);
        keys.insert(KeyCode::BTN_TOOL_FINGER);

        // Multitouch axes - extend to cover offset + size
        let max_x = offset_x + width as i32;
        let max_y = offset_y + height as i32;
        let x_abs = AbsInfo::new(0, 0, max_x, 0, 0, 1);
        let y_abs = AbsInfo::new(0, 0, max_y, 0, 0, 1);
        let slot_abs = AbsInfo::new(0, 0, (max_slots - 1) as i32, 0, 0, 0);
        let tracking_abs = AbsInfo::new(0, 0, 65535, 0, 0, 0);

        let device = EvdevVirtualDevice::builder()
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .name(name)
            .with_keys(&keys)
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_X, x_abs))
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_Y, y_abs))
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_MT_SLOT,
                slot_abs,
            ))
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_MT_TRACKING_ID,
                tracking_abs,
            ))
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_MT_POSITION_X,
                x_abs,
            ))
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_MT_POSITION_Y,
                y_abs,
            ))
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .build()
            .map_err(|e| Error::UinputCreation(e.to_string()))?;

        info!(
            "Created virtual multitouch: {} ({}x{} at offset {},{}, {} slots)",
            name, width, height, offset_x, offset_y, max_slots
        );

        Ok(Self {
            device,
            name: name.to_string(),
        })
    }

    /// Create a new multitouch device (legacy, no offset)
    pub fn new_multitouch(name: &str, width: u32, height: u32, max_slots: u32) -> Result<Self> {
        Self::new_multitouch_with_offset(name, width, height, 0, 0, max_slots)
    }

    /// Create a new virtual stylus/pen device with pressure and tilt support
    /// Compatible with Wacom tablet protocol for drawing applications
    pub fn new_stylus_with_offset(
        name: &str,
        width: u32,
        height: u32,
        offset_x: i32,
        offset_y: i32,
    ) -> Result<Self> {
        let mut keys = AttributeSet::<KeyCode>::new();
        // Tool type buttons
        keys.insert(KeyCode::BTN_TOUCH);
        keys.insert(KeyCode::BTN_TOOL_PEN);
        keys.insert(KeyCode::BTN_TOOL_RUBBER); // Eraser end
                                               // Stylus buttons
        keys.insert(KeyCode::BTN_STYLUS); // Barrel button 1
        keys.insert(KeyCode::BTN_STYLUS2); // Barrel button 2

        // Position axes with 10x resolution for sub-pixel precision
        let resolution = 10;
        let max_x = (offset_x + width as i32) * resolution;
        let max_y = (offset_y + height as i32) * resolution;
        let x_abs = AbsInfo::new(0, 0, max_x, 0, 0, resolution);
        let y_abs = AbsInfo::new(0, 0, max_y, 0, 0, resolution);
        // Pressure: 4096 levels (standard for professional tablets)
        let pressure_abs = AbsInfo::new(0, 0, 4095, 0, 0, 0);
        // Tilt: -90 to 90 degrees
        let tilt_abs = AbsInfo::new(0, -90, 90, 0, 0, 0);
        // Distance for hover detection (0-255)
        let distance_abs = AbsInfo::new(0, 0, 255, 0, 0, 0);

        let device = EvdevVirtualDevice::builder()
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .name(name)
            .with_keys(&keys)
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_X, x_abs))
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_Y, y_abs))
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_PRESSURE,
                pressure_abs,
            ))
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_TILT_X, tilt_abs))
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_TILT_Y, tilt_abs))
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_DISTANCE,
                distance_abs,
            ))
            .map_err(|e| Error::UinputCreation(e.to_string()))?
            .build()
            .map_err(|e| Error::UinputCreation(e.to_string()))?;

        info!(
            "Created virtual stylus: {} ({}x{} at offset {},{}, 4096 pressure levels)",
            name, width, height, offset_x, offset_y
        );

        Ok(Self {
            device,
            name: name.to_string(),
        })
    }

    /// Create a new virtual stylus/pen device (legacy, no offset)
    pub fn new_stylus(name: &str, width: u32, height: u32) -> Result<Self> {
        Self::new_stylus_with_offset(name, width, height, 0, 0)
    }

    /// Emit input events
    pub fn emit(&mut self, events: &[InputEvent]) -> Result<()> {
        self.device
            .emit(events)
            .map_err(|e| Error::InputError(e.to_string()))
    }

    /// Get the device name
    pub fn name(&self) -> &str {
        &self.name
    }
}
