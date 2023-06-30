#![feature(proc_macro_quote)]

use proc_macro::TokenStream;

#[proc_macro_derive(EntityComponent)]
pub fn derive_answer_fn(item: TokenStream) -> TokenStream {
    println!("{:?}", item);
    let mut tokens = TokenStream::new();

    let input = syn::parse::<syn::DeriveInput>(item).unwrap();
    let identifier = input.ident.to_string();

    let crate_name = std::env::var("CARGO_PKG_NAME").unwrap();
    println!("package name: {}", crate_name);

    tokens.extend(
        format!(
            "
static mut {0}_ID: ResourceId = ResourceId::from_index_and_version(0, 0);

impl EntityComponent for {1} {{
    fn entity_component_id() -> ResourceId {{
        unsafe {{
            return {0}_ID;
        }}
    }}
}}",
            identifier.to_uppercase(),
            identifier
        )
        .parse::<TokenStream>()
        .unwrap(),
    );

    return tokens;
}
