use std::{io::Read, path::PathBuf};

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

        std::fs::OpenOptions::new()
            .read(true)
            .open(i.unwrap().path())
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();

        let img = read_png(&mut data.into_iter());

        assert_eq!(img, Ok(()));
    }
}
