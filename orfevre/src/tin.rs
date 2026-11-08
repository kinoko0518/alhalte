use aes_gcm_siv::{Aes256GcmSiv, KeyInit, aead::Aead};

pub enum TinError {
    CipherError(aes_gcm_siv::Error),
}

pub struct Tin(Vec<u8>);

const ZERO_NONCE: [u8; 12] = [0; 12];

type ConvergentKey = Aes256GcmSiv;

fn get_convkey(bytes: &[u8]) -> ConvergentKey {
    let hash = blake3::hash(&bytes);
    ConvergentKey::new(hash.as_bytes().into())
}

impl Tin {
    pub fn tin(plaintext: &[u8]) -> Result<Tin, TinError> {
        let cipher = get_convkey(plaintext);
        let encrypted = cipher
            .encrypt(&ZERO_NONCE.into(), &plaintext[..])
            .map_err(TinError::CipherError)?;

        Ok(Tin(encrypted))
    }

    pub fn open(ciphertext: Tin, conv_key: ConvergentKey) -> Result<Vec<u8>, TinError> {
        let decrypted = conv_key
            .decrypt(&ZERO_NONCE.into(), &ciphertext.0[..])
            .map_err(TinError::CipherError)?;

        Ok(decrypted)
    }
}
