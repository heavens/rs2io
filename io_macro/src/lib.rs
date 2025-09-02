use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Data, DeriveInput, Expr, Token};

/// A temporary struct to parse `key = value` pairs from inside the attribute.
struct PacketAttribute {
    key: syn::Ident,
    value: Expr,
}

impl Parse for PacketAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: syn::Ident = input.parse()?;
        let _: Token![=] = input.parse()?;
        let value = input.parse()?;
        Ok(PacketAttribute { key, value })
    }
}

#[proc_macro_derive(Protocol, attributes(packet))]
pub fn protocol_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let enum_name = &ast.ident;

    let variants = match &ast.data {
        Data::Enum(data_enum) => &data_enum.variants,
        _ => panic!("Protocol derive macro can only be used on enums"),
    };

    // --- Generate match arms for `opcode()` ---
    let opcode_matches = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        // Find the opcode from the `#[packet(...)]` attribute.
        let (opcode, _) = parse_packet_attributes(variant);

        let opcode_expr = opcode
            .unwrap_or_else(|| panic!("Variant {} must have an 'opcode' in its #[packet] attribute", variant_name));

        quote! {
            #enum_name::#variant_name => #opcode_expr,
        }
    });

    // --- Generate match arms for `size()` ---
    let size_matches = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        // Find the size from the `#[packet(...)]` attribute.
        let (_, size) = parse_packet_attributes(variant);

        let size_expr = size
            .unwrap_or_else(|| panic!("Variant {} must have a 'size' in its #[packet] attribute", variant_name));

        quote! {
            #enum_name::#variant_name => #size_expr,
        }
    });

    let expanded = quote! {
        impl #enum_name {
            pub fn opcode(&self) -> i32 {
                match self {
                    #(#opcode_matches)*
                }
            }



            pub fn size(&self) -> i32 {
                match self {
                    #(#size_matches)*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

/// Helper to find the `#[packet(...)]` attribute and parse its contents.
/// Returns a tuple of (Option<Expr>, Option<Expr>) for (opcode, size).
fn parse_packet_attributes(variant: &syn::Variant) -> (Option<Expr>, Option<Expr>) {
    let mut opcode = None;
    let mut size = None;

    // Find the `#[packet(...)]` attribute.
    let attribute = variant.attrs.iter()
        .find(|attr| attr.path.is_ident("packet"))
        .unwrap_or_else(|| panic!("Variant {} is missing the required #[packet(...)] attribute", variant.ident));

    // Parse the comma-separated `key = value` pairs inside the attribute's parentheses.
    let parser = |input: ParseStream| {
        syn::punctuated::Punctuated::<PacketAttribute, Token![,]>::parse_terminated(input)
    };

    let parsed_attrs = attribute.parse_args_with(parser)
        .unwrap_or_else(|e| panic!("Failed to parse attributes for variant {}: {}", variant.ident, e));

    // Find the 'opcode' and 'size' values from the parsed pairs.
    for attr in parsed_attrs {
        if attr.key == "opcode" {
            opcode = Some(attr.value);
        } else if attr.key == "size" {
            size = Some(attr.value);
        } else {
            panic!("Unknown key in #[packet] attribute for variant {}: {}", variant.ident, attr.key);
        }
    }

    (opcode, size)
}