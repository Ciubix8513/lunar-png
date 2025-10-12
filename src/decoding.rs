use crate::{
    helpers::{
        parse_chunk, read_n_const, to_color_type, to_u16, validate_bit_depth, ChunkType, ColorType,
        Filtered, Pallete, TrnsPallete,
    },
    Image, ImageType,
};
use std::io::Read;

static SIGNATURE: &[u8; 8] = &[0x89u8, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
///Errors that can occur while loading an image
pub enum Error {
    ///File has invalid signature
    InvalidSignature,
    ///A chunk in a file has invalid header
    InvalidChunkType,
    ///CRC of a chunk is incorrect k
    InvalidCrc,
    ///Other issue
    InvalidPngData(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSignature => write!(f, "invalid signature"),
            Self::InvalidChunkType => write!(f, "invalid chunk type"),
            Self::InvalidCrc => write!(f, "invalid chunk crc"),
            Self::InvalidPngData(msg) => write!(f, "invalid png data: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

enum TransparencyData {
    None,
    Greyscale(u16),
    Truecolor(u16, u16, u16),
    Indexed(TrnsPallete),
}

#[allow(clippy::missing_panics_doc, clippy::too_many_lines)]
///Parses a png image from a given stream
///
///# Errors
///
///May return an error if the data stream doesn't contain a valid png image
///
///
///# Examples
///
///```no_run
///# use std::io::Read;
///# let mut file = std::fs::File::open("...").unwrap();
///let mut data = Vec::new();
///
///file.read_to_end(&mut data);
///
///let image = lunar_png::read_png(&mut data.into_iter()).unwrap();
///```
pub fn read_png(stream: &mut impl Iterator<Item = u8>) -> Result<Image, Error> {
    if &read_n_const(stream) != SIGNATURE {
        //if signature is incorrect , return a corresponding error
        return Err(Error::InvalidSignature);
    }

    let first_chunk = parse_chunk(stream)?;

    //Return an error if the first chunk is not a header
    if first_chunk.chunk_type != ChunkType::IHDR {
        return Err(Error::InvalidPngData(
            "Invalid png file, IHDR must be the first chunk",
        ));
    }

    let mut data_iter = first_chunk.data.into_iter();
    let width = u32::from_be_bytes(read_n_const(&mut data_iter));
    let height = u32::from_be_bytes(read_n_const(&mut data_iter));

    let bit_depth = data_iter.next().unwrap();
    let color_type = to_color_type(data_iter.next().unwrap());
    let compression_method = data_iter.next().unwrap();
    let filter_method = data_iter.next().unwrap();
    let interlace_method = data_iter.next().unwrap();

    if interlace_method != 0 {
        todo!("Interlacing not yet supported");
    }

    if filter_method != 0 {
        return Err(Error::InvalidPngData("Invalid filter method"));
    }

    if !validate_bit_depth(color_type, bit_depth) {
        return Err(Error::InvalidPngData("Invalid bit depth for color type"));
    }

    if compression_method != 0 {
        return Err(Error::InvalidPngData("Invalid compression method"));
    }

    //Start the chunk reading loop
    let mut png_data = Vec::new();

    let mut pallete = Pallete::empty();
    let mut trns_data = TransparencyData::None;

    loop {
        //Get the chunk
        let chunk = parse_chunk(stream)?;

        //check if it's the last chunk
        if chunk.chunk_type == ChunkType::IEND {
            break;
        }

        match chunk.chunk_type {
            ChunkType::PLTE => {
                //Get the palette
                pallete = Pallete::new(chunk.data);
            }
            //Data
            ChunkType::IDAT => {
                png_data.extend_from_slice(&chunk.data);
            }
            //Transparency
            ChunkType::tRNS => {
                let mut data = chunk.data.into_iter();
                trns_data = match color_type {
                    ColorType::Greyscale => {
                        TransparencyData::Greyscale(u16::from_be_bytes(read_n_const(&mut data)))
                    }
                    ColorType::Truecolor => TransparencyData::Truecolor(
                        u16::from_be_bytes(read_n_const(&mut data)),
                        u16::from_be_bytes(read_n_const(&mut data)),
                        u16::from_be_bytes(read_n_const(&mut data)),
                    ),
                    ColorType::IndexedColor => {
                        TransparencyData::Indexed(TrnsPallete::new(data.collect()))
                    }
                    _ => return Err(Error::InvalidPngData("Image can not contain tRNS chunk")),
                }
            }
            _ => {}
        }
    }

    //Decompress the data
    let mut data = Vec::new();
    let mut decoder = flate2::read::ZlibDecoder::new(&png_data[..]);
    let _ = decoder.read_to_end(&mut data).unwrap();

    let scanline_len = match color_type {
        ColorType::IndexedColor | ColorType::Greyscale => bit_depth as u32 * width / 8 + 1,
        ColorType::Truecolor => bit_depth as u32 * 3 * width / 8 + 1,
        ColorType::GreyscaleAlpha => (bit_depth as u32 * 2 * width) / 8 + 1,
        ColorType::TruecolorAlpha => ((bit_depth as u32 * 4 * width) / 8) + 1,
    };

    //FILTERING!
    let mut filtered = Filtered {
        data,
        color_type,
        scanline_len,
        bit_depth,
    };

    let mut unfiltered_data = Vec::new();
    let mut filter_method = 0;

    for (index, val) in filtered.data.clone().iter().enumerate() {
        if index % scanline_len as usize == 0 {
            filter_method = *val;
            continue;
        }

        match filter_method {
            //None
            0 => {
                //Do nothing
                unfiltered_data.push(*val);
            }
            //Sub
            1 => {
                let o = ((*val as u16 + filtered.get_a(index) as u16) % 256) as u8;
                filtered.set(index, o);
                unfiltered_data.push(o);
            }
            //up
            2 => {
                let o = ((*val as u16 + filtered.get_b(index) as u16) % 256) as u8;
                filtered.set(index, o);

                unfiltered_data.push(o);
            }
            //average
            3 => {
                let a = filtered.get_a(index) as u16;
                let b = filtered.get_b(index) as u16;

                let floor = (a + b) / 2;

                let o = ((*val as u16 + floor) % 256) as u8;
                filtered.set(index, o);

                unfiltered_data.push(o);
            }
            //paeth
            4 => {
                let o = ((*val as u16 + filtered.paeth(index) as u16) % 256) as u8;
                filtered.set(index, o);

                unfiltered_data.push(o);
            }
            _ => {
                return Err(Error::InvalidPngData(
                    "Invalid filter method {filter_method}",
                ))
            }
        }
    }

    let mut img = match color_type {
        ColorType::Greyscale => {
            let new_data = match bit_depth {
                1 | 2 | 4 => {
                    let mut o = Vec::new();

                    //Iterate over N bits (N = bit_depth)
                    //Extract data i & 2^N - 1 << iter_num
                    //Normalize over 0-255 (255 / ((2 << N) - 1) * num)
                    for i in &unfiltered_data {
                        let mut d = Vec::new();
                        for index in 0..(8 / bit_depth) {
                            //Complete indexer
                            let indexer: u8 = ((1 << bit_depth) - 1) << (index * bit_depth);
                            let num = (i & indexer) >> (index * bit_depth);
                            let normalized = (255 / ((1 << bit_depth) - 1)) * num;

                            d.push(normalized);
                        }

                        d.reverse();
                        o.append(&mut d);
                    }
                    o
                }
                16 | 8 => unfiltered_data,
                _ => return Err(Error::InvalidPngData("Invalid bit depth")),
            };

            if bit_depth == 16 {
                let mut img = Image {
                    width,
                    height,
                    img_type: ImageType::R16,
                    data: new_data,
                };

                //if a transparency chunck is found, modify the image type and stick on some bytes
                //at the end of every pixel
                match trns_data {
                    TransparencyData::Greyscale(value) => {
                        img.img_type = ImageType::Rgba16;
                        img.data = img
                            .data
                            .chunks(2)
                            .flat_map(|c| {
                                let v = if to_u16(c[0], c[1]) == value { 0 } else { 0xff };
                                [c[0], c[1], v, v]
                            })
                            .collect();
                    }
                    TransparencyData::None => {}
                    _ => unreachable!(),
                }

                img
            } else {
                let mut img = Image {
                    width,
                    height,
                    img_type: ImageType::R8,
                    data: new_data,
                };

                match trns_data {
                    TransparencyData::Greyscale(value) => {
                        img.img_type = ImageType::Ra8;
                        img.data = img
                            .data
                            .iter()
                            .flat_map(|i| {
                                let v = if *i as u16 == value { 0 } else { 0xff };
                                [*i, v]
                            })
                            .collect();
                    }
                    TransparencyData::None => {}
                    _ => unreachable!(),
                }
                img
            }
        }

        ColorType::Truecolor => {
            let mut img = Image {
                width,
                height,
                img_type: if bit_depth == 8 {
                    ImageType::Rgb8
                } else {
                    ImageType::Rgb16
                },
                data: unfiltered_data,
            };
            match trns_data {
                TransparencyData::Truecolor(r, g, b) => {
                    if bit_depth == 8 {
                        img.data = img
                            .data
                            .chunks(3)
                            .flat_map(|c| {
                                let v = if c[0] as u16 == r && c[1] as u16 == g && c[2] as u16 == b
                                {
                                    0
                                } else {
                                    0xff
                                };
                                [c[0], c[1], c[2], v]
                            })
                            .collect();
                        img.img_type = ImageType::Rgba8;
                    } else {
                        img.img_type = ImageType::Rgba16;

                        img.data = img
                            .data
                            .chunks(6)
                            .flat_map(|c| {
                                let v = if to_u16(c[0], c[1]) == r
                                    && to_u16(c[2], c[3]) == g
                                    && to_u16(c[4], c[5]) == b
                                {
                                    0
                                } else {
                                    0xff
                                };
                                [c[0], c[1], c[2], c[3], c[4], c[5], v, v]
                            })
                            .collect();
                    }
                }
                TransparencyData::None => {}
                _ => unreachable!(),
            }
            img
        }

        ColorType::TruecolorAlpha => Image {
            width,
            height,
            img_type: if bit_depth == 8 {
                ImageType::Rgba8
            } else {
                ImageType::Rgba16
            },
            data: unfiltered_data,
        },
        ColorType::IndexedColor => {
            let indexes = match bit_depth {
                1 | 2 | 4 => {
                    let mut o = Vec::new();

                    //Iterate over N bits (N = bit_depth)
                    //Extract data i & 2^N - 1 << iter_num
                    for i in &unfiltered_data {
                        for index in 0..(8 / bit_depth) {
                            let indexer: u8 = ((1 << bit_depth) - 1) << (index * bit_depth);
                            let num = (i & indexer) >> (index * bit_depth);
                            //This is a terrible hack, buuut like who's gonna be using 1 bit
                            //indexed pngs in 2025!?!?
                            //
                            //or just indexed pngs in general, it's fiiiiine :3
                            if bit_depth == 1 {
                                o.push(num ^ 1);
                            } else {
                                o.push(num);
                            }
                        }
                    }
                    o
                }
                8 => unfiltered_data,
                _ => return Err(Error::InvalidPngData("Invalid bit depth")),
            };

            match trns_data {
                TransparencyData::Indexed(trns_pallete) => Image {
                    width,
                    height,
                    img_type: ImageType::Rgba8,
                    data: indexes
                        .into_iter()
                        .flat_map(|i| {
                            let c = pallete.get(i);
                            [c[0], c[1], c[2], trns_pallete.get(i)]
                        })
                        .collect(),
                },
                TransparencyData::None => Image {
                    width,
                    height,
                    img_type: ImageType::Rgb8,
                    data: indexes
                        .into_iter()
                        .flat_map(|i| pallete.get(i))
                        .copied()
                        .collect(),
                },

                _ => unreachable!(),
            }
        }
        ColorType::GreyscaleAlpha => {
            if bit_depth == 16 {
                Image {
                    width,
                    height,
                    img_type: ImageType::Ra16,
                    data: unfiltered_data,
                }
            } else {
                Image {
                    width,
                    height,
                    img_type: ImageType::Ra8,
                    data: unfiltered_data,
                }
            }
        }
    };

    if bit_depth == 16 {
        let data_inversed = img.data.chunks(2).flat_map(|i| [i[1], i[0]]).collect();
        img.data = data_inversed;
    }

    Ok(img)
}
