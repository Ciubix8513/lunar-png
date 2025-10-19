use std::io::Write;

use crate::{
    Image, ImageType,
    helpers::{Filtered, compute_crc},
};
use chrono::{Datelike, Timelike};
use flate2::Compression;
use pack1::{U16BE, U32BE};

///Compression level of the encoded image
#[derive(Default, Debug, Copy, Clone)]
pub enum CompressionLevel {
    ///No compression at all, fastest
    None,
    ///Do some compression, but optimize for time
    #[default]
    Fast,
    ///Best compression, slowest encoding
    Best,
}

///Settings for png encoding
//Compression options,
//whether to write a timestamp
//etc?
#[derive(Default, Debug, Clone, Copy)]
pub struct PngEncodingOptions {
    ///How much to compress  the image
    pub compression: CompressionLevel,
    ///Wether to write a time stamp to the image
    pub write_timestamp: bool,
}

#[repr(C, packed)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
struct Header {
    signature: [u8; 4],
    width: U32BE,
    height: U32BE,
    bit_depth: u8,
    color_type: u8,
    compression_method: u8,
    filter_method: u8,
    interlace_method: u8,
}

#[repr(C, packed)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
struct Time {
    signature: [u8; 4],
    year: U16BE,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}

///Encodes a png into a byte stream
#[must_use]
#[allow(clippy::too_many_lines, clippy::missing_panics_doc)]
pub fn encode_png(image: &Image, options: &PngEncodingOptions) -> Vec<u8> {
    //Chunk support:
    //IHDR
    //IDAT
    //IEND
    //tIME

    let mut stream = Vec::new();

    let signature = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    stream.extend_from_slice(&signature);

    let header = Header {
        //IHDR
        signature: [0x49, 0x48, 0x44, 0x52],
        width: image.width.into(),
        height: image.height.into(),
        bit_depth: match image.img_type {
            ImageType::R8 | ImageType::Ra8 | ImageType::Rgb8 | ImageType::Rgba8 => 8,
            _ => 16,
        },
        color_type: match image.img_type {
            ImageType::R8 | ImageType::R16 => 0,
            ImageType::Ra8 | ImageType::Ra16 => 4,
            ImageType::Rgb8 | ImageType::Rgb16 => 2,
            ImageType::Rgba8 | ImageType::Rgba16 => 6,
        },
        compression_method: 0,
        filter_method: 0,
        interlace_method: 0,
    };

    let bytes = bytemuck::bytes_of(&header);
    let length: U32BE = (size_of::<Header>() as u32 - 4).into();
    let crc: U32BE = compute_crc(bytes).into();

    stream.extend_from_slice(bytemuck::bytes_of(&length));
    stream.extend_from_slice(bytes);
    stream.extend_from_slice(bytemuck::bytes_of(&crc));

    if options.write_timestamp {
        let time = chrono::Utc::now();

        let time = Time {
            signature: [0x74, 0x49, 0x4D, 0x45],
            year: (time.year() as u16).into(),
            month: time.month() as u8,
            day: time.month() as u8,
            hour: time.hour() as u8,
            minute: time.minute() as u8,
            second: time.second() as u8,
        };

        let bytes = bytemuck::bytes_of(&time);
        let length: U32BE = (size_of::<Time>() as u32 - 4).into();
        let crc: U32BE = compute_crc(bytes).into();

        stream.extend_from_slice(bytemuck::bytes_of(&length));
        stream.extend_from_slice(bytes);
        stream.extend_from_slice(bytemuck::bytes_of(&crc));
    }

    //Written all the non data stuff

    let (filter, profile) = match options.compression {
        CompressionLevel::None => (false, Compression::none()),
        CompressionLevel::Fast => (true, Compression::fast()),
        CompressionLevel::Best => (true, Compression::best()),
    };

    //Allocate enough space for the entire image plus the filter markers
    let mut image_data = Vec::with_capacity(image.data.len() + image.height as usize);

    let scanline_size = image.width
        * match image.img_type {
            ImageType::R8 => 1,
            ImageType::R16 | ImageType::Ra8 => 2,
            ImageType::Rgb8 => 3,
            ImageType::Ra16 | ImageType::Rgba8 => 4,
            ImageType::Rgb16 => 6,
            ImageType::Rgba16 => 8,
        };

    let is_16 = image.img_type.is_16_bit();

    if filter {
        let filtetered = Filtered {
            data: if image.img_type.is_16_bit() {
                image.data.chunks(2).flat_map(|i| [i[1], i[0]]).collect()
            } else {
                image.data.clone()
            },
            color_type: match image.img_type {
                ImageType::R8 | ImageType::R16 => crate::helpers::ColorType::Greyscale,
                ImageType::Ra8 | ImageType::Ra16 => crate::helpers::ColorType::GreyscaleAlpha,
                ImageType::Rgb8 | ImageType::Rgb16 => crate::helpers::ColorType::Truecolor,
                ImageType::Rgba8 | ImageType::Rgba16 => crate::helpers::ColorType::TruecolorAlpha,
            },
            scanline_len: scanline_size,
            bit_depth: match image.img_type {
                ImageType::R8 | ImageType::Ra8 | ImageType::Rgb8 | ImageType::Rgba8 => 8,
                _ => 16,
            },
            ignore_0: false,
        };

        if is_16 {
            let data = image.data.chunks(2).flat_map(|i| [i[1], i[0]]).enumerate();

            for (ind, d) in data {
                if (ind as u32).is_multiple_of(scanline_size) {
                    image_data.push(4);
                }
                let pt = filtetered.paeth(ind);
                image_data.push(d.wrapping_sub(pt));
            }
        } else {
            //Paeth filtering
            for (ind, d) in image.data.iter().enumerate() {
                if (ind as u32).is_multiple_of(scanline_size) {
                    image_data.push(4);
                }
                let pt = filtetered.paeth(ind);
                image_data.push(d.wrapping_sub(pt));
            }
        }
    } else if is_16 {
        for (ind, d) in image.data.chunks(2).enumerate() {
            if (ind as u32).is_multiple_of(scanline_size / 2) {
                image_data.push(0);
            }

            image_data.push(d[1]);
            image_data.push(d[0]);
        }
    } else {
        for (ind, d) in image.data.iter().enumerate() {
            if (ind as u32).is_multiple_of(scanline_size) {
                image_data.push(0);
            }

            image_data.push(*d);
        }
    }

    let mut enc = flate2::write::ZlibEncoder::new(vec![0x49, 0x44, 0x41, 0x54], profile);
    enc.write_all(&image_data).unwrap();
    let compressed = enc.finish().unwrap();

    let len: U32BE = (compressed.len() as u32 - 4).into();
    let crc: U32BE = compute_crc(&compressed).into();

    //Data chunk
    stream.extend_from_slice(bytemuck::bytes_of(&len));
    stream.extend_from_slice(&compressed);
    stream.extend_from_slice(bytemuck::bytes_of(&crc));

    //end
    let data = [0x49u8, 0x45, 0x4e, 0x44];
    let len = 0u32;
    let crc: U32BE = compute_crc(&data).into();

    stream.extend_from_slice(bytemuck::bytes_of(&len));
    stream.extend_from_slice(&data);
    stream.extend_from_slice(bytemuck::bytes_of(&crc));

    stream
}
