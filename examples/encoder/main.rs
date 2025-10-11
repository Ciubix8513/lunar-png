use std::io::Write;

use lunar_png::*;

fn main() {
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

    let mut f = std::fs::File::create("test.png").unwrap();
    f.write_all(&png).unwrap();
}
