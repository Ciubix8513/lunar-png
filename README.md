# lunar-png
A simple png encoding and decoding library


# Usage
```rust
let mut file = std::fs::File::open("something.png").unwrap();
let mut data = Vec::new();

file.read_to_end(&mut data);

//Decode a png image
let image = decode_png(&mut data.into_iter()).unwrap();

//Re-encode that image
let png = encode_png(&image, &PngEncodingOptions::default());
```
