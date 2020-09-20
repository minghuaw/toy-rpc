use std::sync::Mutex;
use std::collections::HashMap;
use async_std::sync::Arc;
use proc_macro::TokenStream;
use quote::quote;
use lazy_static::lazy_static;
use syn::{
    parse_macro_input,
    ItemStruct,
    ItemImpl,
    ItemFn,
    Ident
};

use toy_rpc_definitions;

const SERVICE_PREFIX: &str = "static_toy_rpc_service";
const ATTR_EXPORT_METHOD: &str = "export_method";

/// A macro that impls serde::Deserializer by simply calling the 
/// corresponding functions of the inner deserializer
#[proc_macro]
pub fn impl_inner_deserializer(_: TokenStream) -> TokenStream {
    let output = quote! {
        fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_any(visitor)
        }

        fn deserialize_bool<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_bool(visitor)
        }

        fn deserialize_byte_buf<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_byte_buf(visitor)
        }

        fn deserialize_bytes<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_bytes(visitor)
        }

        fn deserialize_char<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_char(visitor)
        }

        fn deserialize_enum<V>(
                mut self,
                name: &'static str,
                variants: &'static [&'static str],
                visitor: V,
            ) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_enum(name, variants, visitor)
        }

        fn deserialize_f32<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_f32(visitor)
        }

        fn deserialize_f64<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_f64(visitor)
        }

        fn deserialize_i16<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_i16(visitor)
        }

        fn deserialize_i32<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_i32(visitor)
        }

        fn deserialize_i64<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_i64(visitor)
        }

        fn deserialize_i8<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_i8(visitor)
        }

        fn deserialize_identifier<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_identifier(visitor)
        }

        fn deserialize_ignored_any<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_ignored_any(visitor)
        }

        fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_map(visitor)
        }

        fn deserialize_newtype_struct<V>(
                mut self,
                name: &'static str,
                visitor: V,
            ) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_newtype_struct(name, visitor)
        }

        fn deserialize_option<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_option(visitor)
        }

        fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_seq(visitor)
        }

        fn deserialize_str<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_str(visitor)
        }

        fn deserialize_string<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_string(visitor)
        }

        fn deserialize_struct<V>(
                mut self,
                name: &'static str,
                fields: &'static [&'static str],
                visitor: V,
            ) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_struct(name, fields, visitor)
        }

        fn deserialize_tuple<V>(mut self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_tuple(len, visitor)
        }

        fn deserialize_tuple_struct<V>(
                mut self,
                name: &'static str,
                len: usize,
                visitor: V,
            ) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_tuple_struct(name, len, visitor)
        }

        fn deserialize_u16<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_u16(visitor)
        }

        fn deserialize_u32<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_u32(visitor)
        }

        fn deserialize_u64<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_u64(visitor)
        }

        fn deserialize_u8<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_u8(visitor)
        }

        fn deserialize_unit<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_unit(visitor)
        }

        fn deserialize_unit_struct<V>(
                mut self,
                name: &'static str,
                visitor: V,
            ) -> Result<V::Value, Self::Error>
        where
                V: Visitor<'de> {
            self.inner.deserialize_unit_struct(name, visitor)
        }
    };

    output.into()
}

#[proc_macro_attribute]
pub fn export_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    // parse attr
    // let mut args = parse_macro_input!(attr as syn::AttributeArgs);
    // println!("impl attr: {:?}", args);

    // parse item
    let mut input = parse_macro_input!(item as syn::ItemImpl);
    let mut idents: Vec<syn::Ident> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    let ty = &input.self_ty;
    
    let _ = input.items.iter_mut()
        // first filter out method
        .filter_map(|item| {
            match item {
                syn::ImplItem::Method(f) => {
                    Some(f)
                },
                _ => None
            }
        })
        // find whether function has attributes
        .filter(|f| {
            // println!("{:?}", &f.attrs);
            f.attrs.iter()
                .any(|attr| {
                    attr.path.segments.iter()
                        .any(|seg| {
                            seg.ident == ATTR_EXPORT_METHOD 
                        })
                })
        })
        // append ident and names of function with attribute
        .for_each(|f| {
            idents.push(f.sig.ident.clone());
            names.push(f.sig.ident.to_string());
            
            // clear the attributes for now
            f.attrs.retain(|attr| {
                !attr.path.segments.iter()
                    .any(|seg| {
                        seg.ident == ATTR_EXPORT_METHOD 
                    })
            })
        });

    println!("idents: {:?}", idents);

    let output = quote! {
        #input
    };
    output.into()
}

#[proc_macro_attribute]
pub fn export_struct(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let ident = &input.ident;
    let static_name = format!("{}_{}", SERVICE_PREFIX, ident);
    let static_ident = Ident::new(&static_name, ident.span());

    // initialize 
    let static_map_output = quote! {
        lazy_static::lazy_static! {
            static ref #static_ident: 
                std::sync::Mutex<std::collections::HashMap<&'static str, async_std::sync::Arc<toy_rpc_definitions::Handler<#ident>>>> 
            = {
                Mutex::new(
                    HashMap::new()
                )
            };
        }
    };

    let output = quote! {
        #input

        #static_map_output
    };

    output.into()
}

// #[proc_macro_attribute]
// pub fn export_method(attr: TokenStream, item: TokenStream) -> TokenStream {
//     item
// }