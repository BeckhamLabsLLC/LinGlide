//! Wayland screen capture using XDG Desktop Portal and PipeWire

use crate::Frame;
use linglide_core::{Error, Result};
use std::os::fd::{FromRawFd, IntoRawFd, OwnedFd};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info};

/// PipeWire screen capture for Wayland
pub struct PipeWireCapture {
    width: u32,
    height: u32,
    frame_data: Arc<Mutex<Vec<u8>>>,
    sequence: AtomicU64,
    running: Arc<AtomicBool>,
    _thread: Option<std::thread::JoinHandle<()>>,
}

impl PipeWireCapture {
    pub fn new(width: u32, height: u32) -> Result<Self> {
        info!("Initializing Wayland screen capture via portal...");

        let frame_data = Arc::new(Mutex::new(vec![0u8; (width * height * 4) as usize]));
        let running = Arc::new(AtomicBool::new(true));

        let frame_data_clone = frame_data.clone();
        let running_clone = running.clone();

        // Spawn thread to handle portal request and PipeWire stream
        let thread = std::thread::spawn(move || {
            if let Err(e) = run_capture(width, height, frame_data_clone, running_clone) {
                error!("Capture thread error: {}", e);
            }
        });

        // Give the portal dialog time to appear
        std::thread::sleep(std::time::Duration::from_millis(500));

        Ok(Self {
            width,
            height,
            frame_data,
            sequence: AtomicU64::new(0),
            running,
            _thread: Some(thread),
        })
    }

    pub fn capture(&mut self) -> Result<Frame> {
        let data = self
            .frame_data
            .lock()
            .map_err(|_| Error::CaptureError("Lock poisoned".into()))?
            .clone();

        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);
        Ok(Frame::new(data, self.width, self.height, seq))
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl Drop for PipeWireCapture {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

fn run_capture(
    width: u32,
    height: u32,
    frame_data: Arc<Mutex<Vec<u8>>>,
    running: Arc<AtomicBool>,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use ashpd::desktop::screencast::{CursorMode, Screencast, SourceType};
    use ashpd::desktop::PersistMode;

    // Create async runtime for portal
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let (fd, node_id) = rt.block_on(async {
        info!("Requesting screen share permission...");

        let proxy = Screencast::new().await?;
        let session = proxy.create_session().await?;

        proxy
            .select_sources(
                &session,
                CursorMode::Embedded,
                SourceType::Monitor.into(),
                false,
                None,
                PersistMode::DoNot,
            )
            .await?;

        info!("Please select a screen to share in the dialog...");

        let response = proxy.start(&session, None).await?.response()?;
        let streams = response.streams();

        if streams.is_empty() {
            return Err("No screen selected".into());
        }

        let node_id = streams[0].pipe_wire_node_id();
        let fd = proxy.open_pipe_wire_remote(&session).await?;

        info!("Screen share granted, node_id={}", node_id);

        Ok::<_, Box<dyn std::error::Error + Send + Sync>>((fd.into_raw_fd(), node_id))
    })?;

    // Now run PipeWire stream
    run_pipewire(fd, node_id, width, height, frame_data, running)
}

fn run_pipewire(
    fd: i32,
    node_id: u32,
    width: u32,
    height: u32,
    frame_data: Arc<Mutex<Vec<u8>>>,
    running: Arc<AtomicBool>,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use pipewire as pw;

    pw::init();

    let mainloop = pw::main_loop::MainLoop::new(None)?;
    let context = pw::context::Context::new(&mainloop)?;
    let core = context.connect_fd(unsafe { OwnedFd::from_raw_fd(fd) }, None)?;

    let expected_size = (width * height * 4) as usize;
    let frame_data_inner = frame_data.clone();

    let stream = pw::stream::Stream::new(
        &core,
        "linglide",
        pw::properties::properties! {
            *pw::keys::MEDIA_TYPE => "Video",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Screen",
        },
    )?;

    let _listener = stream
        .add_local_listener_with_user_data(())
        .state_changed(|_, _, old, new| {
            debug!("PipeWire state: {:?} -> {:?}", old, new);
        })
        .param_changed(|_, _, id, pod| {
            if id == pw::spa::param::ParamType::Format.as_raw() && pod.is_some() {
                debug!("Format negotiated");
            }
        })
        .process(move |stream, _| {
            static CALL_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let count = CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

            if count == 0 {
                info!("Process callback called for first time");
            }

            match stream.dequeue_buffer() {
                Some(mut buffer) => {
                    let datas = buffer.datas_mut();
                    if count < 3 {
                        info!("Buffer dequeued, datas.len()={}", datas.len());
                    }

                    if !datas.is_empty() {
                        let chunk = datas[0].chunk();
                        let offset = chunk.offset() as usize;
                        let size = chunk.size() as usize;

                        // Check buffer type
                        if count < 3 {
                            let data_type = datas[0].type_();
                            info!(
                                "Buffer type: {:?}, fd: {:?}",
                                data_type,
                                datas[0].as_raw().fd
                            );
                        }

                        // Try to get data - for DMA-BUF we need to mmap the fd
                        let data_result = datas[0].data();

                        if let Some(slice) = data_result {
                            if count < 3 {
                                info!(
                                    "Frame {}: offset={}, size={}, slice_len={}",
                                    count,
                                    offset,
                                    size,
                                    slice.len()
                                );
                            }

                            if size > 0 && offset + size <= slice.len() {
                                let src = &slice[offset..offset + size];
                                if let Ok(mut guard) = frame_data_inner.lock() {
                                    let copy_len = src.len().min(expected_size);
                                    guard[..copy_len].copy_from_slice(&src[..copy_len]);
                                }
                            }
                        } else {
                            // DMA-BUF: need to mmap the file descriptor
                            let raw = datas[0].as_raw();
                            let dmabuf_fd = raw.fd as i32;

                            if dmabuf_fd > 0 {
                                let map_size = raw.maxsize as usize;

                                if count < 3 {
                                    info!("DMA-BUF fd={}, maxsize={}", dmabuf_fd, raw.maxsize);
                                }

                                // Try to mmap the DMA-BUF
                                unsafe {
                                    let ptr = libc::mmap(
                                        std::ptr::null_mut(),
                                        map_size,
                                        libc::PROT_READ,
                                        libc::MAP_SHARED,
                                        dmabuf_fd,
                                        0,
                                    );

                                    if ptr != libc::MAP_FAILED {
                                        let mapped_slice =
                                            std::slice::from_raw_parts(ptr as *const u8, map_size);

                                        if count < 3 {
                                            info!(
                                                "DMA-BUF mapped successfully, {} bytes",
                                                map_size
                                            );
                                        }

                                        if let Ok(mut guard) = frame_data_inner.lock() {
                                            let copy_len = map_size.min(expected_size);
                                            guard[..copy_len]
                                                .copy_from_slice(&mapped_slice[..copy_len]);
                                        }

                                        libc::munmap(ptr, map_size);
                                    } else if count < 10 {
                                        let errno = *libc::__errno_location();
                                        debug!("DMA-BUF mmap failed, errno={}", errno);
                                    }
                                }
                            } else if count < 3 {
                                info!("Frame {}: data() returned None and no valid fd", count);
                            }
                        }
                    }
                }
                None => {
                    if count < 3 {
                        info!("dequeue_buffer returned None");
                    }
                }
            }
        })
        .register()?;

    // Connect to the screencast stream with MAP_BUFFERS to request memory-mapped buffers
    stream.connect(
        pw::spa::utils::Direction::Input,
        Some(node_id),
        pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
        &mut [],
    )?;

    info!("PipeWire stream connected, capturing...");

    // Run until stopped - iterate the loop and check for shutdown
    while running.load(Ordering::SeqCst) {
        mainloop
            .loop_()
            .iterate(std::time::Duration::from_millis(16));
    }

    Ok(())
}
