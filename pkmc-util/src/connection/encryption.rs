use aes::cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConnectionEncryptionError {
    #[error(transparent)]
    AesInvalidLength(#[from] aes::cipher::InvalidLength),
}

#[derive(Default, Debug)]
pub enum ConnectionEncryption {
    #[default]
    Unencrypted,
    Aes {
        key: aes::Aes128,
        encrypt_vector: [u8; 16],
        decrypt_vector: [u8; 16],
    },
}

#[inline(always)]
fn shift_left<const N: usize>(buf: &mut [u8; N], last: u8) {
    for i in 0..(N - 1) {
        buf[i] = buf[i + 1];
    }
    buf[N - 1] = last;
}

impl ConnectionEncryption {
    pub fn new_aes(shared_secret: &[u8; 16]) -> Result<Self, ConnectionEncryptionError> {
        Ok(Self::Aes {
            key: aes::Aes128::new_from_slice(shared_secret)?,
            encrypt_vector: *shared_secret,
            decrypt_vector: *shared_secret,
        })
    }

    pub(crate) fn encrypt(
        &mut self,
        plaintext: &mut [u8],
    ) -> Result<(), ConnectionEncryptionError> {
        match self {
            ConnectionEncryption::Unencrypted => Ok(()),
            &mut ConnectionEncryption::Aes {
                ref key,
                ref mut encrypt_vector,
                ..
            } => {
                let mut temp_block = GenericArray::from([0u8; 16]);

                plaintext.iter_mut().for_each(|plain_byte| {
                    temp_block.clone_from_slice(encrypt_vector);
                    key.encrypt_block(&mut temp_block);

                    let cipher_byte = temp_block[0] ^ *plain_byte;
                    *plain_byte = cipher_byte;

                    shift_left(encrypt_vector, cipher_byte);
                });

                Ok(())
            }
        }
    }

    pub(crate) fn decrypt(
        &mut self,
        ciphertext: &mut [u8],
    ) -> Result<(), ConnectionEncryptionError> {
        match self {
            ConnectionEncryption::Unencrypted => Ok(()),
            &mut ConnectionEncryption::Aes {
                ref key,
                ref mut decrypt_vector,
                ..
            } => {
                let mut temp_block = GenericArray::from([0u8; 16]);

                ciphertext.iter_mut().for_each(|cipher_byte| {
                    temp_block.clone_from_slice(decrypt_vector);
                    key.encrypt_block(&mut temp_block);

                    let plain_byte = temp_block[0] ^ *cipher_byte;

                    shift_left(decrypt_vector, *cipher_byte);

                    *cipher_byte = plain_byte;
                });

                Ok(())
            }
        }
    }
}
