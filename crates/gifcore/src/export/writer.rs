//! GIF89a container and LZW coder: port of gif-h's `GifBegin`,
//! `GifWriteLzwImage` and `GifEnd`, so output bytes match v2.

use std::io::{self, Write};

use super::palette::{Palette, NUM_COLORS, TRANSPARENT_INDEX};

const MIN_CODE_SIZE: u32 = 8;
const CLEAR_CODE: u32 = 1 << MIN_CODE_SIZE;
const MAX_CODE: u32 = 4095;

/// Writes the header: logical screen, a 2-entry dummy global palette and the
/// NETSCAPE2.0 block for infinite looping.
pub fn write_header(out: &mut impl Write, width: u16, height: u16) -> io::Result<()> {
    out.write_all(b"GIF89a")?;
    out.write_all(&width.to_le_bytes())?;
    out.write_all(&height.to_le_bytes())?;
    // Unsorted global colour table of 2 entries, background 0, square pixels,
    // then two black entries.
    out.write_all(&[0xf0, 0, 0, 0, 0, 0, 0, 0, 0])?;
    out.write_all(&[0x21, 0xff, 11])?;
    out.write_all(b"NETSCAPE2.0")?;
    // Sub-block 1, loop count 0 (forever), terminator.
    out.write_all(&[3, 1, 0, 0, 0])
}

/// Writes one full-canvas frame: graphic control extension (disposal "leave
/// in place", transparent index 0), image descriptor, local palette, LZW data.
pub fn write_frame(
    out: &mut impl Write,
    width: u16,
    height: u16,
    indices: &[u8],
    pal: &Palette,
    delay_cs: u16,
) -> io::Result<()> {
    let [d0, d1] = delay_cs.to_le_bytes();
    out.write_all(&[0x21, 0xf9, 0x04, 0x05, d0, d1, TRANSPARENT_INDEX, 0])?;
    out.write_all(&[0x2c, 0, 0, 0, 0])?;
    out.write_all(&width.to_le_bytes())?;
    out.write_all(&height.to_le_bytes())?;
    out.write_all(&[0x80 + 7])?; // local colour table, 256 entries
    out.write_all(&[0, 0, 0])?; // entry 0: transparency
    for c in &pal.colors[1..NUM_COLORS] {
        out.write_all(c)?;
    }
    out.write_all(&[MIN_CODE_SIZE as u8])?;
    write_lzw(out, indices)?;
    out.write_all(&[0]) // image block terminator
}

/// Writes the trailer.
pub fn write_trailer(out: &mut impl Write) -> io::Result<()> {
    out.write_all(&[0x3b])
}

/// LSB-first bit packer that emits 255-byte data sub-blocks.
struct BitWriter {
    chunk: Vec<u8>,
    byte: u8,
    bits: u8,
}

impl BitWriter {
    fn bit(&mut self, bit: u32) {
        self.byte |= ((bit & 1) as u8) << self.bits;
        self.bits += 1;
        if self.bits > 7 {
            self.chunk.push(self.byte);
            self.bits = 0;
            self.byte = 0;
        }
    }

    fn flush(&mut self, out: &mut impl Write) -> io::Result<()> {
        out.write_all(&[self.chunk.len() as u8])?;
        out.write_all(&self.chunk)?;
        self.chunk.clear();
        self.bits = 0;
        self.byte = 0;
        Ok(())
    }

    fn code(&mut self, out: &mut impl Write, mut code: u32, len: u32) -> io::Result<()> {
        for _ in 0..len {
            self.bit(code);
            code >>= 1;
            if self.chunk.len() == 255 {
                self.flush(out)?;
            }
        }
        Ok(())
    }
}

/// gif-h's LZW: a 256-ary dictionary tree, cleared when it reaches 4095 codes.
fn write_lzw(out: &mut impl Write, indices: &[u8]) -> io::Result<()> {
    let mut tree = vec![[0u16; 256]; 4096];
    let mut code_size = MIN_CODE_SIZE + 1;
    let mut max_code = CLEAR_CODE + 1;
    let mut bw = BitWriter {
        chunk: Vec::with_capacity(256),
        byte: 0,
        bits: 0,
    };

    bw.code(out, CLEAR_CODE, code_size)?;
    let mut cur: Option<usize> = None;
    for &v in indices {
        let v = usize::from(v);
        cur = Some(match cur {
            None => v,
            Some(c) if tree[c][v] != 0 => usize::from(tree[c][v]),
            Some(c) => {
                bw.code(out, c as u32, code_size)?;
                max_code += 1;
                tree[c][v] = max_code as u16;
                if max_code >= (1 << code_size) {
                    code_size += 1;
                }
                if max_code == MAX_CODE {
                    bw.code(out, CLEAR_CODE, code_size)?;
                    tree.iter_mut().for_each(|node| *node = [0; 256]);
                    code_size = MIN_CODE_SIZE + 1;
                    max_code = CLEAR_CODE + 1;
                }
                v
            }
        });
    }
    // gif-h writes code -1 (all ones) for an empty image; frames are never empty.
    let last = cur.map_or(u32::MAX, |c| c as u32);
    bw.code(out, last, code_size)?;
    bw.code(out, CLEAR_CODE, code_size)?;
    bw.code(out, CLEAR_CODE + 1, MIN_CODE_SIZE + 1)?;
    while bw.bits != 0 {
        bw.bit(0);
    }
    if !bw.chunk.is_empty() {
        bw.flush(out)?;
    }
    Ok(())
}
