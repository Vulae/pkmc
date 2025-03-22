use rsa::traits::PublicKeyParts;
use sha1::{Digest, Sha1};

/// Calculate player hash from player username.
#[allow(clippy::format_collect)]
pub fn calc_hash(name: &str) -> String {
    // https://gist.github.com/RoccoDev/8fa130f1946f89702f799f89b8469bc9
    let mut hasher = Sha1::new();
    sha1::digest::Digest::update(&mut hasher, name.as_bytes());
    let mut hex = hasher.finalize();

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
    use crate::crypto::calc_hash;

    #[test]
    fn calc_hash_test() {
        assert_eq!(
            "-7c9d5b0044c130109a5d7b5fb5c317c02b4e28c1",
            calc_hash("jeb_"),
        );
        assert_eq!(
            "4ed1f46bbe04bc756bcb17c0c7ce3e4632f06a48",
            calc_hash("Notch"),
        );
        assert_eq!(
            "88e16a1019277b15d58faf0541e11910eb756f6",
            calc_hash("simon"),
        );
    }
}
