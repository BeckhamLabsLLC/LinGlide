//! Fragmented MP4 muxer for browser-compatible streaming

use crate::encoder::EncodedFrame;
use bytes::{BufMut, BytesMut};

/// Fragmented MP4 muxer for H.264 streams
pub struct Fmp4Muxer {
    width: u32,
    height: u32,
    timescale: u32,
    sequence_number: u32,
    sps: Vec<u8>,
    pps: Vec<u8>,
}

impl Fmp4Muxer {
    /// Create a new fMP4 muxer
    pub fn new(width: u32, height: u32, fps: u32) -> Self {
        Self {
            width,
            height,
            timescale: fps * 1000, // Higher timescale for precision
            sequence_number: 1,
            sps: Vec::new(),
            pps: Vec::new(),
        }
    }

    /// Get the codec string for WebCodecs (avc1.PPCCLL format)
    pub fn get_codec_string(&self) -> String {
        if self.sps.len() >= 4 {
            format!(
                "avc1.{:02x}{:02x}{:02x}",
                self.sps[1], self.sps[2], self.sps[3]
            )
        } else {
            // Fallback: High profile, level 4.2
            "avc1.64002a".to_string()
        }
    }

    /// Get the avcC box data for WebCodecs description
    pub fn get_avcc_data(&self) -> Vec<u8> {
        let mut buf = BytesMut::new();
        buf.put_u8(1); // version
        buf.put_u8(if self.sps.len() > 1 {
            self.sps[1]
        } else {
            0x64
        }); // profile
        buf.put_u8(if self.sps.len() > 2 {
            self.sps[2]
        } else {
            0x00
        }); // profile compat
        buf.put_u8(if self.sps.len() > 3 {
            self.sps[3]
        } else {
            0x2a
        }); // level
        buf.put_u8(0xFF); // length size minus one (3 = 4 bytes)
        buf.put_u8(0xE1); // num SPS (1)
        buf.put_u16(self.sps.len() as u16);
        buf.put_slice(&self.sps);
        buf.put_u8(1); // num PPS
        buf.put_u16(self.pps.len() as u16);
        buf.put_slice(&self.pps);
        buf.to_vec()
    }

    /// Parse SPS and PPS from H.264 headers
    pub fn set_headers(&mut self, headers: &[u8]) {
        let mut i = 0;
        while i + 4 < headers.len() {
            // Look for start codes
            if headers[i] == 0 && headers[i + 1] == 0 && headers[i + 2] == 0 && headers[i + 3] == 1
            {
                let start = i + 4;
                // Find next start code or end
                let mut end = headers.len();
                for j in start..headers.len().saturating_sub(3) {
                    if headers[j] == 0
                        && headers[j + 1] == 0
                        && headers[j + 2] == 0
                        && headers[j + 3] == 1
                    {
                        end = j;
                        break;
                    }
                }

                if start < end {
                    let nal_type = headers[start] & 0x1F;
                    match nal_type {
                        7 => self.sps = headers[start..end].to_vec(), // SPS
                        8 => self.pps = headers[start..end].to_vec(), // PPS
                        _ => {}
                    }
                }
                i = end;
            } else {
                i += 1;
            }
        }
    }

    /// Generate the initialization segment (ftyp + moov)
    pub fn create_init_segment(&self) -> Vec<u8> {
        let mut buf = BytesMut::new();

        // ftyp box
        self.write_ftyp(&mut buf);

        // moov box
        self.write_moov(&mut buf);

        buf.to_vec()
    }

    /// Create a media segment for the given frame
    pub fn create_media_segment(&mut self, frame: &EncodedFrame, duration: u32) -> Vec<u8> {
        let mut buf = BytesMut::new();

        // moof box
        self.write_moof(&mut buf, frame, duration);

        // mdat box
        self.write_mdat(&mut buf, &frame.data);

        self.sequence_number += 1;

        buf.to_vec()
    }

    fn write_box(buf: &mut BytesMut, box_type: &[u8; 4], content: &[u8]) {
        let size = 8 + content.len() as u32;
        buf.put_u32(size);
        buf.put_slice(box_type);
        buf.put_slice(content);
    }

    fn write_ftyp(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        content.put_slice(b"isom"); // major brand
        content.put_u32(0x200); // minor version
        content.put_slice(b"isomiso2avc1mp41"); // compatible brands
        Self::write_box(buf, b"ftyp", &content);
    }

    fn write_moov(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        self.write_mvhd(&mut content);
        self.write_trak(&mut content);
        self.write_mvex(&mut content);
        Self::write_box(buf, b"moov", &content);
    }

    fn write_mvhd(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        content.put_u8(0); // version
        content.put_slice(&[0, 0, 0]); // flags
        content.put_u32(0); // creation time
        content.put_u32(0); // modification time
        content.put_u32(self.timescale); // timescale
        content.put_u32(0); // duration
        content.put_u32(0x00010000); // rate (1.0)
        content.put_u16(0x0100); // volume (1.0)
        content.put_u16(0); // reserved
        content.put_u64(0); // reserved
                            // Matrix (identity)
        content.put_u32(0x00010000);
        content.put_u32(0);
        content.put_u32(0);
        content.put_u32(0);
        content.put_u32(0x00010000);
        content.put_u32(0);
        content.put_u32(0);
        content.put_u32(0);
        content.put_u32(0x40000000);
        // Pre-defined
        for _ in 0..6 {
            content.put_u32(0);
        }
        content.put_u32(2); // next track ID
        Self::write_box(buf, b"mvhd", &content);
    }

    fn write_trak(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        self.write_tkhd(&mut content);
        self.write_mdia(&mut content);
        Self::write_box(buf, b"trak", &content);
    }

    fn write_tkhd(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        content.put_u8(0); // version
        content.put_slice(&[0, 0, 3]); // flags (track enabled + in movie)
        content.put_u32(0); // creation time
        content.put_u32(0); // modification time
        content.put_u32(1); // track ID
        content.put_u32(0); // reserved
        content.put_u32(0); // duration
        content.put_u64(0); // reserved
        content.put_u16(0); // layer
        content.put_u16(0); // alternate group
        content.put_u16(0); // volume
        content.put_u16(0); // reserved
                            // Matrix (identity)
        content.put_u32(0x00010000);
        content.put_u32(0);
        content.put_u32(0);
        content.put_u32(0);
        content.put_u32(0x00010000);
        content.put_u32(0);
        content.put_u32(0);
        content.put_u32(0);
        content.put_u32(0x40000000);
        content.put_u32(self.width << 16); // width (fixed-point)
        content.put_u32(self.height << 16); // height (fixed-point)
        Self::write_box(buf, b"tkhd", &content);
    }

    fn write_mdia(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        self.write_mdhd(&mut content);
        self.write_hdlr(&mut content);
        self.write_minf(&mut content);
        Self::write_box(buf, b"mdia", &content);
    }

    fn write_mdhd(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        content.put_u8(0); // version
        content.put_slice(&[0, 0, 0]); // flags
        content.put_u32(0); // creation time
        content.put_u32(0); // modification time
        content.put_u32(self.timescale);
        content.put_u32(0); // duration
        content.put_u16(0x55C4); // language (und)
        content.put_u16(0); // pre-defined
        Self::write_box(buf, b"mdhd", &content);
    }

    fn write_hdlr(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        content.put_u8(0); // version
        content.put_slice(&[0, 0, 0]); // flags
        content.put_u32(0); // pre-defined
        content.put_slice(b"vide"); // handler type
        content.put_u32(0); // reserved
        content.put_u32(0);
        content.put_u32(0);
        content.put_slice(b"VideoHandler\0"); // name
        Self::write_box(buf, b"hdlr", &content);
    }

    fn write_minf(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        self.write_vmhd(&mut content);
        self.write_dinf(&mut content);
        self.write_stbl(&mut content);
        Self::write_box(buf, b"minf", &content);
    }

    fn write_vmhd(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        content.put_u8(0); // version
        content.put_slice(&[0, 0, 1]); // flags
        content.put_u16(0); // graphics mode
        content.put_u16(0); // opcolor
        content.put_u16(0);
        content.put_u16(0);
        Self::write_box(buf, b"vmhd", &content);
    }

    fn write_dinf(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        self.write_dref(&mut content);
        Self::write_box(buf, b"dinf", &content);
    }

    fn write_dref(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        content.put_u8(0); // version
        content.put_slice(&[0, 0, 0]); // flags
        content.put_u32(1); // entry count

        // url entry
        let mut url = BytesMut::new();
        url.put_u8(0); // version
        url.put_slice(&[0, 0, 1]); // flags (self-contained)
        Self::write_box(&mut content, b"url ", &url);

        Self::write_box(buf, b"dref", &content);
    }

    fn write_stbl(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        self.write_stsd(&mut content);
        self.write_stts(&mut content);
        self.write_stsc(&mut content);
        self.write_stsz(&mut content);
        self.write_stco(&mut content);
        Self::write_box(buf, b"stbl", &content);
    }

    fn write_stsd(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        content.put_u8(0); // version
        content.put_slice(&[0, 0, 0]); // flags
        content.put_u32(1); // entry count

        self.write_avc1(&mut content);

        Self::write_box(buf, b"stsd", &content);
    }

    fn write_avc1(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        content.put_slice(&[0; 6]); // reserved
        content.put_u16(1); // data reference index
        content.put_u16(0); // pre-defined
        content.put_u16(0); // reserved
        content.put_u32(0); // pre-defined
        content.put_u32(0);
        content.put_u32(0);
        content.put_u16(self.width as u16);
        content.put_u16(self.height as u16);
        content.put_u32(0x00480000); // horiz resolution (72 dpi)
        content.put_u32(0x00480000); // vert resolution (72 dpi)
        content.put_u32(0); // reserved
        content.put_u16(1); // frame count
        content.put_slice(&[0; 32]); // compressor name
        content.put_u16(0x0018); // depth (24-bit color)
        content.put_i16(-1); // pre-defined

        self.write_avcc(&mut content);

        Self::write_box(buf, b"avc1", &content);
    }

    fn write_avcc(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        content.put_u8(1); // version
        content.put_u8(if self.sps.len() > 1 {
            self.sps[1]
        } else {
            0x64
        }); // profile
        content.put_u8(if self.sps.len() > 2 {
            self.sps[2]
        } else {
            0x00
        }); // profile compat
        content.put_u8(if self.sps.len() > 3 {
            self.sps[3]
        } else {
            0x1F
        }); // level
        content.put_u8(0xFF); // length size minus one (3 = 4 bytes)
        content.put_u8(0xE1); // num SPS (1)
        content.put_u16(self.sps.len() as u16);
        content.put_slice(&self.sps);
        content.put_u8(1); // num PPS
        content.put_u16(self.pps.len() as u16);
        content.put_slice(&self.pps);
        Self::write_box(buf, b"avcC", &content);
    }

    fn write_stts(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        content.put_u8(0); // version
        content.put_slice(&[0, 0, 0]); // flags
        content.put_u32(0); // entry count (empty for fragmented)
        Self::write_box(buf, b"stts", &content);
    }

    fn write_stsc(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        content.put_u8(0);
        content.put_slice(&[0, 0, 0]);
        content.put_u32(0);
        Self::write_box(buf, b"stsc", &content);
    }

    fn write_stsz(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        content.put_u8(0);
        content.put_slice(&[0, 0, 0]);
        content.put_u32(0); // sample size
        content.put_u32(0); // sample count
        Self::write_box(buf, b"stsz", &content);
    }

    fn write_stco(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        content.put_u8(0);
        content.put_slice(&[0, 0, 0]);
        content.put_u32(0);
        Self::write_box(buf, b"stco", &content);
    }

    fn write_mvex(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        self.write_trex(&mut content);
        Self::write_box(buf, b"mvex", &content);
    }

    fn write_trex(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        content.put_u8(0);
        content.put_slice(&[0, 0, 0]);
        content.put_u32(1); // track ID
        content.put_u32(1); // default sample description index
        content.put_u32(0); // default sample duration
        content.put_u32(0); // default sample size
        content.put_u32(0); // default sample flags
        Self::write_box(buf, b"trex", &content);
    }

    fn write_moof(&self, buf: &mut BytesMut, frame: &EncodedFrame, duration: u32) {
        let mut content = BytesMut::new();
        self.write_mfhd(&mut content);
        self.write_traf(&mut content, frame, duration);
        Self::write_box(buf, b"moof", &content);
    }

    fn write_mfhd(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        content.put_u8(0);
        content.put_slice(&[0, 0, 0]);
        content.put_u32(self.sequence_number);
        Self::write_box(buf, b"mfhd", &content);
    }

    fn write_traf(&self, buf: &mut BytesMut, frame: &EncodedFrame, duration: u32) {
        let mut content = BytesMut::new();
        self.write_tfhd(&mut content);
        self.write_tfdt(&mut content, frame.pts as u64 * duration as u64);
        self.write_trun(&mut content, frame, duration);
        Self::write_box(buf, b"traf", &content);
    }

    fn write_tfhd(&self, buf: &mut BytesMut) {
        let mut content = BytesMut::new();
        content.put_u8(0);
        content.put_slice(&[0x02, 0x00, 0x20]); // flags: default-base-is-moof + default-sample-flags
        content.put_u32(1); // track ID
        content.put_u32(0x01010000); // default sample flags (non-keyframe)
        Self::write_box(buf, b"tfhd", &content);
    }

    fn write_tfdt(&self, buf: &mut BytesMut, decode_time: u64) {
        let mut content = BytesMut::new();
        content.put_u8(1); // version 1 for 64-bit time
        content.put_slice(&[0, 0, 0]);
        content.put_u64(decode_time);
        Self::write_box(buf, b"tfdt", &content);
    }

    fn write_trun(&self, buf: &mut BytesMut, frame: &EncodedFrame, duration: u32) {
        let mut content = BytesMut::new();
        content.put_u8(0);
        // flags: data-offset + sample-duration + sample-size + sample-flags
        content.put_slice(&[0x00, 0x0F, 0x01]);
        content.put_u32(1); // sample count

        // Calculate data offset (moof size + mdat header)
        // This will be adjusted after we know the full moof size
        let moof_size = 8 + // moof box header
            8 + 8 + // mfhd
            8 + // traf box header
            8 + 8 + // tfhd
            8 + 12 + // tfdt
            8 + 20; // trun (this box)
        content.put_u32((moof_size + 8) as u32); // data offset (moof + mdat header)

        content.put_u32(duration); // sample duration
        content.put_u32(frame.data.len() as u32); // sample size

        // Sample flags
        if frame.is_keyframe {
            content.put_u32(0x02000000); // depends on nothing (keyframe)
        } else {
            content.put_u32(0x01010000); // depends on I-frame
        }

        Self::write_box(buf, b"trun", &content);
    }

    fn write_mdat(&self, buf: &mut BytesMut, data: &[u8]) {
        let size = 8 + data.len() as u32;
        buf.put_u32(size);
        buf.put_slice(b"mdat");
        buf.put_slice(data);
    }
}
