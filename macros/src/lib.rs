//! Provides proc_macros for toy-rpc.
//!
//! This is exported in `toy_rpc` as the `toy_rpc::macros` module.

// #[cfg(any(feature = "server", feature = "client"))]
mod util;
use darling::FromMeta;
// #[cfg(any(feature = "server", feature = "client"))]
use util::item_impl::*;
// #[cfg(any(feature = "server", feature = "client"))]
use util::item_trait::*;

// #[cfg(any(feature = "server", feature = "client"))]
pub(crate) const ATTR_EXPORT_METHOD: &str = "export_method";
#[cfg(feature = "server")]
pub(crate) const HANDLER_SUFFIX: &str = "handler";
#[cfg(feature = "server")]
pub(crate) const EXPORTED_TRAIT_SUFFIX: &str = "Handler";
#[cfg(all(
    feature = "client",
    feature = "runtime",
))]
pub(crate) const CLIENT_SUFFIX: &str = "Client";
#[cfg(all(
    feature = "client",
    feature = "runtime",
))]
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


// =============================================================================
// #[export_impl]
// =============================================================================

/// "Export" methods in the impl block with `#[export_method]` attribute. Methods without
/// the attribute will not be affected. This will also generate client stub.
///
/// When using with `#[async_trait]`, place `#[async_trait]` before `#[export_macro]`.
/// Under the hood, this macro generates method handlers. This macro implements the
/// `toy_rpc::util::RegisterService` trait, which returns the handler hashmap
/// when a service is registered on the server.
/// 
/// ### Note
/// 
/// - The default service name generated will be the same as the name of the struct.
///
/// ### Example - Export impl block
/// 
/// ```rust
/// struct Abacus { }
/// 
/// #[export_impl] // This will give a default service name of "Abacus"
/// impl Abacus {
///     #[export_method]
///     async fn subtract(&self, args(i32, i32)) -> Result<i32, String> {
///         // ...
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn export_impl(_attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // parse item
    let input = syn::parse_macro_input!(item as syn::ItemImpl);
    #[cfg(feature = "server")]
    let (handler_impl, names, handler_idents) = transform_impl(input.clone());

    // extract Self type and use it for construct Ident for handler HashMap
    #[cfg(any(
        feature = "server", 
        all(
            feature = "client",
            feature = "runtime"
        )
    ))]
    let ident = {
        let self_ty = &input.self_ty;
        match util::parse_impl_self_ty(self_ty) {
            Ok(i) => i,
            Err(err) => return err.to_compile_error().into(),
        }
    };
    #[cfg(feature = "server")]
    let register_service_impl = impl_register_service_for_struct(ident, names, handler_idents);

    // generate client stub
    #[cfg(all(feature = "client", feature = "runtime"))]
    let (client_ty, client_impl) = generate_service_client_for_struct(&ident, &input);
    #[cfg(all(feature = "client", feature = "runtime"))]
    let (stub_trait, stub_impl) = generate_client_stub_for_struct(&ident);

    let input = remove_export_attr_from_impl(input);
    #[cfg(feature = "server")]
    let handler_impl = remove_export_attr_from_impl(handler_impl);
    #[cfg(all(feature = "client", feature = "runtime"))]
    let client_impl = remove_export_attr_from_impl(client_impl);

    #[cfg(all(
        feature = "server",
        feature = "client",
        feature = "runtime"
    ))]
    let output = quote::quote! {
        #input
        #handler_impl
        #register_service_impl
        #client_ty
        #client_impl
        #stub_trait
        #stub_impl
    };
    #[cfg(all(
        not(feature = "server"),
        feature = "client",
        feature = "runtime"
    ))]
    let output = quote::quote! {
        #input
        #client_ty
        #client_impl
        #stub_trait
        #stub_impl
    };
    #[cfg(all(
        feature = "server",
        any(
            not(feature = "client"),
            not(feature = "runtime")
        )
    ))]
    let output = quote::quote! {
        #input
        #handler_impl
        #register_service_impl
    };
    #[cfg(all(
        not(feature = "server"),
        any(
            not(feature = "client"),
            not(feature = "runtime")
        )
    ))]
    let output = quote::quote! {
        #input
    };
    output.into()
}

// =============================================================================
// #[export_trait]
// =============================================================================


#[derive(Debug, darling::FromMeta)]
struct MacroArgs {
    #[darling(default)]
    impl_for_client: bool,
}

/// "Exports" methods defined in the trait with the `#[export_method]` attribute. 
/// Methods not marked with `#[export_method]` will not be affected. 
/// 
/// This macro should be used together with `#[export_trait_impl]` to allow conveniently 
/// register the struct that implements the service trait as a service. 
/// 
/// ## Note
/// 
/// - The default service name generated will be the same as the name of the trait. 
/// 
/// - This macro should be placed on the trait definition. 
/// 
/// ## Example
/// 
/// ```rust 
/// #[async_trait]
/// #[export_trait] // This will give a default service name of "Arith"
/// pub trait Arith {
///     // Mark method(s) to be "exported" with `#[export_method]` 
///     // in the definition.
///     #[export_method]
///     async fn add(&self, args(i32, i32)) -> Result<i32, String>;
/// }
/// ```
#[proc_macro_attribute]
pub fn export_trait(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let attr_args = syn::parse_macro_input!(attr as syn::AttributeArgs);
    let args = match MacroArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(err) => { return proc_macro::TokenStream::from(err.write_errors()); }
    };
    println!("{:?}", args);

    let input = syn::parse_macro_input!(item as syn::ItemTrait);
    #[cfg(feature = "server")]
    let (transformed_trait, 
        transformed_trait_impl,
        names, 
        handler_idents
    ) = transform_trait(input.clone());
    #[cfg(feature = "server")]
    let local_registry = impl_local_registry_for_trait(
        &input.ident, &transformed_trait.ident, names, handler_idents);

    #[cfg(all(feature = "client", feature = "runtime"))]
    let (client_ty, client_impl) = generate_service_client_for_trait(&input.ident, &input);
    #[cfg(all(feature = "client", feature = "runtime"))]
    let (stub_trait, stub_impl) = generate_client_stub_for_trait(&input.ident);

    #[cfg(all(feature = "client", feature = "runtime"))]
    let trait_impl = {
        let trait_impl = generate_trait_impl_for_client(&input);
        remove_export_attr_from_impl(trait_impl)
    };

    let input = remove_export_attr_from_trait(input);
    #[cfg(feature = "server")]
    let transformed_trait = remove_export_attr_from_trait(transformed_trait);
    #[cfg(feature = "server")]
    let transformed_trait_impl = remove_export_attr_from_impl(transformed_trait_impl);

    #[cfg(all(
        feature = "server",
        feature = "client",
        feature = "runtime"
    ))]
    let output = if args.impl_for_client {
        quote::quote! {
            #input
            #transformed_trait
            #transformed_trait_impl
            #local_registry
            #client_ty
            #client_impl
            #stub_trait
            #stub_impl
            #trait_impl
        }
    } else {
        quote::quote! {
            #input
            #transformed_trait
            #transformed_trait_impl
            #local_registry
            #client_ty
            #client_impl
            #stub_trait
            #stub_impl
        }
    };
    #[cfg(all(
        not(feature = "server"),
        feature = "client",
        feature = "runtime"
    ))]
    let output = if args.impl_for_client {
        quote::quote! {
            #input
            #client_ty
            #client_impl
            #stub_trait
            #stub_impl
            #trait_impl
        }
    } else {
        quote::quote! {
            #input
            #client_ty
            #client_impl
            #stub_trait
            #stub_impl
        }
    };
    #[cfg(all(
        feature = "server",
        any(
            not(feature = "client"),
            not(feature = "runtime")
        )
    ))]
    let output = quote::quote! {
        #input
        #transformed_trait
        #transformed_trait_impl
        #local_registry
    };
    #[cfg(all(
        not(feature = "server"),
        any(
            not(feature = "client"),
            not(feature = "runtime")
        )
    ))]
    let output = quote::quote! {
        #input
    };
    output.into()
}

/// This macro implements the `toy_rpc::util::RegisterService` trait to allow
/// convenient registration of the service. This should be used along with the
/// macro `#[export_trait]`. 
/// 
/// ## Note
/// 
/// - This macro should be placed on the impl block of the defined RPC service 
/// trait 
/// 
/// ## Example 
/// 
/// ```rust 
/// struct Abacus { }
/// 
/// #[async_trait]
/// #[export_trait_impl] // The default name will follow the name of the trait (ie. "Arith")
/// impl Arith  for Abacus {
///     // Notice that you do NOT mark the method with `#[export_method]`
///     // again
///     async fn add(&self, args(i32, i32)) -> Result<i32, String> {
///         // ...
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn export_trait_impl(_attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemImpl);
    #[cfg(feature = "server")]
    let trait_ident = get_trait_ident_from_item_impl(&input).unwrap();

    // extract Self type and use it for construct Ident for handler HashMap
    #[cfg(feature = "server")]
    let type_ident = {
        let self_ty = &input.self_ty;
        match util::parse_impl_self_ty(self_ty) {
            Ok(i) => i,
            Err(err) => return err.to_compile_error().into(),
        }
    };

    #[cfg(feature = "server")]
    let register_impl = impl_register_service_for_trait_impl(&trait_ident, type_ident);
    
    let input = remove_export_attr_from_impl(input);

    #[cfg(feature = "server")]
    let output = quote::quote! {
        #input
        #register_impl
    };
    #[cfg(not(feature = "server"))]
    let output = quote::quote! {
        #input
    };
    output.into()
}