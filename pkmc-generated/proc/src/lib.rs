use std::{env, io::Write as _, path::PathBuf};

use convert_case::Casing as _;
use quote::{quote, ToTokens};
use syn::{parse::Parse, parse_macro_input, spanned::Spanned as _, LitStr, Token};

mod reports;

// NOTE: Getting macro invocation file is currently unstable, so we just have this workaround to get it from the parent directory of this crate.
pub(crate) fn file_path(path: &str) -> PathBuf {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("Failed to get cargo manifest dir"));
    let base_dir = manifest_dir
        .parent()
        .expect("Cargo manifest dir must have parent");
    let mut file = base_dir.to_path_buf();
    file.push(path);
    file
}

pub(crate) fn fix_identifier(str: &str) -> String {
    str.trim_start_matches("minecraft:")
        .replace(":", "_")
        .to_case(convert_case::Case::Pascal)
}

#[proc_macro]
pub fn report_registry_generate_enum(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    reports::registries::report_registry_generate_enum(input)
}

#[proc_macro]
pub fn report_packets_generate_consts(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    reports::packets::report_packets_generate_consts(input)
}

struct CachedCompressedJson {
    input: LitStr,
    output: LitStr,
}

impl Parse for CachedCompressedJson {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            input: input.parse()?,
            output: {
                input.parse::<Token![,]>()?;
                input.parse()?
            },
        })
    }
}

impl ToTokens for CachedCompressedJson {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let input_file = file_path(&self.input.value());
        let output_file = file_path(&self.output.value());

        if !output_file.exists() {
            // We decode then encode JSON because it may be pretty formatted.
            let content: serde_json::Value = serde_json::from_reader(
                std::fs::File::open(&input_file).expect("Failed to open file"),
            )
            .expect("Failed to parse JSON");

            let mut encoder =
                flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
            encoder
                .write_all(&serde_json::to_vec(&content).expect("Failed to encode JSON"))
                .expect("Failed to GZ compress");
            let compressed = encoder.finish().expect("Failed to GZ compress");

            std::fs::write(&output_file, compressed).expect("Failed to write compressed contents");
        }

        let output_litstr = LitStr::new(
            output_file
                .canonicalize()
                .expect("Failed to canonicalize output path")
                .to_str()
                .expect("Failed to convert PathBuf to &str"),
            tokens.span(),
        );
        tokens.extend(quote! {{
            use std::io::Read as _;
            let mut decompressed = Vec::new();
            flate2::read::GzDecoder::new(std::io::Cursor::new(&include_bytes!(#output_litstr)))
                .read_to_end(&mut decompressed)
                .expect("Failed to decompress");
            decompressed
        }});
    }
}

/// Same as include_bytes!, but only for JSON and compresses content.
/// `let thing: Thing = serde_json::from_slice(&include_cached_json_compressed_bytes!("thing.json", "thing.json.gz")).unwrap()`
#[proc_macro]
pub fn include_cached_json_compressed_bytes(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let c = parse_macro_input!(input as CachedCompressedJson);
    quote! { #c }.into()
}
