pub mod encrypted;

use std::ops::Deref;

use aes_gcm_siv::{
    Aes256GcmSiv, Key, KeyInit, Nonce,
    aead::{Aead, Generate},
};
use blake3::Hasher;

pub use crate::encrypted::Encrypted;
use crate::encrypted::{ConvergentKey, high_entropy_encrypt, low_entropy_encrypt};

pub enum CannerError {
    IoError(std::io::Error),
    AesGcmSivError(aes_gcm_siv::Error),
    OpenerByteNotFound,
    UnknownCompression(u8),
    UnknownEncryption(u8),
    LenNotFound,
    NonceNotFound,
    LabelNotFound,
}

const LABEL_BYTELEN: usize = 32;
const WRAPPED_KEY_LEN: usize = 32 + 28;

type Label = [u8; LABEL_BYTELEN];
type RoomKey = Key<Aes256GcmSiv>;

/// RoomKeyで暗号化された収束鍵。先頭12byteはNonce。
pub struct WrappedKey(pub [u8; WRAPPED_KEY_LEN]);

impl TryFrom<Vec<u8>> for WrappedKey {
    type Error = String;

    fn try_from(vec: Vec<u8>) -> Result<Self, Self::Error> {
        let arr: [u8; WRAPPED_KEY_LEN] = vec
            .try_into()
            .map_err(|v: Vec<u8>| format!("Expected {} bytes, got {}", WRAPPED_KEY_LEN, v.len()))?;
        Ok(Self(arr))
    }
}

impl Deref for WrappedKey {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub type TinID = [u8; 32];

impl WrappedKey {
    pub fn wrap(conv_key: ConvergentKey, room_key: RoomKey) -> Result<Self, aes_gcm_siv::Error> {
        let cipher = Aes256GcmSiv::new(&room_key);
        let nonce = Nonce::generate();
        let mut bytes = nonce.to_vec();
        bytes.extend(cipher.encrypt(&nonce, &conv_key[..])?);

        Ok(Self(bytes.try_into().unwrap()))
    }
}

pub struct Tin {
    label: Label,
    encrypted: Encrypted,
}

impl Tin {
    pub fn plaintext(label: Label, bytes: Vec<u8>) -> Self {
        let encrypted = Encrypted::Plaintext(bytes);

        Self { label, encrypted }
    }

    pub fn high_entropy(label: Label, bytes: Vec<u8>) -> Result<Self, CannerError> {
        let encrypted = Encrypted::HighEntropy(high_entropy_encrypt(bytes)?);

        Ok(Self { label, encrypted })
    }

    pub fn low_entropy(
        label: Label,
        bytes: Vec<u8>,
        room_key: impl Into<RoomKey>,
    ) -> Result<(Self, WrappedKey), CannerError> {
        let (ciphertext, wkey) = low_entropy_encrypt(bytes, room_key.into())?;
        let encrypted = Encrypted::LowEntropy(ciphertext);
        Ok((Self { label, encrypted }, wkey))
    }

    pub fn into_bytes(self) -> Vec<u8> {
        let mut bytes = vec![self.encrypted.opener()];
        bytes.extend(self.label);
        bytes.extend(self.encrypted.into_bytes());
        bytes
    }

    pub fn from_bytes(mut bytes: Vec<u8>) -> Result<Self, CannerError> {
        let label: [u8; LABEL_BYTELEN] = (bytes.len() >= LABEL_BYTELEN)
            .then(|| {
                let mut label = bytes.drain(..LABEL_BYTELEN);
                let label = std::array::from_fn(|_| label.next().unwrap());
                label
            })
            .ok_or(CannerError::LabelNotFound)?;
        let encrypted = Encrypted::try_from_bytes(bytes)?;

        Ok(Self { label, encrypted })
    }

    pub fn get_id(&self) -> TinID {
        let mut hasher = Hasher::new();

        self.encrypted.hash(&mut hasher);
        hasher.update(&self.label);

        hasher.finalize().into()
    }
}
