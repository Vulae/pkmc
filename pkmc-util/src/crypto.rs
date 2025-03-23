use rsa::traits::PublicKeyParts;
use sha1::{Digest, Sha1};

#[derive(Debug, Default)]
pub struct MinecraftSha1 {
    hasher: Sha1,
}

impl MinecraftSha1 {
    pub fn update<D: AsRef<[u8]>>(&mut self, data: D) {
        self.hasher.update(data)
    }

    #[allow(clippy::format_collect)]
    pub fn finalize(self) -> String {
        let mut hex = self.hasher.finalize();

        if hex[0] & 0x80 != 0 {
            {
                let mut carry = true;
                for i in (0..hex.len()).rev() {
                    hex[i] = !hex[i];
                    if carry {
                        carry = hex[i] == 0xFF;
                        hex[i] += 1;
                    }
                }
            }
            format!(
                "-{}",
                hex.iter()
                    .map(|v| format!("{:02x}", v))
                    .collect::<String>()
                    .trim_start_matches('0')
                    .to_owned(),
            )
        } else {
            hex.iter()
                .map(|v| format!("{:02x}", v))
                .collect::<String>()
                .trim_start_matches('0')
                .to_owned()
        }
    }

    pub fn calc<D: AsRef<[u8]>>(data: D) -> String {
        let mut hasher = Self::default();
        hasher.update(data);
        hasher.finalize()
    }
}

/// https://minecraft.wiki/w/Protocol_encryption#Key_Exchange
pub fn rsa_encode_public_key(key: &rsa::RsaPublicKey) -> Box<[u8]> {
    yasna::construct_der(|writer| {
        writer.write_sequence(|writer| {
            writer.next().write_sequence(|writer| {
                writer
                    .next()
                    .write_oid(&yasna::models::ObjectIdentifier::from_slice(&[
                        1, 2, 840, 113549, 1, 1, 1,
                    ]));
                writer.next().write_null();
            });

            let inner = yasna::construct_der(|writer| {
                writer.write_sequence(|writer| {
                    writer
                        .next()
                        .write_bigint_bytes(&key.n().to_bytes_be(), true);
                    writer
                        .next()
                        .write_bigint_bytes(&key.e().to_bytes_be(), true);
                });
            });
            writer.next().write_bitvec_bytes(&inner, inner.len() * 8);
        });
    })
    .into_boxed_slice()
}

#[cfg(test)]
mod test {
    use crate::crypto::MinecraftSha1;

    #[test]
    fn calc_hash_test() {
        assert_eq!(
            "-7c9d5b0044c130109a5d7b5fb5c317c02b4e28c1",
            MinecraftSha1::calc("jeb_"),
        );
        assert_eq!(
            "4ed1f46bbe04bc756bcb17c0c7ce3e4632f06a48",
            MinecraftSha1::calc("Notch"),
        );
        assert_eq!(
            "88e16a1019277b15d58faf0541e11910eb756f6",
            MinecraftSha1::calc("simon"),
        );
    }
}
