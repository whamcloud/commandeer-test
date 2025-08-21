use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Expr, Ident, ItemFn, Result,
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote,
};

struct CommandeerArgs {
    mode: Ident,
    commands: Vec<String>,
}

const RECORD: &str = "Record";
const REPLAY: &str = "Replay";

impl Parse for CommandeerArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut commands = vec![];

        let ident: Ident = input.parse()?;

        let mode = match ident.to_string().as_str() {
            x if [RECORD, REPLAY].contains(&x) => Ident::new(x, proc_macro2::Span::call_site()),
            _ => {
                return Err(syn::Error::new(
                    ident.span(),
                    format!("Expected '{RECORD}' or '{REPLAY}'"),
                ));
            }
        };

        input.parse::<syn::Token![,]>()?;

        while !input.is_empty() {
            if input.peek(syn::LitStr) {
                let lit: syn::LitStr = input.parse()?;

                commands.push(lit.value());
            } else {
                return Err(input.error("Expected a command string"));
            }

            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        if commands.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "Expected at least one command string",
            ));
        }

        Ok(CommandeerArgs { mode, commands })
    }
}

/// Procedural macro for setting up commandeer test environment
///
/// Usage: `#[commandeer(Record, "echo", "ls")]`
///
/// This expands to code that creates a Commandeer instance and mocks the specified commands
#[proc_macro_attribute]
pub fn commandeer(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as CommandeerArgs);
    let mut input_fn = parse_macro_input!(input as ItemFn);

    let fn_name = &input_fn.sig.ident;

    let test_file_name = format!("cmds_{fn_name}.json");

    // Split by commas and parse each part

    let mock_commands: Vec<Expr> = args
        .commands
        .iter()
        .map(|cmd| {
            parse_quote! {
                commandeer.mock_command(#cmd)
            }
        })
        .collect();

    let mode = args.mode;

    // Create the setup statements
    let setup_stmts: Vec<syn::Stmt> = vec![parse_quote! {
        let commandeer = commandeer_test::Commandeer::new(#test_file_name, commandeer_test::Mode::#mode);
    }];

    let mock_stmts: Vec<syn::Stmt> = mock_commands
        .iter()
        .map(|expr| {
            parse_quote! {
                #expr;
            }
        })
        .collect();

    // Prepend the setup code to the function body
    let mut new_stmts = setup_stmts;
    new_stmts.extend(mock_stmts);
    new_stmts.extend(input_fn.block.stmts);

    input_fn.block.stmts = new_stmts;

    let body_str = quote!(#input_fn).to_string();

    if body_str.contains("local_serial_core") {
        return syn::Error::new_spanned(
            input_fn.sig.fn_token,
            "Out of order error. `commandeer` macro must be above the `serial_test` macro.",
        )
        .to_compile_error()
        .into();
    }

    TokenStream::from(quote! { #input_fn })
}
