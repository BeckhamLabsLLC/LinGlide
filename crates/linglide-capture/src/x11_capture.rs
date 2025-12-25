//! X11 screen capture using MIT-SHM extension

use crate::Frame;
use linglide_core::{Error, Result};
use std::ptr;
use tracing::{debug, info};

/// X11 screen capture using MIT-SHM for zero-copy performance
pub struct X11Capture {
    conn: xcb::Connection,
    screen_num: i32,
    shm_seg: xcb::shm::Seg,
    shm_id: i32,
    shm_addr: *mut libc::c_void,
    width: u32,
    height: u32,
    offset_x: i32,
    offset_y: i32,
    sequence: u64,
}

// Safety: X11 connection and SHM are managed properly
unsafe impl Send for X11Capture {}

impl X11Capture {
    /// Create a new X11 capture instance
    pub fn new(width: u32, height: u32, offset_x: i32, offset_y: i32) -> Result<Self> {
        // Connect to X11
        let (conn, screen_num) = xcb::Connection::connect(None)
            .map_err(|e| Error::X11Connection(e.to_string()))?;

        // Check for SHM extension
        let shm_cookie = conn.send_request(&xcb::shm::QueryVersion {});
        conn.wait_for_reply(shm_cookie)
            .map_err(|_| Error::X11ExtensionMissing("MIT-SHM".to_string()))?;

        info!("MIT-SHM extension available");

        // Calculate buffer size
        let buffer_size = (width * height * 4) as usize;

        // Create shared memory segment
        let shm_id = unsafe {
            libc::shmget(
                libc::IPC_PRIVATE,
                buffer_size,
                libc::IPC_CREAT | 0o777,
            )
        };

        if shm_id < 0 {
            return Err(Error::CaptureError(format!(
                "shmget failed: {}",
                std::io::Error::last_os_error()
            )));
        }

        // Attach shared memory
        let shm_addr = unsafe { libc::shmat(shm_id, ptr::null(), 0) };
        if shm_addr == libc::MAP_FAILED {
            unsafe { libc::shmctl(shm_id, libc::IPC_RMID, ptr::null_mut()) };
            return Err(Error::CaptureError(format!(
                "shmat failed: {}",
                std::io::Error::last_os_error()
            )));
        }

        // Generate SHM segment ID for X11
        let shm_seg: xcb::shm::Seg = conn.generate_id();

        // Attach SHM to X server
        conn.send_request(&xcb::shm::Attach {
            shmseg: shm_seg,
            shmid: shm_id as u32,
            read_only: false,
        });

        conn.flush()
            .map_err(|e| Error::X11Connection(e.to_string()))?;

        debug!(
            "X11 capture initialized: {}x{} at offset ({}, {})",
            width, height, offset_x, offset_y
        );

        Ok(Self {
            conn,
            screen_num,
            shm_seg,
            shm_id,
            shm_addr,
            width,
            height,
            offset_x,
            offset_y,
            sequence: 0,
        })
    }

    /// Capture a single frame
    pub fn capture(&mut self) -> Result<Frame> {
        let setup = self.conn.get_setup();
        let screen = setup
            .roots()
            .nth(self.screen_num as usize)
            .ok_or_else(|| Error::X11Connection("Invalid screen".to_string()))?;

        let root = screen.root();

        // Request the image via SHM
        let cookie = self.conn.send_request(&xcb::shm::GetImage {
            drawable: xcb::x::Drawable::Window(root),
            x: self.offset_x as i16,
            y: self.offset_y as i16,
            width: self.width as u16,
            height: self.height as u16,
            plane_mask: !0,
            format: xcb::x::ImageFormat::ZPixmap as u8,
            shmseg: self.shm_seg,
            offset: 0,
        });

        self.conn
            .wait_for_reply(cookie)
            .map_err(|e| Error::CaptureError(format!("GetImage failed: {:?}", e)))?;

        // Copy data from shared memory
        let buffer_size = (self.width * self.height * 4) as usize;
        let data = unsafe {
            std::slice::from_raw_parts(self.shm_addr as *const u8, buffer_size).to_vec()
        };

        self.sequence += 1;

        Ok(Frame::new(data, self.width, self.height, self.sequence))
    }

    /// Get the capture dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Get the capture offset
    pub fn offset(&self) -> (i32, i32) {
        (self.offset_x, self.offset_y)
    }
}

impl Drop for X11Capture {
    fn drop(&mut self) {
        // Detach from X server
        self.conn.send_request(&xcb::shm::Detach {
            shmseg: self.shm_seg,
        });
        let _ = self.conn.flush();

        // Detach and remove shared memory
        unsafe {
            libc::shmdt(self.shm_addr);
            libc::shmctl(self.shm_id, libc::IPC_RMID, ptr::null_mut());
        }

        debug!("X11 capture resources cleaned up");
    }
}
