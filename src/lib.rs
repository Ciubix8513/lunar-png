use std::fmt::Debug;

use helpers::*;

mod helpers;
#[cfg(test)]
mod tests;

static SIGNATURE: &[u8; 8] = &[0x89u8, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidSignature,
    InvalidChunkType,
}

#[derive(Debug, PartialEq)]
pub enum ImageType {
    Rgb8,
    Rgba8,
    Rgb16,
    Rgba16,
}

enum TransparencyData {
    None,
    Greyscale(u16),
    Truecolor(u16, u16, u16),
    Indexed(TrnsPallete),
}

#[derive(PartialEq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub img_type: ImageType,
    pub data: Vec<u8>,
}

impl Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Image")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("img_type", &self.img_type)
            .finish()
    }
}

impl Image {
    ///Adds an alpha channel to the image, does nothing if the image already contains an alpha
    ///channel
    pub fn add_alpha(&mut self) {
        match self.img_type {
            ImageType::Rgb8 => {
                self.img_type = ImageType::Rgba8;
                self.data = self
                    .data
                    .chunks(3)
                    .flat_map(|c| [c[0], c[1], c[2], 0xff])
                    .collect();
            }
            ImageType::Rgb16 => {
                self.img_type = ImageType::Rgba16;
                self.data = self
                    .data
                    .chunks(6)
                    .flat_map(|c| [c[0], c[1], c[2], c[3], c[4], c[5], 0xff, 0xff])
                    .collect();
            }
            _ => {}
        }
    }
}

pub fn read_png(stream: &mut impl Iterator<Item = u8>) -> Result<Image, Error> {
    if &read_n_const(stream) != SIGNATURE {
        //if signature is incorrect , return a corresponding error
        return Err(Error::InvalidSignature);
    }

    let first_chunk = parse_chunk(stream);

    if first_chunk.chunk_type != ChunkType::IHDR {
        panic!("Invalid png file, IHDR must be the first chunk")
    }

    let mut data_iter = first_chunk.data.into_iter();
    let width = u32::from_be_bytes(read_n_const(&mut data_iter));
    let height = u32::from_be_bytes(read_n_const(&mut data_iter));

    let bit_depth = data_iter.next().unwrap();
    let color_type = to_color_type(data_iter.next().unwrap());
    let compression_method = data_iter.next().unwrap();
    let filter_method = data_iter.next().unwrap();
    let interlace_method = data_iter.next().unwrap();

    if !validate_bit_depth(color_type, bit_depth) {
        panic!("Invalid bit depth for {color_type:?} type");
    }

    if compression_method != 0 {
        panic!("Invalid compression method");
    }

    println!("  | Image is {} x {}", width, height);
    println!("  | Bit depth = {bit_depth}");
    println!("  | Color type = {color_type:?}");
    println!("  | Compression method = {compression_method}");
    println!("  | Filter method = {filter_method}");
    println!("  | interlace_method = {interlace_method}");
    let mut total_chunks = 1;

    //Start the chunk reading loop
    let mut png_data = Vec::new();
    let mut reached_data = false;

    let mut pallete = Pallete::empty();

    let mut trns_data = TransparencyData::None;

    loop {
        //Get the chunk
        let chunk = parse_chunk(stream);

        total_chunks += 1;

        if chunk.chunk_type == ChunkType::IEND {
            println!("  | Finished reading file");
            println!("  | total chunks {total_chunks}");

            break;
        }

        if reached_data && chunk.chunk_type != ChunkType::IDAT {
            panic!("Invaluid file: Data chunks can not be interupted")
        }

        match chunk.chunk_type {
            ChunkType::PLTE => {
                //Get the palette
                pallete = Pallete::new(chunk.data);
            }
            ChunkType::IDAT => {
                reached_data = true;
                png_data.extend_from_slice(&chunk.data);
            }
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
                    _ => panic!("Image can not contain tRNS chunk"),
                }
            }
            // ChunkType::cHRM => todo!(),
            // ChunkType::gAMA => todo!(),
            // ChunkType::iCCP => todo!(),
            // ChunkType::sBIT => todo!(),
            // ChunkType::sRGB => todo!(),
            // ChunkType::cICP => todo!(),(
            // ChunkType::mDCv => todo!(),
            // ChunkType::bKGD => todo!(),
            // ChunkType::hIST => todo!(),
            // ChunkType::pHYs => todo!(),
            // ChunkType::sPLT => todo!(),
            // ChunkType::eXIf => todo!(),
            // ChunkType::tIME => todo!(),
            // ChunkType::acTL => todo!(),
            // ChunkType::fcTL => todo!(),
            // ChunkType::fdAT => todo!(),
            _ => println!("  | {:?}", chunk.chunk_type),
        }
    }

    //Decompress the data
    let mut data = Vec::new();
    {
        let mut decoder = flate2::read::ZlibDecoder::new(png_data.as_slice());
        use std::io::Read;
        decoder.read_to_end(&mut data).unwrap();
    }

    //Defiltering
    match filter_method {
        //None
        0 => {}
        //Sub
        1 => {}
        //Up
        2 => {}
        //Average
        3 => {}
        //Paeth
        4 => {}
        _ => panic!("Invalid filter method"),
    }

    let img = match color_type {
        ColorType::Greyscale => {
            let new_data = match bit_depth {
                1 | 2 | 4 => {
                    let mut o = Vec::new();

                    //Iterate over N bits (N = bit_depth)
                    //Extract data i & 2^N - 1 << iter_num
                    //Normalize over 0-255 (255 / ((2 << N) - 1) * num)
                    for i in &data {
                        for index in 0..(8 / bit_depth) {
                            let indexer: u8 = (1 << bit_depth) - 1;
                            let num = (i & (indexer << index)) >> index;
                            let normalized = (255 / ((1 << bit_depth) - 1)) * num;

                            o.push(normalized);
                        }
                    }
                    o
                }
                16 | 8 => data,
                _ => panic!("Invalid bit depth"),
            };

            if bit_depth == 16 {
                let mut img = Image {
                    width,
                    height,
                    img_type: ImageType::Rgb16,
                    data: new_data
                        .chunks(2)
                        .into_iter()
                        .flat_map(|i| [i[0], i[1], i[0], i[1], i[0], i[1]])
                        .collect(),
                };

                //if a transparency chunck is found, modify the image type and stick on some bytes
                //at the end of every pixel
                match trns_data {
                    TransparencyData::Greyscale(value) => {
                        img.img_type = ImageType::Rgba16;
                        img.data = img
                            .data
                            .chunks(6)
                            .flat_map(|c| {
                                let v = if to_u16(c[0], c[1]) == value { 0 } else { 0xff };
                                [c[0], c[1], c[2], c[3], c[4], c[5], v, v]
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
                    img_type: ImageType::Rgb8,
                    data: new_data.into_iter().flat_map(|i| [i, i, i]).collect(),
                };

                match trns_data {
                    TransparencyData::Greyscale(value) => {
                        img.img_type = ImageType::Rgba16;
                        img.data = img
                            .data
                            .chunks(3)
                            .flat_map(|c| {
                                let v = if c[0] as u16 == value { 0 } else { 0xff };
                                [c[0], c[1], c[2], v]
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
                data,
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
            data,
        },
        ColorType::IndexedColor => {
            let indexes = match bit_depth {
                1 | 2 | 4 => {
                    let mut o = Vec::new();

                    //Iterate over N bits (N = bit_depth)
                    //Extract data i & 2^N - 1 << iter_num
                    for i in &data {
                        for index in 0..(8 / bit_depth) {
                            let indexer: u8 = (1 << bit_depth) - 1;
                            let num = (i & (indexer << index)) >> index;
                            o.push(num);
                        }
                    }
                    o
                }
                8 => data,
                _ => panic!("Invalid bit depth"),
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
                    //TODO deal with transparency
                    img_type: ImageType::Rgb8,
                    data: indexes
                        .into_iter()
                        .flat_map(|i| pallete.get(i))
                        .cloned()
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
                    img_type: ImageType::Rgba16,
                    data: data
                        .chunks(4)
                        .flat_map(|i| [i[2], i[3], i[2], i[3], i[2], i[3], i[0], i[1]])
                        .collect(),
                }
            } else {
                Image {
                    width,
                    height,
                    img_type: ImageType::Rgba8,
                    data: data
                        .chunks(2)
                        .flat_map(|i| [i[1], i[1], i[1], i[0]])
                        .collect(),
                }
            }
        }
    };

    Ok(img)
}
