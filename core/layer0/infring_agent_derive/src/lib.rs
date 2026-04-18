use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Item, ItemFn, ItemStruct};

fn attr_name_override(attr: &TokenStream) -> Option<String> {
    let raw = attr.to_string();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some((_, rhs)) = trimmed.split_once('=') {
        return Some(rhs.trim().trim_matches('"').to_string());
    }
    Some(trimmed.trim_matches('"').to_string())
}

fn expand_tool_for_fn(item: ItemFn, tool_name: String) -> TokenStream {
    let fn_ident = item.sig.ident.clone();
    let schema_ident = format_ident!("{fn_ident}_tool_schema");
    let receipt_ident = format_ident!("{fn_ident}_tool_receipt");
    let out = quote! {
        #item

        pub fn #schema_ident() -> ::serde_json::Value {
            ::serde_json::json!({
                "type": "infring_tool",
                "name": #tool_name,
                "kind": "fn",
                "schema": {
                    "input": {"type": "object"},
                    "output": {"type": "object"}
                }
            })
        }

        pub fn #receipt_ident(status: &str, duration_ms: u64, error_code: ::std::option::Option<&str>) -> ::serde_json::Value {
            ::serde_json::json!({
                "type": "tool_receipt",
                "tool": #tool_name,
                "status": status,
                "duration_ms": duration_ms,
                "error_code": error_code.unwrap_or(""),
            })
        }
    };
    out.into()
}

fn expand_tool_for_struct(item: ItemStruct, tool_name: String) -> TokenStream {
    let ident = item.ident.clone();
    let out = quote! {
        #item

        impl #ident {
            pub fn infring_tool_name() -> &'static str {
                #tool_name
            }

            pub fn infring_tool_schema() -> ::serde_json::Value {
                ::serde_json::json!({
                    "type": "infring_tool",
                    "name": #tool_name,
                    "kind": "struct",
                    "schema": {
                        "input": {"type": "object"},
                        "output": {"type": "object"}
                    }
                })
            }

            pub fn infring_tool_receipt(status: &str, duration_ms: u64, error_code: ::std::option::Option<&str>) -> ::serde_json::Value {
                ::serde_json::json!({
                    "type": "tool_receipt",
                    "tool": #tool_name,
                    "status": status,
                    "duration_ms": duration_ms,
                    "error_code": error_code.unwrap_or(""),
                })
            }
        }
    };
    out.into()
}

fn expand_agent_for_fn(item: ItemFn, agent_name: String) -> TokenStream {
    let fn_ident = item.sig.ident.clone();
    let schema_ident = format_ident!("{fn_ident}_agent_schema");
    let receipt_ident = format_ident!("{fn_ident}_agent_receipt");
    let out = quote! {
        #item

        pub fn #schema_ident() -> ::serde_json::Value {
            ::serde_json::json!({
                "type": "infring_agent",
                "name": #agent_name,
                "kind": "fn",
                "contract": {
                    "initial_prompt": {"type": "string"},
                    "lifespan_seconds": {"type": "integer"},
                    "permissions": {"type": "object"}
                }
            })
        }

        pub fn #receipt_ident(status: &str, provider: &str, tool_count: usize) -> ::serde_json::Value {
            ::serde_json::json!({
                "type": "agent_receipt",
                "agent": #agent_name,
                "status": status,
                "provider": provider,
                "tool_count": tool_count,
            })
        }
    };
    out.into()
}

fn expand_agent_for_struct(item: ItemStruct, agent_name: String) -> TokenStream {
    let ident = item.ident.clone();
    let out = quote! {
        #item

        impl #ident {
            pub fn infring_agent_name() -> &'static str {
                #agent_name
            }

            pub fn infring_agent_schema() -> ::serde_json::Value {
                ::serde_json::json!({
                    "type": "infring_agent",
                    "name": #agent_name,
                    "kind": "struct",
                    "contract": {
                        "initial_prompt": {"type": "string"},
                        "lifespan_seconds": {"type": "integer"},
                        "permissions": {"type": "object"}
                    }
                })
            }

            pub fn infring_agent_receipt(status: &str, provider: &str, tool_count: usize) -> ::serde_json::Value {
                ::serde_json::json!({
                    "type": "agent_receipt",
                    "agent": #agent_name,
                    "status": status,
                    "provider": provider,
                    "tool_count": tool_count,
                })
            }
        }
    };
    out.into()
}

#[proc_macro_attribute]
pub fn infring_tool(attr: TokenStream, item: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(item as Item);
    let explicit = attr_name_override(&attr);
    match parsed {
        Item::Fn(function_item) => {
            let name = explicit.unwrap_or_else(|| function_item.sig.ident.to_string());
            expand_tool_for_fn(function_item, name)
        }
        Item::Struct(struct_item) => {
            let name = explicit.unwrap_or_else(|| struct_item.ident.to_string());
            expand_tool_for_struct(struct_item, name)
        }
        _ => quote! {
            compile_error!("#[infring_tool] supports fn or struct items only.");
        }
        .into(),
    }
}

#[proc_macro_attribute]
pub fn infring_agent(attr: TokenStream, item: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(item as Item);
    let explicit = attr_name_override(&attr);
    match parsed {
        Item::Fn(function_item) => {
            let name = explicit.unwrap_or_else(|| function_item.sig.ident.to_string());
            expand_agent_for_fn(function_item, name)
        }
        Item::Struct(struct_item) => {
            let name = explicit.unwrap_or_else(|| struct_item.ident.to_string());
            expand_agent_for_struct(struct_item, name)
        }
        _ => quote! {
            compile_error!("#[infring_agent] supports fn or struct items only.");
        }
        .into(),
    }
}

