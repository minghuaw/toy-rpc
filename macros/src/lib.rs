//! Provides proc_macros for toy-rpc.
//!
//! This is exported in `toy_rpc` as the `toy_rpc::macros` module.

mod util;

#[cfg(any(feature = "server", feature = "client"))]
pub(crate) const ATTR_EXPORT_METHOD: &str = "export_method";
#[cfg(feature = "server")]
pub(crate) const HANDLER_SUFFIX: &str = "handler";
#[cfg(feature = "client")]
pub(crate) const CLIENT_SUFFIX: &str = "Client";
#[cfg(feature = "client")]
pub(crate) const CLIENT_STUB_SUFFIX: &str = "ClientStub";

/// A macro that impls serde::Deserializer by simply calling the
/// corresponding functions of the inner deserializer
#[proc_macro]
pub fn impl_inner_deserializer(_: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let output = quote::quote! {
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

/// Export methods in the impl block with #[export_method] attribute. Methods without
/// the attribute will not be affected. This will also generate client stub.
///
/// When using with `#[async_trait]`, place `#[async_trait]` before `#[export_macro]`.
/// Under the hood, this macro generates method wrappers which are added to a
/// lazy static hashmap of handlers. This macro implements the
/// `toy_rpc::util::RegisterService` trait, which returns the handler hashmap
/// when a service is registered on the server.
///
/// Example - Export impl block
///
/// ```rust
/// pub struct Example { }
///
/// #[export_impl]
/// impl Example {
///     #[export_method]
///     async fn exported_method(&self, args: ()) -> Result<String, String> {
///         Ok("This is an exported method".to_string())
///     }
///
///     async fn not_exported_method(&self, args: ()) -> Result<String, String> {
///         Ok("This method is NOT exported".to_string())
///     }
/// }
/// ```
///
/// Example - Export trait impl block
///
/// ```rust
/// use async_trait::async_trait;
/// use toy_rpc::macros::export_impl;
///
/// #[async_trait]
/// pub trait Rpc {
///     async fn increment(&self, arg: i32) -> Result<i32, String>;
/// }
///
/// pub struct Foo { }
///
/// #[async_trait] // make sure `#[async_trait]` is declared before `#[export_impl]`
/// #[export_impl]
/// impl Rpc for Foo {
///     #[export_method]
///     async fn increment(&self, arg: i32) -> Result<i32, String> {
///         Ok(arg + 1)
///     }
/// }
/// ```
///
/// Example - use client stub
///
/// ```rust
/// mod rpc {
///     // service state
///     pub struct Foo {
///         pub id: i32
///     }       
///
///     // service impl
///     #[export_impl]
///     impl Foo {
///         pub async fn get_id(&self, _: ()) -> Result<i32, String> {
///             Ok(self.id)
///         }
///     }
/// }
///
/// use toy_rpc::Client;
/// use rpc::*;
///
/// #[async_std::main]
/// async fn main() {
///     let addr = "127.0.0.1:23333";
///     let client = Client::dial(addr).await.unwrap();
///
///     // assume the `Foo` service is registered on the server
///     let reply = client
///                 .foo()
///                 .get_id(())
///                 .await
///                 .unwrap();
/// }
///
/// ```
#[cfg(all(feature="server", feature="client"))]
#[proc_macro_attribute]
pub fn export_impl(_attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // parse item
    let input = syn::parse_macro_input!(item as syn::ItemImpl);
    let (handler_impl, names, fn_idents) = util::transform_impl(input.clone());

    // extract Self type and use it for construct Ident for handler HashMap
    let self_ty = &input.self_ty;
    let ident = match util::parse_impl_self_ty(self_ty) {
        Ok(i) => i,
        Err(err) => return err.to_compile_error().into(),
    };

    // generate client stub
    let (client_ty, client_impl, stub_trait, stub_impl) =
        util::generate_client_stub(&ident, input.clone());

    let register_service_impl = util::generate_register_service_impl(ident, names, fn_idents);

    let input = util::remove_export_method_attr(input);
    let client_impl = util::remove_export_method_attr(client_impl);
    let handler_impl = util::remove_export_method_attr(handler_impl);

    let output = quote::quote! {
        #input
        #handler_impl
        #client_ty
        #client_impl
        #stub_trait
        #stub_impl
        #register_service_impl
    };
    output.into()
}

#[cfg(all(feature="server", not(feature="client")))]
#[proc_macro_attribute]
pub fn export_impl(_attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // parse item
    let input = syn::parse_macro_input!(item as syn::ItemImpl);
    let (handler_impl, names, fn_idents) = util::transform_impl(input.clone());

    // extract Self type and use it for construct Ident for handler HashMap
    let self_ty = &input.self_ty;
    let ident = match util::parse_impl_self_ty(self_ty) {
        Ok(i) => i,
        Err(err) => return err.to_compile_error().into(),
    };

    let register_service_impl = util::generate_register_service_impl(ident, names, fn_idents);

    let input = util::remove_export_method_attr(input);
    let handler_impl = util::remove_export_method_attr(handler_impl);

    let output = quote::quote! {
        #input
        #handler_impl
        #register_service_impl
    };
    output.into()
}

#[cfg(all(not(feature="server"), feature="client"))]
#[proc_macro_attribute]
pub fn export_impl(_attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // parse item
    let input = syn::parse_macro_input!(item as syn::ItemImpl);
    // let (handler_impl, names, fn_idents) = util::transform_impl(input.clone());

    // extract Self type and use it for construct Ident for handler HashMap
    let self_ty = &input.self_ty;
    let ident = match util::parse_impl_self_ty(self_ty) {
        Ok(i) => i,
        Err(err) => return err.to_compile_error().into(),
    };

    // generate client stub
    let (client_ty, client_impl, stub_trait, stub_impl) =
        util::generate_client_stub(&ident, input.clone());

    // let register_service_impl = util::generate_register_service_impl(ident, names, fn_idents);

    let input = util::remove_export_method_attr(input);
    let client_impl = util::remove_export_method_attr(client_impl);
    // let handler_impl = util::remove_export_method_attr(handler_impl);

    let output = quote::quote! {
        #input
        #client_ty
        #client_impl
        #stub_trait
        #stub_impl
    };
    output.into()
}