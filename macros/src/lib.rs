//! Provides proc_macros for toy-rpc.
//!
//! This is exported in `toy_rpc` as the `toy_rpc::macros` module.

use proc_macro::TokenStream;
// use quote::{quote};

// #[cfg(any(feature = "server", feature = "client"))]
// use quote::ToTokens;
// #[cfg(any(feature = "server", feature = "client"))]
// use syn::{parse_macro_input, parse_quote, GenericArgument, Ident};

#[cfg(feature = "server")]
const SERVICE_PREFIX: &str = "STATIC_TOY_RPC_SERVICE";
#[cfg(any(feature = "server", feature = "client"))]
const ATTR_EXPORT_METHOD: &str = "export_method";
#[cfg(feature = "server")]
const HANDLER_SUFFIX: &str = "handler";
#[cfg(feature = "client")]
const CLIENT_SUFFIX: &str = "Client";
#[cfg(feature = "client")]
const CLIENT_STUB_SUFFIX: &str = "ClientStub";

/// A macro that impls serde::Deserializer by simply calling the
/// corresponding functions of the inner deserializer
#[proc_macro]
pub fn impl_inner_deserializer(_: TokenStream) -> TokenStream {
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
pub fn export_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // parse item
    let input = syn::parse_macro_input!(item as syn::ItemImpl);
    let (handler_impl, names, fn_idents) = transform_impl(input.clone());

    // extract Self type and use it for construct Ident for handler HashMap
    let self_ty = &input.self_ty;
    let ident = match parse_impl_self_ty(self_ty) {
        Ok(i) => i,
        Err(err) => return err.to_compile_error().into(),
    };
    let static_name = format!("{}_{}", SERVICE_PREFIX, ident.to_string().to_uppercase());
    let static_ident = syn::Ident::new(&static_name, ident.span());

    // generate client stub
    let (client_ty, client_impl, stub_trait, stub_impl) =
        generate_client_stub(&ident, input.clone());

    let lazy = quote::quote! {
        // store the handler functions in a gloabl lazy hashmap
        toy_rpc::lazy_static::lazy_static! {
            pub static ref #static_ident:
                std::collections::HashMap<&'static str, toy_rpc::service::AsyncHandler<#self_ty>>
                = {
                    let mut map: std::collections::HashMap<&'static str, toy_rpc::service::AsyncHandler<#self_ty>>
                        = std::collections::HashMap::new();
                    #(map.insert(#names, #self_ty::#fn_idents);)*;
                    map
                };
        }
    };

    let register_service_impl = generate_register_service_impl(ident);

    let input = remove_export_method_attr(input);
    let client_impl = remove_export_method_attr(client_impl);
    let handler_impl = remove_export_method_attr(handler_impl);

    let output = quote::quote! {
        #input
        #handler_impl
        #client_ty
        #client_impl
        #stub_trait
        #stub_impl
        #lazy
        #register_service_impl
    };
    output.into()
}

#[cfg(all(feature="server", not(feature="client")))]
#[proc_macro_attribute]
pub fn export_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // parse item
    let input = syn::parse_macro_input!(item as syn::ItemImpl);
    let (handler_impl, names, fn_idents) = transform_impl(input.clone());

    // extract Self type and use it for construct Ident for handler HashMap
    let self_ty = &input.self_ty;
    let ident = match parse_impl_self_ty(self_ty) {
        Ok(i) => i,
        Err(err) => return err.to_compile_error().into(),
    };
    let static_name = format!("{}_{}", SERVICE_PREFIX, ident.to_string().to_uppercase());
    let static_ident = syn::Ident::new(&static_name, ident.span());

    let lazy = quote::quote! {
        // store the handler functions in a gloabl lazy hashmap
        toy_rpc::lazy_static::lazy_static! {
            pub static ref #static_ident:
                std::collections::HashMap<&'static str, toy_rpc::service::AsyncHandler<#self_ty>>
                = {
                    let mut map: std::collections::HashMap<&'static str, toy_rpc::service::AsyncHandler<#self_ty>>
                        = std::collections::HashMap::new();
                    #(map.insert(#names, #self_ty::#fn_idents);)*;
                    map
                };
        }
    };

    let register_service_impl = generate_register_service_impl(ident);

    let input = remove_export_method_attr(input);
    let client_impl = remove_export_method_attr(client_impl);
    let handler_impl = remove_export_method_attr(handler_impl);

    let output = quote::quote! {
        #input
        #handler_impl
        #lazy
        #register_service_impl
    };
    output.into()
}

#[cfg(all(not(feature="server"), feature="client"))]
#[proc_macro_attribute]
pub fn export_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // parse item
    let input = syn::parse_macro_input!(item as syn::ItemImpl);
    let (handler_impl, names, fn_idents) = transform_impl(input.clone());

    // extract Self type and use it for construct Ident for handler HashMap
    let self_ty = &input.self_ty;
    let ident = match parse_impl_self_ty(self_ty) {
        Ok(i) => i,
        Err(err) => return err.to_compile_error().into(),
    };
    let static_name = format!("{}_{}", SERVICE_PREFIX, ident.to_string().to_uppercase());
    let static_ident = syn::Ident::new(&static_name, ident.span());

    // generate client stub
    let (client_ty, client_impl, stub_trait, stub_impl) =
        generate_client_stub(&ident, input.clone());

    let lazy = quote::quote! {
        // store the handler functions in a gloabl lazy hashmap
        toy_rpc::lazy_static::lazy_static! {
            pub static ref #static_ident:
                std::collections::HashMap<&'static str, toy_rpc::service::AsyncHandler<#self_ty>>
                = {
                    let mut map: std::collections::HashMap<&'static str, toy_rpc::service::AsyncHandler<#self_ty>>
                        = std::collections::HashMap::new();
                    #(map.insert(#names, #self_ty::#fn_idents);)*;
                    map
                };
        }
    };

    let register_service_impl = generate_register_service_impl(ident);

    let input = remove_export_method_attr(input);
    let client_impl = remove_export_method_attr(client_impl);
    let handler_impl = remove_export_method_attr(handler_impl);

    let output = quote::quote! {
        #input
        #client_ty
        #client_impl
        #stub_trait
        #stub_impl
    };
    output.into()
}

/// transform impl block to meet the signature of service function
///
/// Example
///
/// ```rust
/// pub struct Foo { }
///
/// #[export_impl]
/// impl Foo {
///     #[export_method]
///     async fn increment(&self, arg: i32) -> Result<i32, String> {
///         Ok(arg + 1)
///     }
/// }
/// ```
///
/// will generate the following impl
///
/// ```rust
/// pub struct Foo { }
///
/// impl Foo {
///     async fn increment(&self, arg: i32) -> Result<i32, String> {
///         Ok(arg + 1)
///     }
/// }
/// pub fn increment_handler(
///     self: std::sync::Arc<Self>,
///     mut deserializer: Box<dyn toy_rpc::erased_serde::Deserializer<'static> + Send>,
/// ) -> toy_rpc::service::HandlerResultFut {
///     Box::pin(async move {
///     let req: i32 = toy_rpc::erased_serde::deserialize(&mut deserializer)
///         .map_err(|e| toy_rpc::error::Error::ParseError(Box::new(e)))?;
///     let res = self
///         .increment(req) // executes the RPC method
///         .await
///         .map(|r| {
///             Box::new(r)
///                 as Box<dyn toy_rpc::erased_serde::Serialize + Send + Sync + 'static>
///         })
///         .map_err(|e| toy_rpc::error::Error::ExecutionError(e.to_string()));
///     res
///     })
/// }
/// ```
#[cfg(feature = "server")]
fn transform_impl(input: syn::ItemImpl) -> (syn::ItemImpl, Vec<String>, Vec<syn::Ident>) {
    let mut names = Vec::new();
    let mut idents = Vec::new();
    let mut output = filter_exported_methods(input);

    output.trait_ = None;
    output
        .items
        .iter_mut()
        // first filter out method
        .filter_map(|item| match item {
            syn::ImplItem::Method(f) => Some(f),
            _ => None,
        })
        .for_each(|f| {
            names.push(f.sig.ident.to_string());
            transform_method(f);
            idents.push(f.sig.ident.clone());
        });

    (output, names, idents)
}

/// transform method to meet the signature of service function
#[cfg(feature = "server")]
fn transform_method(f: &mut syn::ImplItemMethod) {
    // change function ident
    let ident = f.sig.ident.clone();
    let concat_name = format!("{}_{}", &ident.to_string(), HANDLER_SUFFIX);
    let handler_ident = syn::Ident::new(&concat_name, ident.span());

    // change asyncness
    f.sig.asyncness = None;

    // transform function request type
    if let syn::FnArg::Typed(pt) = f.sig.inputs.last().unwrap() {
        let req_ty = &pt.ty;

        f.block = syn::parse_quote!({
            Box::pin(
                async move {
                    let req: #req_ty = toy_rpc::erased_serde::deserialize(&mut deserializer)
                        .map_err(|e| toy_rpc::error::Error::ParseError(Box::new(e)))?;
                    let res = self.#ident(req).await
                        .map(|r| Box::new(r) as Box<dyn toy_rpc::erased_serde::Serialize + Send + Sync + 'static>)
                        .map_err(|e| toy_rpc::error::Error::ExecutionError(e.to_string()));
                    res
                }
            )
        });

        f.sig.inputs = syn::parse_quote!(
            self: std::sync::Arc<Self>, mut deserializer: Box<dyn toy_rpc::erased_serde::Deserializer<'static> + Send>
        );

        f.sig.output = syn::parse_quote!(
            -> toy_rpc::service::HandlerResultFut
        );
    };

    f.sig.ident = handler_ident;
}

/// remove #[export_method] attribute
#[cfg(feature = "server")]
fn remove_export_method_attr(mut input: syn::ItemImpl) -> syn::ItemImpl {
    input
        .items
        .iter_mut()
        // first filter out method
        .filter_map(|item| match item {
            syn::ImplItem::Method(f) => Some(f),
            _ => None,
        })
        .for_each(|f| {
            // clear the attributes for now
            f.attrs.retain(|attr| {
                let ident = attr.path.get_ident().unwrap();
                ident != ATTR_EXPORT_METHOD
            })
        });

    input
}

/// Generate client stub for the service impl block
#[cfg(feature = "client")]
fn generate_client_stub(
    ident: &syn::Ident,
    input: syn::ItemImpl,
) -> (syn::Item, syn::ItemImpl, syn::Item, syn::ItemImpl) {
    let concat_name = format!("{}{}", &ident.to_string(), CLIENT_SUFFIX);
    let client_ident = syn::Ident::new(&concat_name, ident.span());

    let client_struct = syn::parse_quote!(
        pub struct #client_ident<'c> {
            client: &'c toy_rpc::client::Client<toy_rpc::client::Connected>,
            service_name: &'c str,
        }
    );

    let client_impl = client_stub_impl(ident, &client_ident, input);

    let concat_name = format!("{}{}", &ident.to_string(), CLIENT_STUB_SUFFIX);
    let stub_ident = syn::Ident::new(&concat_name, ident.span());
    let stub_fn = parse_stub_fn_name(ident);

    let stub_trait = syn::parse_quote!(
        pub trait #stub_ident {
            fn #stub_fn<'c>(&'c self) -> #client_ident;
        }
    );

    let service_name = ident.to_string();
    let stub_impl: syn::ItemImpl = syn::parse_quote!(
        impl #stub_ident for toy_rpc::client::Client<toy_rpc::client::Connected> {
            fn #stub_fn<'c>(&'c self) -> #client_ident {
                #client_ident {
                    client: self,
                    service_name: #service_name,
                }
            }
        }
    );

    (client_struct, client_impl, stub_trait, stub_impl)
}

/// Generate client stub implementation that allows, conveniently, type checking with the RPC argument
#[cfg(feature = "client")]
fn client_stub_impl(
    service_ident: &syn::Ident,
    client_ident: &syn::Ident,
    input: syn::ItemImpl,
) -> syn::ItemImpl {
    let mut input = filter_exported_methods(input);
    let mut generated_items: Vec<syn::ImplItem> = Vec::new();
    input.trait_ = None;
    input
        .items
        .iter_mut()
        // first filter out method
        .filter_map(|item| match item {
            syn::ImplItem::Method(f) => Some(f),
            _ => None,
        })
        .for_each(|f| {
            if let Some(gen) = generate_client_stub_method(service_ident, f) {
                generated_items.push(syn::ImplItem::Method(gen));
            }
        });

    let mut output: syn::ItemImpl = syn::parse_quote!(
        impl<'c> #client_ident<'c> {

        }
    );

    output.items = generated_items;
    output
}

#[cfg(feature = "client")]
fn generate_client_stub_method(
    service_ident: &syn::Ident,
    f: &mut syn::ImplItemMethod,
) -> Option<syn::ImplItemMethod> {
    if let syn::FnArg::Typed(pt) = f.sig.inputs.last().unwrap() {
        let fn_ident = &f.sig.ident;
        let req_ty = &pt.ty;

        if let syn::ReturnType::Type(_, ret_ty) = f.sig.output.clone() {
            let ok_ty = get_ok_ident_from_type(ret_ty)?;
            return Some(generate_client_stub_method_impl(
                service_ident,
                fn_ident,
                &req_ty,
                &ok_ty,
            ));
        }
    }

    None
}

#[cfg(feature = "client")]
fn generate_client_stub_method_impl(
    service_ident: &syn::Ident,
    fn_ident: &syn::Ident,
    req_ty: &syn::Type,
    ok_ty: &syn::GenericArgument,
) -> syn::ImplItemMethod {
    let service = service_ident.to_string();
    let method = fn_ident.to_string();
    let service_method = format!("{}.{}", service, method);
    syn::parse_quote!(
        pub fn #fn_ident<A>(&'c self, args: A) -> toy_rpc::client::Call<#ok_ty>
        where
            A: std::borrow::Borrow<#req_ty> + Send + Sync + toy_rpc::serde::Serialize + 'static,
        {
            self.client.call(#service_method, args)
        }
    )
}

#[cfg(feature = "client")]
fn get_ok_ident_from_type(ty: Box<syn::Type>) -> Option<syn::GenericArgument> {
    let ty = Box::leak(ty);
    let arg = syn::GenericArgument::Type(ty.to_owned());
    recursively_get_result_from_generic_arg(&arg)
}

#[cfg(feature = "client")]
fn recursively_get_result_from_generic_arg(arg: &syn::GenericArgument) -> Option<syn::GenericArgument> {
    match &arg {
        syn::GenericArgument::Type(ty) => {
            recusively_get_result_from_type(&ty)
        }
        syn::GenericArgument::Binding(binding) => {
            recusively_get_result_from_type(&binding.ty)
        }
        _ => None,
    }
}

#[cfg(feature = "client")]
fn recusively_get_result_from_type(ty: &syn::Type) -> Option<syn::GenericArgument> {
    match ty {
        syn::Type::Path(ref path) => {
            let ident = &path.path.segments.last()?.ident.to_string()[..];
            match &path.path.segments.last()?.arguments {
                syn::PathArguments::AngleBracketed(angle_bracket) => {
                    if ident == "Result" {
                        return angle_bracket.args.first().map(|g| g.to_owned());
                    }
                    recursively_get_result_from_generic_arg(angle_bracket.args.first()?)
                }
                _ => None,
            }
        }
        syn::Type::TraitObject(ref tobj) => {
            if let syn::TypeParamBound::Trait(bound) = tobj.bounds.first()? {
                match &bound.path.segments.last()?.arguments {
                    syn::PathArguments::AngleBracketed(angle_bracket) => {
                        return recursively_get_result_from_generic_arg(angle_bracket.args.first()?)
                    }
                    _ => return None,
                }
            }
            None
        }
        _ => None,
    }
}

/// Generate implementation of the `toy_rpc::util::RegisterService` trait.
///
/// The static hashmap of handlers will be returned by `handlers()` method.
/// The service struct name will be returned by `default_name()` method.
///
#[cfg(feature = "server")]
fn generate_register_service_impl(ident: &syn::Ident) -> impl quote::ToTokens {
    let name = ident.to_string();
    let static_name = format!("{}_{}", SERVICE_PREFIX, &name.to_uppercase());
    let static_ident = syn::Ident::new(&static_name, ident.span());
    let ret = quote::quote! {
        impl toy_rpc::util::RegisterService for #ident {
            fn handlers() -> &'static std::collections::HashMap<&'static str, toy_rpc::service::AsyncHandler<Self>> {
                &*#static_ident
            }

            fn default_name() -> &'static str {
                #name
            }
        }
    };

    ret
}

#[cfg(any(feature = "server", feature = "client"))]
fn parse_impl_self_ty(self_ty: &syn::Type) -> Result<&syn::Ident, syn::Error> {
    match self_ty {
        syn::Type::Path(tp) => Ok(&tp.path.segments[0].ident),
        _ => Err(syn::Error::new_spanned(
            quote::quote! {},
            "Compile Error: Self type",
        )),
    }
}

#[cfg(feature = "client")]
fn parse_stub_fn_name(ident: &syn::Ident) -> syn::Ident {
    let mut output_fn = String::new();
    for c in ident.to_string().chars() {
        if c.is_uppercase() {
            output_fn.push('_');
            output_fn.push_str(&c.to_lowercase().to_string());
        } else {
            output_fn.push(c);
        }
    }
    output_fn = output_fn
        .trim_start_matches('_')
        .trim_end_matches('_')
        .into();

    syn::Ident::new(&output_fn, ident.span())
}

#[cfg(any(feature = "server", feature = "client"))]
fn filter_exported_methods(input: syn::ItemImpl) -> syn::ItemImpl {
    let mut output = input;
    output.items.retain(|item| match item {
        syn::ImplItem::Method(f) => {
            let is_exported = f.attrs.iter().any(|attr| {
                let ident = attr.path.get_ident().unwrap();
                ident == ATTR_EXPORT_METHOD
            });

            is_exported
        }
        _ => false,
    });
    output
}
