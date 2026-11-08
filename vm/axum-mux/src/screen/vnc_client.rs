//! Minimal RFB (VNC) client for frame capture.
//!
//! Implements just enough of the protocol to connect to a QEMU VNC server,
//! read the initial framebuffer, and optionally subscribe to incremental
//! updates. No encryption, no advanced encodings — raw + tight only.
//!
//! This is intentionally NOT a full VNC client library. It's a capture
//! backend for screen sharing features (thumbnails, PipeWire source).

use anyhow::{Context, Result, bail};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Pixel format descriptor from the server.
#[derive(Debug, Clone)]
pub struct PixelFormat {
    pub depth: u8,
    pub true_color: bool,
    pub red_shift: u8,
    pub green_shift: u8,
    pub blue_shift: u8,
    pub red_max: u16,
    pub green_max: u16,
    pub blue_max: u16,
}

/// A captured framebuffer (RGBA8).
#[derive(Debug, Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    /// Raw RGBA8 pixels, row-major.
    pub pixels: Vec<u8>,
}

impl Frame {
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.pixels.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.pixels.is_empty()
    }

    /// Downscale to a max dimension (preserving aspect ratio).
    /// Uses simple nearest-neighbor sampling — good enough for thumbnails.
    pub fn downscale(&self, max_dim: u32) -> Frame {
        if self.width <= max_dim && self.height <= max_dim {
            return self.clone();
        }

        let scale = (max_dim as f64) / (self.width.max(self.height) as f64);
        let new_w = (self.width as f64 * scale) as u32;
        let new_h = (self.height as f64 * scale) as u32;

        let mut out = Frame {
            width: new_w,
            height: new_h,
            pixels: vec![0u8; (new_w * new_h * 4) as usize],
        };

        for y in 0..new_h {
            for x in 0..new_w {
                let src_x = ((x as f64 + 0.5) / scale).min(self.width as f64 - 1.0) as u32;
                let src_y = ((y as f64 + 0.5) / scale).min(self.height as f64 - 1.0) as u32;
                let src_idx = ((src_y * self.width + src_x) * 4) as usize;
                let dst_idx = ((y * new_w + x) * 4) as usize;
                out.pixels[dst_idx..dst_idx + 4]
                    .copy_from_slice(&self.pixels[src_idx..src_idx + 4]);
            }
        }

        out
    }
}

/// Minimal VNC client for frame capture.
pub struct VncCapture {
    stream: TcpStream,
    pixel_format: PixelFormat,
    width: u32,
    height: u32,
}

impl VncCapture {
    /// Connect to a VNC server and complete the handshake.
    /// `target` is "host:port" (e.g., "127.0.0.1:5930").
    pub async fn connect(target: &str) -> Result<Self> {
        let mut stream = TcpStream::connect(target)
            .await
            .with_context(|| format!("connect to VNC target {}", target))?;

        // 1. Exchange version strings.
        let mut buf = [0u8; 12];
        stream.read_exact(&mut buf).await?;
        let server_version = String::from_utf8_lossy(&buf);
        if !server_version.starts_with("RFB ") {
            bail!("bad RFB version from server: {}", server_version);
        }

        // We support up to 3.8. Pick the best common version.
        let our_version = "RFB 003.008\n";
        stream.write_all(our_version.as_bytes()).await?;

        // 2. Security types.
        let n_types = stream.read_u8().await?;
        if n_types == 0 {
            // Server sent a reason string instead.
            let len = stream.read_u32().await?;
            let mut reason = vec![0u8; len as usize];
            stream.read_exact(&mut reason).await?;
            bail!(
                "no security types offered: {}",
                String::from_utf8_lossy(&reason)
            );
        }

        let mut types = vec![0u8; n_types as usize];
        stream.read_exact(&mut types).await?;

        // Prefer None (1), then VNC auth (2).
        let selected = if types.contains(&1) {
            1
        } else if types.contains(&2) {
            2
        } else {
            bail!("unsupported security types: {:?}", types);
        };

        stream.write_u8(selected).await?;

        // 3. Authentication.
        if selected == 2 {
            // VNC auth: server sends 16-byte challenge.
            let mut challenge = [0u8; 16];
            stream.read_exact(&mut challenge).await?;
            // For QEMU with no password, send zeros.
            // (Full DES-based auth would go here if passwords were in scope.)
            stream.write_all(&[0u8; 16]).await?;
        }

        // SecurityResult: u32 status (0 = OK).
        let result = stream.read_u32().await?;
        if result != 0 {
            bail!("VNC auth failed, result={}", result);
        }

        // 4. ClientInit: shared=1 (don't disconnect other clients).
        stream.write_all(&[0u8, 0u8, 0u8, 1]).await?;

        // 5. ServerInit.
        let width = stream.read_u32().await?;
        let height = stream.read_u32().await?;
        let pf = read_pixel_format(&mut stream).await?;
        let name_len = stream.read_u32().await?;
        let mut name_buf = vec![0u8; name_len as usize];
        stream.read_exact(&mut name_buf).await?;

        Ok(Self {
            stream,
            width,
            height,
            pixel_format: pf,
        })
    }

    /// Request and read a full framebuffer update.
    pub async fn capture_frame(&mut self) -> Result<Frame> {
        // FramebufferUpdateRequest: incremental=0 (full), x, y, w, h.
        self.stream
            .write_all(&[
                3u8,       // msg type
                0,         // padding
                0,         // incremental = false
                0, 0,      // x
                0, 0,      // y
                (self.width >> 8) as u8, self.width as u8,
                (self.height >> 8) as u8, self.height as u8,
            ])
            .await?;

        // Read FramebufferUpdate message.
        let msg_type = self.stream.read_u8().await?;
        if msg_type != 0 {
            bail!("expected FramebufferUpdate (0), got {}", msg_type);
        }
        self.stream.read_u8().await?; // padding
        let n_rects = self.stream.read_u16().await?;

        let mut frame = Frame {
            width: self.width,
            height: self.height,
            pixels: vec![0u8; (self.width * self.height * 4) as usize],
        };

        for _ in 0..n_rects {
            let x = self.stream.read_u16().await? as u32;
            let y = self.stream.read_u16().await? as u32;
            let w = self.stream.read_u16().await? as u32;
            let h = self.stream.read_u16().await? as u32;
            let encoding = self.stream.read_i32().await?;

            match encoding {
                0 => {
                    // Raw encoding.
                    let bytes_per_pixel = if self.pixel_format.depth >= 24 {
                        4
                    } else {
                        2
                    };
                    let data_len = (w * h * bytes_per_pixel as u32) as usize;
                    let mut data = vec![0u8; data_len];
                    self.stream.read_exact(&mut data).await?;
                    decode_raw(&data, &self.pixel_format, x, y, w, h, &mut frame);
                }
                1 => {
                    // CoRRE — skip for now (QEMU rarely uses it by default).
                    bail!("CoRRE encoding not supported");
                }
                _ if (0x8000..=0xFFFF).contains(&encoding) => {
                    // Pseudo-encodings (cursor, desktop size, etc.) — skip data.
                    let len = read_pseudo_encoding_data(&mut self.stream, encoding).await?;
                    if len > 0 {
                        let mut discard = vec![0u8; (len as usize).min(4096)];
                        let mut remaining = len;
                        while remaining > 0 {
                            let to_read = (remaining as usize).min(discard.len());
                            let n = self.stream.read(&mut discard[..to_read]).await?;
                            if n == 0 {
                                break;
                            }
                            remaining -= n as u64;
                        }
                    }
                }
                _ => {
                    bail!("unsupported VNC encoding: {}", encoding);
                }
            }
        }

        Ok(frame)
    }
}

async fn read_pixel_format(stream: &mut TcpStream) -> Result<PixelFormat> {
    let depth = stream.read_u8().await?;
    let _bpp = stream.read_u8().await?;
    let _big_endian = stream.read_u8().await?;
    let true_color = stream.read_u8().await?;
    let red_max = stream.read_u16().await?;
    let green_max = stream.read_u16().await?;
    let blue_max = stream.read_u16().await?;
    let red_shift = stream.read_u8().await?;
    let green_shift = stream.read_u8().await?;
    let blue_shift = stream.read_u8().await?;
    let mut _padding = [0u8; 3];
    stream.read_exact(&mut _padding).await?;

    Ok(PixelFormat {
        depth,
        true_color: true_color != 0,
        red_shift,
        green_shift,
        blue_shift,
        red_max,
        green_max,
        blue_max,
    })
}

/// Read the variable-length data for pseudo-encodings and return how many
/// bytes to skip. For most pseudo-encodings we just need to drain the stream.
async fn read_pseudo_encoding_data(stream: &mut TcpStream, encoding: i32) -> Result<u64> {
    match encoding {
        // DesktopName: u32 length + name bytes.
        1 => {
            let len = stream.read_u32().await? as u64;
            Ok(len as u64)
        }
        // Cursor / PointerPos etc.: fixed small headers, drain conservatively.
        _ => {
            // Most pseudo-encodings have a small header we've already read
            // (x, y, w, h from the rect). Nothing more to skip for now.
            Ok(0)
        }
    }
}

/// Decode raw-encoded pixels into the RGBA8 frame buffer.
fn decode_raw(
    data: &[u8],
    pf: &PixelFormat,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    frame: &mut Frame,
) {
    if !pf.true_color {
        // Pseudo-color (palette) not supported — fill with black.
        return;
    }

    let bpp = if pf.depth >= 24 { 4 } else { 2 };
    let max_val = 65535u32;

    for row in 0..h {
        for col in 0..w {
            let src_idx = ((row * w + col) * bpp as u32) as usize;
            if src_idx + bpp > data.len() {
                return;
            }

            let pixel_val: u32 = if bpp == 4 {
                u32::from_be_bytes([
                    data[src_idx],
                    data[src_idx + 1],
                    data[src_idx + 2],
                    data[src_idx + 3],
                ])
            } else {
                u32::from_be_bytes([0, 0, data[src_idx], data[src_idx + 1]])
            };

            let r = scale_channel(
                (pixel_val >> pf.red_shift) & (pf.red_max as u32),
                pf.red_max,
            );
            let g = scale_channel(
                (pixel_val >> pf.green_shift) & (pf.green_max as u32),
                pf.green_max,
            );
            let b = scale_channel(
                (pixel_val >> pf.blue_shift) & (pf.blue_max as u32),
                pf.blue_max,
            );

            let dst_idx = (((y + row) * frame.width + x + col) * 4) as usize;
            if dst_idx + 4 <= frame.pixels.len() {
                frame.pixels[dst_idx] = r as u8;
                frame.pixels[dst_idx + 1] = g as u8;
                frame.pixels[dst_idx + 2] = b as u8;
                frame.pixels[dst_idx + 3] = 255; // opaque
            }
        }
    }

    let _ = max_val;
}

fn scale_channel(value: u32, max: u16) -> f64 {
    if max == 0 {
        return 0.0;
    }
    (value as f64 / max as f64 * 255.0).min(255.0)
}
