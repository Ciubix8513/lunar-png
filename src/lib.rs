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
pub enum ImageType {
    Rgb8,
    Rgba8,
    Rgb16,
    Rgba16,
}

pub struct Image {
    width: u32,
    height: u32,
    img_type: ImageType,
    data: Vec<u8>,
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

    println!("Image is {} x {}", width, height);
    println!("Bit depth = {bit_depth}");
    println!("Color type = {color_type:?}");
    println!("Compression method = {compression_method}");
    println!("Filter method = {filter_method}");
    println!("interlace_method = {interlace_method}");
    let mut total_chunks = 1;

    //Start the chunk reading loop
    let mut png_data = Vec::new();
    let mut reached_data = false;

    let mut pallete;

    loop {
        //Get the chunk
        let chunk = parse_chunk(stream);

        total_chunks += 1;

        if chunk.chunk_type == ChunkType::IEND {
            println!("Finished reading file");
            println!("total chunks {total_chunks}");

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
            // ChunkType::tRNS => todo!(),
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
            _ => println!("{:?}", chunk.chunk_type),
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
                            let num = i * indexer << index;

                            let normalized = (255 / ((1 << bit_depth) - 1)) * num;

                            o.push(normalized);
                        }
                    }
                    todo!()
                }
                8 => data,
                16 => {
                    todo!()
                }
                _ => panic!("Invalid bit depth"),
            };

            //We do be doing some iter crimes :3
            let data = new_data
                .iter()
                .copied()
                .zip(new_data.iter().cloned())
                .zip(new_data.iter().copied())
                .map(|i| vec![i.0 .0, i.0 .1, i.1])
                .flatten()
                .collect();

            Image {
                width,
                height,
                img_type: ImageType::Rgb8,
                data,
            }
        }
        ColorType::Truecolor => Image {
            width,
            height,
            img_type: if bit_depth == 8 {
                ImageType::Rgb8
            } else {
                ImageType::Rgb16
            },
            data,
        },
        ColorType::IndexedColor => todo!(),
        ColorType::GreyscaleAlpha => todo!(),
        ColorType::TruecolorAlpha => todo!(),
    };

    Ok(img)
}
