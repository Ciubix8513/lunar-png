use crate::Error;

#[allow(non_camel_case_types)]
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
    //Textual information
    iTXt,
    tEXt,
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

pub struct Chunk {
    chunk_type: ChunkType,
    length: u32,
    data: Vec<u8>,
    crc: [u8; 4],
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

pub fn parse_chunk(stream: &mut impl Iterator<Item = u8>) -> Chunk {
    let length = u32::from_ne_bytes(read_n_const(stream));
    let chunk_type = get_chunk_type(read_n_const(stream)).unwrap();
    let data = read_n(stream, length);
    let crc = read_n_const(stream);

    Chunk {
        length,
        chunk_type,
        data,
        crc,
    }
}
