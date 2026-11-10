use std::string::FromUtf8Error;

use aes_gcm_siv::{Aes256GcmSiv, KeyInit, aead::Aead};

#[derive(Debug)]
pub enum TinError {
    CipherError(aes_gcm_siv::Error),
    NotEnoughBytesForMimetypeWidth,
    NotEnoughBytesForMimetype,
    MimetypeIsntUtf8(FromUtf8Error),
}

pub struct Tin(Vec<u8>);

const ZERO_NONCE: [u8; 12] = [0; 12];

type ConvergentKey = Aes256GcmSiv;

fn get_convkey(bytes: &[u8]) -> ConvergentKey {
    let hash = blake3::hash(&bytes);
    ConvergentKey::new(hash.as_bytes().into())
}

type MimetypeLen = u32;
const MIMETYPE_WIDTH: usize = size_of::<MimetypeLen>();

impl Tin {
    pub fn tin(plaintext: &[u8], mimetype: &str) -> Result<(Tin, ConvergentKey), TinError> {
        let bin = (mimetype.len() as MimetypeLen)
            .to_le_bytes()
            .into_iter()
            .chain(mimetype.as_bytes().into_iter().copied())
            .chain(plaintext.into_iter().copied())
            .collect::<Vec<u8>>();

        let cipher = get_convkey(&bin[..]);
        let encrypted = cipher
            .encrypt(&ZERO_NONCE.into(), &bin[..])
            .map_err(TinError::CipherError)?;

        Ok((Tin(encrypted), cipher))
    }

    fn mimetype_len(bin: &[u8]) -> Result<usize, TinError> {
        bin.first_chunk::<MIMETYPE_WIDTH>()
            .ok_or(TinError::NotEnoughBytesForMimetypeWidth)
            .map(|x| (0..x.len()).fold(0 as MimetypeLen, |acc, y| acc | (x[y] as u32) << y))
            .map(|x| x as usize)
    }

    fn deconstruct(mut bin: Vec<u8>) -> Result<(String, Vec<u8>), TinError> {
        let mimetype_len = Self::mimetype_len(&bin[..])?;
        let mimetype_ends = mimetype_len + MIMETYPE_WIDTH;

        let mimetype = bin
            .get(MIMETYPE_WIDTH..mimetype_ends)
            .ok_or(TinError::NotEnoughBytesForMimetype)
            .and_then(|x| String::from_utf8(x.to_vec()).map_err(TinError::MimetypeIsntUtf8))?;
        let payload = bin.split_off(mimetype_ends);

        Ok((mimetype, payload))
    }

    pub fn open(self, conv_key: ConvergentKey) -> Result<(String, Vec<u8>), TinError> {
        let decrypted = conv_key
            .decrypt(&ZERO_NONCE.into(), &self.0[..])
            .map_err(TinError::CipherError)?;
        let deconstructed = Self::deconstruct(decrypted)?;

        Ok(deconstructed)
    }
}

#[test]
fn tin_open_close_test() {
    let plaintext = vec![0_u8; 64];
    let mimetype = "application/zero";

    let (tin, conv_key) = Tin::tin(&plaintext, mimetype).unwrap();
    let (dmimetype, dplaintext) = tin.open(conv_key).unwrap();

    assert_eq!(mimetype, dmimetype);
    assert_eq!(plaintext, dplaintext);
}
