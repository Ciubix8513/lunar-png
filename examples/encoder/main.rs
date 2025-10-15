use std::{
    io::{Read as _, Write},
    path::PathBuf,
};

use lunar_png::*;

fn test_data() {
    let images = PathBuf::from("test-data/basic_tests").read_dir().unwrap();

    for i in images {
        let mut data = Vec::new();

        let file = i.unwrap().path();
        let filename = file.file_name().unwrap().to_str().unwrap().to_string();

        println!("\nLoading {}", filename);
        std::fs::OpenOptions::new()
            .read(true)
            .open(file)
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();

        let img = read_png(&mut data.into_iter()).unwrap();

        let png = encode_png(
            &img,
            &PngEncodingOptions {
                compression: CompressionLevel::Fast,
                write_timestamp: false,
            },
        );

        let mut f = std::fs::File::create(format!("test-data/reencoded/{filename}")).unwrap();
        f.write_all(&png).unwrap();
    }
}

fn main() {
    let img = Image {
        width: 8,
        height: 8,
        img_type: ImageType::R8,
        data: (0..64).map(|_| 255).collect(),
    };

    let png = encode_png(
        &img,
        &PngEncodingOptions {
            compression: CompressionLevel::None,
            write_timestamp: true,
        },
    );

    let png1 = encode_png(
        &img,
        &PngEncodingOptions {
            compression: CompressionLevel::Fast,
            write_timestamp: true,
        },
    );

    let mut d1: [u8; 64] = [0; 64];

    for (ind, i) in read_png(&mut png1.iter().copied())
        .unwrap()
        .data
        .iter()
        .enumerate()
    {
        d1[ind] = *i;
    }

    let mut f = std::fs::File::create("test.png").unwrap();
    f.write_all(&png).unwrap();

    let mut f = std::fs::File::create("test_compressed.png").unwrap();
    f.write_all(&png1).unwrap();

    test_data();
}
