use crate::{Error, ImageType};

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq)]
pub enum ChunkType {
    ///Image header
    IHDR,
    ///Palette table
    PLTE,
    ///Image data chunks
    IDAT,
    ///Image trailer(last chunk of a png datastream)
    IEND,
    ///Transparency information
    tRNS,
    //Color space information
    cHRM,
    gAMA,
    iCCP,
    sBIT,
    sRGB,
    cICP,
    mDCv,
    ///International textual data
    iTXt,
    ///Textual data
    tEXt,
    ///Compressed textual data
    zTXt,
    //Misc information
    bKGD,
    hIST,
    pHYs,
    sPLT,
    eXIf,
    ///Time information
    tIME,
    //Animation information
    acTL,
    fcTL,
    fdAT,
}

#[derive(Debug, Clone, Copy)]
pub enum ColorType {
    Greyscale,
    Truecolor,
    IndexedColor,
    GreyscaleAlpha,
    TruecolorAlpha,
}

pub fn to_color_type(t: u8) -> ColorType {
    match t {
        0 => ColorType::Greyscale,
        2 => ColorType::Truecolor,
        3 => ColorType::IndexedColor,
        4 => ColorType::GreyscaleAlpha,
        6 => ColorType::TruecolorAlpha,
        _ => unreachable!(),
    }
}

pub fn validate_bit_depth(t: ColorType, depth: u8) -> bool {
    let valid = match t {
        ColorType::Greyscale => vec![1, 2, 4, 8, 16],
        ColorType::Truecolor => vec![8, 16],
        ColorType::IndexedColor => vec![1, 2, 4, 8],
        ColorType::GreyscaleAlpha => vec![8, 16],
        ColorType::TruecolorAlpha => vec![8, 16],
    };

    for i in valid {
        if i == depth {
            return true;
        }
    }

    false
}

pub struct TrnsPallete {
    inner: Vec<u8>,
}

impl TrnsPallete {
    pub fn empty() -> Self {
        Self { inner: Vec::new() }
    }
    pub fn new(data: Vec<u8>) -> Self {
        Self { inner: data }
    }

    pub fn get(&self, index: u8) -> u8 {
        if index >= self.inner.len() as u8 {
            255
        } else {
            self.inner[index as usize]
        }
    }
}

pub struct Pallete {
    inner: Vec<u8>,
}

impl Pallete {
    pub fn empty() -> Self {
        Self { inner: Vec::new() }
    }
    pub fn new(data: Vec<u8>) -> Self {
        Self { inner: data }
    }

    pub fn get(&self, index: u8) -> &[u8] {
        let index = index as usize * 3;

        &self.inner[index..index + 3]
    }
}

pub struct Chunk {
    pub chunk_type: ChunkType,
    pub length: u32,
    pub data: Vec<u8>,
}

pub fn get_chunk_type(data: [u8; 4]) -> Result<ChunkType, Error> {
    let string = if let Ok(d) = String::from_utf8(data.to_vec()) {
        d
    } else {
        return Err(Error::InvalidChunkType);
    };

    match string.as_str() {
        "IHDR" => Ok(ChunkType::IHDR),
        "PLTE" => Ok(ChunkType::PLTE),
        "IDAT" => Ok(ChunkType::IDAT),
        "IEND" => Ok(ChunkType::IEND),
        "tRNS" => Ok(ChunkType::tRNS),
        "cHRM" => Ok(ChunkType::cHRM),
        "gAMA" => Ok(ChunkType::gAMA),
        "iCCP" => Ok(ChunkType::iCCP),
        "sBIT" => Ok(ChunkType::sBIT),
        "sRGB" => Ok(ChunkType::sRGB),
        "cICP" => Ok(ChunkType::cICP),
        "mDCv" => Ok(ChunkType::mDCv),
        "iTXt" => Ok(ChunkType::iTXt),
        "tEXt" => Ok(ChunkType::tEXt),
        "zTXt" => Ok(ChunkType::zTXt),
        "bKGD" => Ok(ChunkType::bKGD),
        "hIST" => Ok(ChunkType::hIST),
        "pHYs" => Ok(ChunkType::pHYs),
        "sPLT" => Ok(ChunkType::sPLT),
        "eXIf" => Ok(ChunkType::eXIf),
        "tIME" => Ok(ChunkType::tIME),
        "acTL" => Ok(ChunkType::acTL),
        "fcTL" => Ok(ChunkType::fcTL),
        "fdAT" => Ok(ChunkType::fdAT),
        _ => Err(Error::InvalidChunkType),
    }
}

pub fn read_n_const<T: Default + Copy, const N: usize>(
    stream: &mut impl Iterator<Item = T>,
) -> [T; N] {
    let mut output = [T::default(); N];

    for i in 0..N {
        output[i] = stream.next().unwrap();
    }

    output
}

pub fn read_n<T: Default + Copy>(stream: &mut impl Iterator<Item = T>, n: u32) -> Vec<T> {
    let mut o = Vec::new();
    for _ in 0..n {
        o.push(stream.next().unwrap());
    }
    o
}

//TODO error handling
pub fn parse_chunk(stream: &mut impl Iterator<Item = u8>) -> Chunk {
    let length = u32::from_be_bytes(read_n_const(stream));
    //Read type + data
    let mut data = read_n(stream, length + 4);

    let computed_crc = u32::to_be_bytes(compute_crc(&data));
    let crc = read_n_const(stream);

    if computed_crc != crc {
        panic!("Incorect CRC");
    }

    let mut chunk_type = [0; 4];

    for i in 0..4 {
        chunk_type[i] = data.remove(0);
    }

    let chunk_type = get_chunk_type(chunk_type).unwrap();

    Chunk {
        length,
        chunk_type,
        data,
    }
}

pub fn extract_string(data: &mut impl Iterator<Item = u8>) -> String {
    let str = data.take_while(|i| *i != 0).collect::<Vec<_>>();

    String::from_utf8(str).unwrap()
}

///Statically computed table for fast CRC computation
const fn compute_crc_table() -> [u32; 256] {
    let mut n = 0u32;
    let mut k;

    let mut output = [0u32; 256];

    while n < 256 {
        let mut c = n;

        k = 0;
        while k < 8 {
            if c & 1 == 1 {
                c = 0xedb88320 ^ (c >> 1);
            } else {
                c = c >> 1;
            }
            k += 1;
        }
        output[n as usize] = c;
        n += 1;
    }

    output
}

//Copied from sample CRC implementation https://www.w3.org/TR/png-3/#samplecrc
///Calculates 32bit CRC

pub fn compute_crc(data: &[u8]) -> u32 {
    let mut c = u32::MAX;

    let table = compute_crc_table();
    for i in data {
        let i = *i as u32;
        c = table[((c ^ i) & 0xff) as usize] ^ (c >> 8)
    }

    c ^ u32::MAX
}

///Merges 2 u8 to create a u16
pub fn to_u16(a: u8, b: u8) -> u16 {
    (a as u16) | ((b as u16) << 8)
}

pub struct Filtered {
    pub data: Vec<u8>,
    pub color_type: ColorType,
    pub scanline_len: u32,
}

impl Filtered {
    pub fn set(&mut self, index: usize, val: u8) {
        self.data[index] = val;
    }
    //Bytes
    //  |c|b|
    //  |a|x|
    //  x = current
    pub fn get_a(&self, index: usize) -> u8 {
        let offset = match self.color_type {
            ColorType::IndexedColor | ColorType::Greyscale => 1,
            ColorType::Truecolor => 3,
            ColorType::GreyscaleAlpha => 2,
            ColorType::TruecolorAlpha => 4,
        };

        if index as u32 % self.scanline_len <= offset as u32 {
            return 0;
        }

        let final_index = index.checked_sub(offset).unwrap_or(0);

        if final_index as u32 % self.scanline_len == 0 {
            return 0;
        }

        self.data.get(final_index as usize).copied().unwrap_or(0)
    }

    pub fn get_b(&self, index: usize) -> u8 {
        self.data
            .get(index - self.scanline_len as usize)
            .copied()
            .unwrap_or(0)
    }

    pub fn get_c(&self, index: usize) -> u8 {
        self.get_a(index - self.scanline_len as usize)
    }
}
