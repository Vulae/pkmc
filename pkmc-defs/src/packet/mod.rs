pub mod configuration;
pub mod handshake;
pub mod login;
pub mod play;
pub mod status;

#[macro_export]
macro_rules! generate_id_enum {
    ($vis:vis $ident:ident; $($value:ident => $id:literal,)+) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        $vis enum $ident {
            $($value,)+
        }

        impl $ident {
            $vis fn into_id(&self) -> i32 {
                match self {
                    $(Self::$value => $id,)+
                }
            }

            $vis fn from_id(id: i32) -> Option<Self> {
                Some(match id {
                    $($id => Self::$value,)+
                    _ => return None,
                })
            }
        }

        impl ::pkmc_util::connection::PacketEncodable for $ident {
            fn packet_encode(self, mut writer: impl ::std::io::Write) -> ::std::io::Result<()> {
                use ::pkmc_util::connection::PacketEncoder as _;
                writer.encode(self.into_id())?;
                Ok(())
            }
        }

        impl ::pkmc_util::connection::PacketDecodable for $ident
        where
            Self: Sized,
        {
            fn packet_decode(mut reader: impl ::std::io::Read) -> ::std::io::Result<Self> {
                let id: i32 = reader.decode()?;
                Self::from_id(id).ok_or_else(|| ::std::io::Error::new(
                    ::std::io::ErrorKind::InvalidData,
                    format!("Invalid ID {} for $ident", id),
                ))
            }
        }
    };
}
