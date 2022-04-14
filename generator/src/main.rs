use std::borrow::Cow;

use anyhow::Result;
use ironworks_schema::{saint_coinach::Provider, Node, ReferenceTarget, Sheet};
use lazy_static::lazy_static;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use regex::Regex;

fn main() -> Result<()> {
	saint_coinach()?;

	Ok(())
}

// TODO: Seperate file and all that jazz.
fn saint_coinach() -> Result<()> {
	let provider = Provider::new()?;
	// TODO: fetch updates to the provider to make sure it's fresh
	// TODO: arg for version?
	let version = provider.version("HEAD")?;

	let schema = version.schema("CustomTalk")?;

	generate_sheet(schema);

	Ok(())
}

// TODO: some note about being an entry point
// TOOD: i'll probably need some generator "context" concept to register structs onto and suchforth
// TODO: names travel down the tree - how do i name the structs I generate? context?
fn generate_sheet(sheet: Sheet) {
	// TODO: handle the order field
	generate_node(&sheet.schema);
}

// TODO: gen node should probably return the (rust) type of the node
// TODO: it'll also need to return some way to _read_ itself
fn generate_node(node: &Node) -> TokenStream {
	match node {
		Node::Array { count, schema } => generate_array(count, schema),
		Node::Reference(targets) => generate_reference(targets),
		Node::Scalar => generate_scalar(),
		Node::Struct(fields) => generate_struct(fields),
	}
}

fn generate_array(count: &u32, schema: &Node) -> TokenStream {
	let type_identifier = generate_node(schema);
	quote! { #type_identifier[#count] }
}

fn generate_reference(_targets: &[ReferenceTarget]) -> TokenStream {
	let identifier = format_ident!("TodoReference");
	quote! { #identifier }
}

fn generate_scalar() -> TokenStream {
	// TODO: this will need column header data to resolve the type
	let identifier = format_ident!("TodoScalar");
	quote! { #identifier }
}

// in addition to returning the name of the struct, something about registering?
fn generate_struct(fields: &[(String, Node)]) -> TokenStream {
	let field_tokens = fields.iter().map(|(name, node)| {
		let type_identifier = generate_node(node);

		// TODO: this will probably need to call gen node to replace the Todo
		let name_identifier = format_ident!("{}", to_identifier(name));
		quote! { #name_identifier: #type_identifier }
	});

	// TODO: actually make this properly
	let struct_ident = format_ident!("Test");

	let struct_tokens = quote! {
		struct #struct_ident {
			#(#field_tokens),*
		}
	};

	println!("{struct_tokens}");

	quote! { #struct_ident }
}

lazy_static! {
	static ref RE_INVALID_CHARS: Regex = Regex::new(r"[^a-zA-Z0-9]").unwrap();
}

fn to_identifier(arg: &str) -> Cow<str> {
	RE_INVALID_CHARS.replace_all(arg, "")
}
