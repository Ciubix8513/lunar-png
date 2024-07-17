# lunar-png
A simple png loading library


# Usage
```rust
let mut file = std::fs::File::open("something.png").unwrap();
let mut data = Vec::new();

file.read_to_end(&mut data);

let image = lunar_png::read_png(&mut data.into_iter()).unwrap();
```
