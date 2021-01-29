//! # bitwise-io
//!
//! A simple wrapper around the `BufRead` and `Write` Trait for bitwise IO
//!
use std::io::{BufRead, Write, ErrorKind};
use std::fmt::{Display, Formatter};
use std::collections::VecDeque;


/// Bit representation
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Bit {
    Zero = 0,
    One = 1,
}

/// Reader for bitwise reading from `BufRead`
#[derive(Debug)]
pub struct BitReader<R: BufRead> {
    inner: R,
    buf: Box<[u8]>,
    pos: usize,
    init_read: bool,
}

const DEFAULT_BUF_SIZE: usize = 1024;

/// Writer to bitwise writing to `Write`
#[derive(Debug)]
pub struct BitWriter<W: Write> {
    inner: W,
    buf: VecDeque<Bit>,
    pub pad_zero: bool,
}

/**************************************************************************************************
                        BitReader - Implementations
 *************************************************************************************************/
impl<R: BufRead> BitReader<R> {
    /// Creates a new BitReader from a BufRead
    /// Buffer is not filled on create
    pub fn new(mut inner: R) -> std::io::Result<BitReader<R>> {
        let buf = inner.fill_buf()?.to_vec().into_boxed_slice();

        Ok(BitReader { inner, buf, pos: 0 , init_read: false})
    }

    /// Read a single Bit from BufRead
    pub fn read(&mut self) -> std::io::Result<Bit> {
        if self.init_read == false {
            reader_fill_buf(self)?;
        }

        if self.is_empty() {
            Err(std::io::Error::new(ErrorKind::Other, "End of File"))
        }

        let mut byte_offset = self.pos / 8;
        let mut bit_offset = self.pos % 8;

        let byte = self.buf[byte_offset];

        let mask = 1 << (7 - bit_offset);

        let bit = Bit::from(byte & mask);

        bit_offset += 1;
        if bit_offset > 7 {
            let byte_o = reader_update(self, byte_offset + 1)?;

            byte_offset = byte_o;
            bit_offset = 0;
        }

        self.pos = byte_offset * 8 + bit_offset;

        Ok(bit)
    }

    /// Try Reading n Bits from BufRead
    pub fn read_multi(&mut self, n: usize) -> std::io::Result<Vec<Bit>> {
        let mut output = Vec::with_capacity(n);

        for _ in 0..n {
            output.push(self.read()?);
        }

        Ok(output)
    }

    /// Returns true if the Buffer is empty
    /// Always true after newly created
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Returns the length of the internal buffer
    pub fn buf_len(&self) -> usize {
        self.buf.len()
    }
}

/// Consume the Buffer and read from file if byte_offset is buffer_length
fn reader_update<R: BufRead>(reader: &mut BitReader<R>, byte_offset: usize) -> std::io::Result<usize> {
    let buf_len = reader.buf.len();

    if byte_offset >= buf_len {
        reader_fill_buf(reader)?;

        reader.pos = 0;

        Ok(0)
    } else {
        Ok(byte_offset)
    }
}

/// Consume buf.len() and fill buf
fn reader_fill_buf<R: BufRead>(reader: &mut BitReader<R>) -> std::io::Result<()> {
    reader.inner.consume(reader.buf.len());

    let buf = reader.inner.fill_buf()?;

    reader.buf = buf.to_vec().into_boxed_slice();

    Ok(())
}


/**************************************************************************************************
                        BitReader - Implementations
 *************************************************************************************************/
impl<W: Write> BitWriter<W> {
    /// Create a new BitWriter from a Write Trait with default capacity of 1024 Bytes
    pub fn new(inner: W, pad_zero: bool) -> Self {
        BitWriter::with_capacity(DEFAULT_BUF_SIZE, inner, pad_zero)
    }

    /// Create a new BitWriter with a capacity (in Bytes)
    pub fn with_capacity(capacity: usize, inner: W, pad_zero: bool) -> Self {
        BitWriter {
            inner,
            buf: VecDeque::with_capacity(capacity * 8),
            pad_zero,
        }
    }

    /// Writes a single Bit into the internal Buffer
    /// If internal buffer is full -> Call internal write
    pub fn write(&mut self, bit: Bit) -> std::io::Result<()> {
        if self.buf.len() == DEFAULT_BUF_SIZE {
            match self.write_buf() {
                Ok(_) => {
                    self.buf.push_back(bit);
                    Ok(())
                }
                Err(err) => Err(err)
            }
        } else {
            self.buf.push_back(bit);
            Ok(())
        }
    }

    /// Writes a vector of Bits into the internal Buffer
    /// If internal buffer is full -> Call internal write
    pub fn write_bits(&mut self, bits: &Vec<Bit>) -> std::io::Result<()> {
        for bit in bits {
            self.write(bit.clone())?
        }
        Ok(())
    }

    /// Write the internal Buffer and Pad with Zero? If needed
    pub fn write_buf(&mut self) -> std::io::Result<()> {
        writer_pad_buf(self);

        let bytes = writer_buf_to_bytes(self);
        match self.inner.write(&*bytes) {
            Ok(_) => {
                self.inner.flush()
            }
            Err(err) => Err(err)
        }
    }

    /// Removes excess bits that do not form a byte
    pub fn discard_non_byte(&mut self) {
        while self.buf.len() % 8 != 0 {
            let _ = self.buf.pop_back();
        }
    }

    /// Returns true if the Buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Returns the length of the internal buffer
    pub fn buf_len(&self) -> usize {
        self.buf.len()
    }
}

impl<W: Write> Drop for BitWriter<W> {
    fn drop(&mut self) {
        let _ = self.write_buf();
    }
}

/// Removes all complete bytes from the Buffer and returns them in a Vector
fn writer_buf_to_bytes<W: Write>(writer: &mut BitWriter<W>) -> Vec<u8> {
    let mut bytes = Vec::new();

    while writer.buf.len() >= 8 {
        let mut byte = 0;
        for i in 0..8 {
            byte |= writer.buf.pop_front().unwrap() as u8;

            if i < 7 {
                byte = byte << 1;
            }
        }
        bytes.push(byte);
    }

    bytes
}

/// Pad Byte
fn writer_pad_buf<W: Write>(writer: &mut BitWriter<W>) {
    let pad_bit = match writer.pad_zero {
        true => Bit::Zero,
        false => Bit::One,
    };

    for _ in 0..(writer.buf.len() % 8) {
        writer.buf.push_back(pad_bit);
    }
}

/**************************************************************************************************
                        Bit - Implementations
 *************************************************************************************************/
impl Display for Bit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Bit::Zero => write!(f, "0"),
            Bit::One => write!(f, "1"),
        }
    }
}

impl From<u8> for Bit {
    fn from(value: u8) -> Self {
        if value > 0 {
            return Bit::One;
        } else {
            return Bit::Zero;
        }
    }
}

impl From<u16> for Bit {
    fn from(value: u16) -> Self {
        if value > 0 {
            return Bit::One;
        } else {
            return Bit::Zero;
        }
    }
}

impl From<u32> for Bit {
    fn from(value: u32) -> Self {
        if value > 0 {
            return Bit::One;
        } else {
            return Bit::Zero;
        }
    }
}

impl From<u64> for Bit {
    fn from(value: u64) -> Self {
        if value > 0 {
            return Bit::One;
        } else {
            return Bit::Zero;
        }
    }
}

impl From<i8> for Bit {
    fn from(value: i8) -> Self {
        if value > 0 {
            return Bit::One;
        } else {
            return Bit::Zero;
        }
    }
}

impl From<i16> for Bit {
    fn from(value: i16) -> Self {
        if value > 0 {
            return Bit::One;
        } else {
            return Bit::Zero;
        }
    }
}

impl From<i32> for Bit {
    fn from(value: i32) -> Self {
        if value > 0 {
            return Bit::One;
        } else {
            return Bit::Zero;
        }
    }
}

impl From<i64> for Bit {
    fn from(value: i64) -> Self {
        if value > 0 {
            return Bit::One;
        } else {
            return Bit::Zero;
        }
    }
}

impl From<usize> for Bit {
    fn from(value: usize) -> Self {
        if value > 0 {
            return Bit::One;
        } else {
            return Bit::Zero;
        }
    }
}

impl From<bool> for Bit {
    fn from(value: bool) -> Self {
        match value {
            false => Bit::Zero,
            true => Bit::One,
        }
    }
}
