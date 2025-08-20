use proc_macro::TokenStream;
use quote::quote;
use syn::{Expr, Ident, ItemFn, parse_macro_input, parse_quote};

/// Procedural macro for setting up commandeer test environment
///
/// Usage: `#[commandeer(Record, "echo", "ls")]`
///
/// This expands to code that creates a Commandeer instance and mocks the specified commands
#[proc_macro_attribute]
pub fn commandeer(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut input_fn = parse_macro_input!(input as ItemFn);

    // Parse arguments manually since AttributeArgs was removed in syn 2.0
    let args_str = args.to_string();
    let fn_name = &input_fn.sig.ident;

    // Generate test file name from function name
    let test_file_name = format!("cmds_{fn_name}.json");

    // Simple parsing - look for Record/Replay and string literals
    let mut mode_str = "Record"; // default
    let mut commands = vec![];

    // Split by commas and parse each part
    for part in args_str.split(',') {
        let part = part.trim();
        if part == "Record" || part == "Replay" {
            mode_str = part;
        } else if part.starts_with('"') && part.ends_with('"') {
            // Remove quotes and add to commands
            let cmd = &part[1..part.len() - 1];
            commands.push(cmd.to_string());
        }
    }

    let mode_ident = Ident::new(mode_str, proc_macro2::Span::call_site());

    // Generate mock_command calls
    let mock_commands: Vec<Expr> = commands
        .iter()
        .map(|cmd| {
            parse_quote! {
                commandeer.mock_command(#cmd)
            }
        })
        .collect();

    // Create the setup statements
    let setup_stmts: Vec<syn::Stmt> = vec![parse_quote! {
        let commandeer = crate::Commandeer::new(#test_file_name, crate::Mode::#mode_ident);
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

    TokenStream::from(quote! { #input_fn })
}
