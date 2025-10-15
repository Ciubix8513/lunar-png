use std::{io::Read, path::PathBuf};

use crate::{decoding::Error, helpers::to_u16};

use super::*;

#[test]
fn test_loading() {
    let mut incorect_image = include_bytes!("../test-data/garbage.png")
        .to_vec()
        .into_iter();
    assert_eq!(read_png(&mut incorect_image), Err(Error::InvalidSignature));

    let images = PathBuf::from("test-data/basic_tests").read_dir().unwrap();

    for i in images {
        let mut data = Vec::new();

        let file = i.unwrap().path();

        println!("\nLoading {}", file.file_name().unwrap().to_str().unwrap());
        std::fs::OpenOptions::new()
            .read(true)
            .open(file)
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();

        let img = read_png(&mut data.into_iter());

        assert!(img.is_ok());
    }
}

#[test]
fn test_u8_to_u16() {
    let a = 0x00;
    let b = 0x00;
    let expected = 0x0000;
    assert_eq!(to_u16(a, b), expected);

    let a = 0x01;
    let b = 0x10;
    let expected = 0x1001;
    assert_eq!(to_u16(a, b), expected);
}

#[test]
fn encoding() {
    let img = Image {
        width: 8,
        height: 8,
        img_type: ImageType::R8,
        data: (0..64).map(|_| 255).collect(),
    };

    let png = encode_png(
        &img,
        PngEncodingOptions {
            compression: CompressionLevel::None,
            write_timestamp: true,
        },
    );

    let img1 = read_png(&mut png.into_iter()).unwrap();

    assert_eq!(img, img1);
}

#[test]
fn encoding_compressed() {
    let img = Image {
        width: 8,
        height: 8,
        img_type: ImageType::R8,
        data: (0..64).map(|_| 255).collect(),
    };

    let png = encode_png(
        &img,
        PngEncodingOptions {
            compression: CompressionLevel::Fast,
            write_timestamp: true,
        },
    );

    let img1 = read_png(&mut png.into_iter()).unwrap();

    assert_eq!(img, img1);
}

#[test]
fn all_image_reencoding() {
    let images = PathBuf::from("test-data/basic_tests").read_dir().unwrap();

    for i in images {
        let mut data = Vec::new();

        let file = i.unwrap().path();

        println!("\nLoading {}", file.file_name().unwrap().to_str().unwrap());
        std::fs::OpenOptions::new()
            .read(true)
            .open(file)
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();

        let img = read_png(&mut data.into_iter()).unwrap();

        let png = encode_png(
            &img,
            PngEncodingOptions {
                compression: CompressionLevel::Fast,
                write_timestamp: false,
            },
        );

        let img1 = read_png(&mut png.into_iter()).unwrap();

        assert_eq!(img.img_type, img1.img_type);
        assert_eq!(img.width, img1.width);
        assert_eq!(img.height, img1.height);
        assert_eq!(img.data.len(), img1.data.len());

        for (ind, (a, b)) in img.data.iter().zip(img1.data.iter()).enumerate() {
            if a != b {
                panic!("Pixel {ind}: {a} != {b}");
            }
        }
    }
}
