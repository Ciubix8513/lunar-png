//! # lunar-png
//!
//! A simple png loading library
#![deny(missing_docs)]
#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
use std::fmt::Debug;

#[cfg(feature = "decoding")]
mod decoding;
#[cfg(feature = "encoding")]
mod encoding;

#[cfg(any(feature = "decoding", feature = "encoding"))]
mod helpers;
#[cfg(test)]
mod tests;

#[cfg(feature = "decoding")]
pub use decoding::{read_png, Error};
#[cfg(feature = "encoding")]
pub use encoding::{encode_png, CompressionLevel, PngEncodingOptions};

#[derive(Debug, PartialEq, Eq)]
///Image type of a loaded image
pub enum ImageType {
    ///A grayscale image with bit depth of 8
    R8,
    ///A grayscale image with bit depth of 16
    R16,
    ///A grayscale image  with an alpha channel and bit depth of 8
    Ra8,
    ///A grayscale image  with an alpha channel and bit depth of 16
    Ra16,
    ///An rgb image with the bit depth of 8
    Rgb8,
    ///An rgba image with the bit depth of 8
    Rgba8,
    ///An rgb image with the bit depth of 16
    Rgb16,
    ///An rgba image with the bit depth of 16
    Rgba16,
}

#[derive(PartialEq, Eq)]
///A loaded png image
pub struct Image {
    ///Width of the image
    pub width: u32,
    ///Height of the image
    pub height: u32,
    ///Type of the image
    pub img_type: ImageType,
    ///Actual data. Data in an image is stored in scanlines, going left to right, top to bottom
    pub data: Vec<u8>,
}

#[allow(clippy::missing_fields_in_debug)]
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
    ///Adds an alpha channel to the image, does nothing if the image already contains an alpha channel
    pub fn add_alpha(&mut self) {
        match self.img_type {
            ImageType::R8 => {
                self.img_type = ImageType::Ra8;
                self.data = self.data.iter().flat_map(|i| [*i, 0xff]).collect();
            }
            ImageType::R16 => {
                self.img_type = ImageType::Ra16;
                self.data = self
                    .data
                    .chunks(2)
                    .flat_map(|i| [i[0], i[1], 0xff, 0xff])
                    .collect();
            }
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

    ///Adds channels to a grayscale image. Does nothing if the image is not grayscale
    pub fn add_channels(&mut self) {
        match self.img_type {
            ImageType::R8 => {
                self.img_type = ImageType::Rgb8;
                self.data = self.data.iter().copied().flat_map(|i| [i, i, i]).collect();
            }
            ImageType::R16 => {
                self.img_type = ImageType::Rgb16;
                self.data = self
                    .data
                    .chunks(2)
                    .flat_map(|i| [i[0], i[1], i[0], i[1], i[0], i[1]])
                    .collect();
            }
            ImageType::Ra8 => {
                self.img_type = ImageType::Rgba8;
                self.data = self
                    .data
                    .chunks(2)
                    .flat_map(|i| [i[0], i[0], i[0], i[1]])
                    .collect();
            }
            ImageType::Ra16 => {
                self.img_type = ImageType::Rgba16;
                self.data = self
                    .data
                    .chunks(4)
                    .flat_map(|i| [i[0], i[1], i[0], i[1], i[0], i[1], i[2], i[3]])
                    .collect();
            }
            _ => {}
        }
    }
}
