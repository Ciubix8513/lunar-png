use super::*;

#[test]
fn test_loading() {
    let mut image = include_bytes!("../test-data/pfp.png").to_vec().into_iter();

    let mut incorect_image = include_bytes!("../test-data/garbage.png")
        .to_vec()
        .into_iter();

    assert_eq!(read_png(&mut image), Ok(()));

    assert_eq!(read_png(&mut incorect_image), Err(Error::InvalidSignature));
}
