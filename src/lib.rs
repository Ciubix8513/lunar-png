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

pub fn read_png(stream: &mut impl Iterator<Item = u8>) -> Result<(), Error> {
    if &read_n_const(stream) != SIGNATURE {
        //if signature is incorrect , return a corresponding error
        return Err(Error::InvalidSignature);
    }

    Ok(())
}
