//! In-memory image buffer types for gifcore.

/// 4-channel, 8-bit RGBA (no padding).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rgba8 {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// 4-channel, 8-bit BGRA (no padding).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bgra8 {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// 3-channel, 8-bit BGR (no padding).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bgr8 {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Rgba8 {
    /// Zero-filled image.
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width as usize) * (height as usize) * 4;
        Rgba8 {
            width,
            height,
            data: vec![0u8; size],
        }
    }

    /// Wrap raw bytes; returns `None` if length doesn't match w × h × 4.
    pub fn from_raw(width: u32, height: u32, data: Vec<u8>) -> Option<Self> {
        let expected = (width as usize) * (height as usize) * 4;
        if data.len() == expected {
            Some(Rgba8 {
                width,
                height,
                data,
            })
        } else {
            None
        }
    }

    /// Return a slice of the pixel at `(x, y)`.
    pub fn pixel(&self, x: u32, y: u32) -> &[u8] {
        let offset = (y as usize) * (self.width as usize) * 4 + (x as usize) * 4;
        &self.data[offset..offset + 4]
    }

    /// Return a mutable slice of the pixel at `(x, y)`.
    pub fn pixel_mut(&mut self, x: u32, y: u32) -> &mut [u8] {
        let offset = (y as usize) * (self.width as usize) * 4 + (x as usize) * 4;
        &mut self.data[offset..offset + 4]
    }

    /// Number of channels per pixel.
    pub const CHANNELS: usize = 4;
}

impl Bgra8 {
    /// Zero-filled image.
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width as usize) * (height as usize) * 4;
        Bgra8 {
            width,
            height,
            data: vec![0u8; size],
        }
    }

    /// Wrap raw bytes; returns `None` if length doesn't match w × h × 4.
    pub fn from_raw(width: u32, height: u32, data: Vec<u8>) -> Option<Self> {
        let expected = (width as usize) * (height as usize) * 4;
        if data.len() == expected {
            Some(Bgra8 {
                width,
                height,
                data,
            })
        } else {
            None
        }
    }

    /// Return a slice of the pixel at `(x, y)`.
    pub fn pixel(&self, x: u32, y: u32) -> &[u8] {
        let offset = (y as usize) * (self.width as usize) * 4 + (x as usize) * 4;
        &self.data[offset..offset + 4]
    }

    /// Return a mutable slice of the pixel at `(x, y)`.
    pub fn pixel_mut(&mut self, x: u32, y: u32) -> &mut [u8] {
        let offset = (y as usize) * (self.width as usize) * 4 + (x as usize) * 4;
        &mut self.data[offset..offset + 4]
    }

    /// Number of channels per pixel.
    pub const CHANNELS: usize = 4;
}

impl Bgr8 {
    /// Zero-filled image.
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width as usize) * (height as usize) * 3;
        Bgr8 {
            width,
            height,
            data: vec![0u8; size],
        }
    }

    /// Wrap raw bytes; returns `None` if length doesn't match w × h × 3.
    pub fn from_raw(width: u32, height: u32, data: Vec<u8>) -> Option<Self> {
        let expected = (width as usize) * (height as usize) * 3;
        if data.len() == expected {
            Some(Bgr8 {
                width,
                height,
                data,
            })
        } else {
            None
        }
    }

    /// Return a slice of the pixel at `(x, y)`.
    pub fn pixel(&self, x: u32, y: u32) -> &[u8] {
        let offset = (y as usize) * (self.width as usize) * 3 + (x as usize) * 3;
        &self.data[offset..offset + 3]
    }

    /// Return a mutable slice of the pixel at `(x, y)`.
    pub fn pixel_mut(&mut self, x: u32, y: u32) -> &mut [u8] {
        let offset = (y as usize) * (self.width as usize) * 3 + (x as usize) * 3;
        &mut self.data[offset..offset + 3]
    }

    /// Number of channels per pixel.
    pub const CHANNELS: usize = 3;
}

/// Convert RGBA → BGRA (swap R↔B, keep A).
pub fn rgba_to_bgra(src: &Rgba8) -> Bgra8 {
    let mut out = Bgra8::new(src.width, src.height);
    for y in 0..src.height {
        for x in 0..src.width {
            let p = src.pixel(x, y);
            out.pixel_mut(x, y)[0] = p[2]; // out R = in R
            out.pixel_mut(x, y)[1] = p[1]; // G stays
            out.pixel_mut(x, y)[2] = p[0]; // out B = in B
            out.pixel_mut(x, y)[3] = p[3]; // A stays
        }
    }
    out
}

/// Convert BGRA → BGR (drop alpha channel).
pub fn bgra_to_bgr(src: &Bgra8) -> Bgr8 {
    let mut out = Bgr8::new(src.width, src.height);
    for y in 0..src.height {
        for x in 0..src.width {
            let p = src.pixel(x, y);
            out.pixel_mut(x, y)[0] = p[0]; // B stays
            out.pixel_mut(x, y)[1] = p[1]; // G stays
            out.pixel_mut(x, y)[2] = p[2]; // R stays
        }
    }
    out
}

/// Convert BGR → RGBA (add alpha = 255).
pub fn bgr_to_rgba(src: &Bgr8) -> Rgba8 {
    let mut out = Rgba8::new(src.width, src.height);
    for y in 0..src.height {
        for x in 0..src.width {
            let p = src.pixel(x, y);
            out.pixel_mut(x, y)[0] = p[2]; // out B = in B
            out.pixel_mut(x, y)[1] = p[1]; // G stays
            out.pixel_mut(x, y)[2] = p[0]; // out R = in R
            out.pixel_mut(x, y)[3] = 255; // A = opaque
        }
    }
    out
}
