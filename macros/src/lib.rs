//! Provides proc_macros for toy-rpc.

use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{GenericArgument, Ident, parse_macro_input, parse_quote};

const SERVICE_PREFIX: &str = "STATIC_TOY_RPC_SERVICE";
const ATTR_EXPORT_METHOD: &str = "export_method";
const HANDLER_SUFFIX: &str = "handler";
const CLIENT_SUFFIX: &str = "Client";
const CLIENT_STUB_SUFFIX: &str = "ClientStub";

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

/// Export methods in the impl block with #[export_method] attribute. Methods without
/// the attribute will not be affected. This will also generate client stub.
///
/// When using with `#[async_trait]`, place `#[async_trait]` before `#[export_macro]`. 
///
/// Example - Export impl block
///
/// ```rust
/// struct ExampleService { }
///
/// #[export_impl]
/// impl ExampleService {
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
///     // assume the `Foo` service is registered as "foo_service" 
///     // on the server
///     let reply = client.foo("foo_service").get_id(()).await.unwrap();
/// }
/// 
/// ```
#[proc_macro_attribute]
pub fn export_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // parse item
    let input = parse_macro_input!(item as syn::ItemImpl);
    let (handler_impl, names, fn_idents) = transform_impl(input.clone());

    // extract Self type and use it for construct Ident for handler HashMap
    let self_ty = &input.self_ty;
    let ident = match parse_impl_self_ty(self_ty) {
        Ok(i) => i,
        Err(err) => return err.to_compile_error().into(),
    };
    let static_name = format!("{}_{}", SERVICE_PREFIX, ident.to_string().to_uppercase());
    let static_ident = Ident::new(&static_name, ident.span());

    // generate client stub
    let (client_ty, client_impl, stub_trait, stub_impl) = generate_client_stub(&ident, input.clone());

    let lazy = quote! {
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

    let output = quote! {
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

/// transform impl block to meet the signature of service function
fn transform_impl(input: syn::ItemImpl) -> (syn::ItemImpl, Vec<String>, Vec<Ident>) {
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
fn transform_method(f: &mut syn::ImplItemMethod) {
    // change function ident
    let ident = f.sig.ident.clone();
    let concat_name = format!("{}_{}", &ident.to_string(), HANDLER_SUFFIX);
    let handler_ident = Ident::new(&concat_name, ident.span());

    // change asyncness
    f.sig.asyncness = None;

    // transform function request type
    if let syn::FnArg::Typed(pt) = f.sig.inputs.last().unwrap() {
        let req_ty = &pt.ty;

        f.block = parse_quote!({
            Box::pin(
                async move {
                    let req: #req_ty = toy_rpc::erased_serde::deserialize(&mut deserializer)
                        .map_err(|_| toy_rpc::error::Error::RpcError(toy_rpc::error::RpcError::InvalidRequest))?;
                    let res = self.#ident(req).await
                        .map(|r| Box::new(r) as Box<dyn toy_rpc::erased_serde::Serialize + Send + Sync + 'static>)
                        .map_err(|e| toy_rpc::error::Error::RpcError(
                            toy_rpc::error::RpcError::ServerError(e.to_string())
                        ));
                    res
                }
            )
        });

        f.sig.inputs = parse_quote!(
            self: std::sync::Arc<Self>, mut deserializer: Box<dyn toy_rpc::erased_serde::Deserializer<'static> + Send>
        );

        f.sig.output = parse_quote!(
            -> toy_rpc::service::HandlerResultFut
        );
    };

    f.sig.ident = handler_ident;
}

/// remove #[export_method] attribute
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
fn generate_client_stub(ident: &Ident, input: syn::ItemImpl) -> (syn::Item, syn::ItemImpl, syn::Item, syn::ItemImpl) {
    let concat_name = format!("{}{}", &ident.to_string(), CLIENT_SUFFIX);
    let client_ident = Ident::new(&concat_name, ident.span());

    let client_struct = parse_quote!(
        pub struct #client_ident<'c> {
            client: &'c toy_rpc::client::Client<toy_rpc::client::Connected>,
            service_name: &'c str,
        }
    );

    let client_impl = client_stub_impl(&client_ident, input);

    let concat_name = format!("{}{}", &ident.to_string(), CLIENT_STUB_SUFFIX);
    let stub_ident = Ident::new(&concat_name, ident.span());
    // let stub_fn_name = Ident::new(&ident.to_string().to_lowercase(), ident.span());
    let stub_fn = parse_stub_fn_name(ident);
    
    let stub_trait = parse_quote!(
        pub trait #stub_ident {
            fn #stub_fn<'c>(&'c self, service_name: &'c str) -> #client_ident;
        }
    );

    let stub_impl: syn::ItemImpl = parse_quote!(
        impl #stub_ident for toy_rpc::client::Client<toy_rpc::client::Connected> {
            fn #stub_fn<'c>(&'c self, service_name: &'c str) -> #client_ident {
                #client_ident {
                    client: self,
                    service_name,
                }
            }
        }  
    );

    return (client_struct, client_impl, stub_trait, stub_impl)
}

fn client_stub_impl(client_ident: &Ident, input: syn::ItemImpl) -> syn::ItemImpl {
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
            if let Some(gen) = generate_client_stub_method(f) {
                generated_items.push(syn::ImplItem::Method(gen));
            }
        });
    
    let mut output: syn::ItemImpl = parse_quote!(
        impl<'c> #client_ident<'c> {

        }
    );

    output.items = generated_items;
    output
}

fn generate_client_stub_method(f: &mut syn::ImplItemMethod) -> Option<syn::ImplItemMethod> {
    if let syn::FnArg::Typed(pt) = f.sig.inputs.last().unwrap() {
        let fn_ident = &f.sig.ident;
        let req_ty = &pt.ty;
        
        if let syn::ReturnType::Type(_, ret_ty) = f.sig.output.clone() {
            let ok_ty = get_ok_ident_from_type(ret_ty)?;
            return Some(generate_client_stub_method_impl(fn_ident, &req_ty, &ok_ty))
        }
    }

    return None
}   

fn generate_client_stub_method_impl(fn_ident: &Ident, req_ty: &Box<syn::Type>, ok_ty: &GenericArgument) -> syn::ImplItemMethod {
    let method = fn_ident.to_string();
    parse_quote!(
        pub async fn #fn_ident<A>(&'c self, args: A) -> Result<#ok_ty, toy_rpc::error::Error>
        where 
            A: std::borrow::Borrow<#req_ty> + Send + Sync + toy_rpc::serde::Serialize,
        {
            let method = #method;
            let service_method = format!("{}.{}", self.service_name, method);

            self.client.async_call(service_method, args).await
        }
    )
}

fn get_ok_ident_from_type(ty: Box<syn::Type>) -> Option<GenericArgument> {
    let ty = Box::leak(ty);
    let arg = syn::GenericArgument::Type(ty.to_owned());
    return recursively_get_restul_from_generic_arg(&arg)
}

fn recursively_get_restul_from_generic_arg(arg: &GenericArgument) -> Option<GenericArgument> {
    match &arg {
        &syn::GenericArgument::Type(ty) => {
            return recusively_get_result_from_type(&ty);
        },
        &syn::GenericArgument::Binding(binding) => {
            return recusively_get_result_from_type(&binding.ty);
        },
        _ => { None }
    }
}

fn recusively_get_result_from_type(ty: &syn::Type) -> Option<GenericArgument> {
    match ty {
        &syn::Type::Path(ref path) => {
            let ident = &path.path.segments.last()?.ident.to_string()[..];
            match &path.path.segments.last()?.arguments {
                syn::PathArguments::AngleBracketed(angle_bracket) => {
                    if ident == "Result" {
                        return angle_bracket.args.first()
                            .map(|g| g.to_owned())
                    }
                    return recursively_get_restul_from_generic_arg(angle_bracket.args.first()?)
                },
                _ => {
                    return None
                }
            }
        },
        &syn::Type::TraitObject(ref tobj) => {
            if let syn::TypeParamBound::Trait(bound) = tobj.bounds.first()? {
                match &bound.path.segments.last()?.arguments {
                    syn::PathArguments::AngleBracketed(angle_bracket) => {
                        return recursively_get_restul_from_generic_arg(angle_bracket.args.first()?)
                    },
                    _ => {
                        return None
                    }
                }
            }
            None
        }
        _ => {
            None
        }
    }    
}

fn generate_register_service_impl(ident: &Ident) -> impl ToTokens {
    let name = ident.to_string();
    let static_name = format!("{}_{}", SERVICE_PREFIX, &name.to_uppercase());
    let static_ident = syn::Ident::new(&static_name, ident.span());
    let ret = quote! {
        impl toy_rpc::util::RegisterService for #ident {
            fn handlers() -> &'static std::collections::HashMap<&'static str, toy_rpc::service::AsyncHandler<Self>> {
                &*#static_ident
            }

            fn default_name() -> &'static str {
                let name = #name;
                name.as_ref()
            }
        }
    };

    ret
}
struct ServiceExport {
    instance_id: syn::Ident,
    impl_path: syn::Path,
}

impl syn::parse::Parse for ServiceExport {
    fn parse(input: syn::parse::ParseStream) -> Result<Self, syn::Error> {
        let instance_id: syn::Ident = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let impl_path: syn::Path = input.parse()?;

        Ok(ServiceExport {
            instance_id,
            impl_path,
        })
    }
}

/// Find the exported methods with the provided path
/// 
/// Example 
/// 
/// ```rust
/// struct Foo { }
/// 
/// #[export_impl]
/// impl Foo { 
///     //rpc service impl here
/// }
/// 
/// mod rpc {
///     pub struct Bar { }
/// 
///     #[export_impl]
///     impl Bar {
///         //rpc service impl here
///     }
/// }
/// 
/// use toy_rpc::Server;
/// 
/// fn main() {
///     let foo = Arc::new(Foo {});
///     let bar = Arc::new(rpc::Bar {});
///     
///     let server = Server::builder()
///         .register("foo_service", service!(foo, Foo))
///         .register("bar_service", service!(bar, rpc::Bar))
///         .build();
/// }
/// 
/// ```
#[proc_macro]
pub fn service(input: TokenStream) -> TokenStream {
    let ServiceExport {
        instance_id,
        impl_path,
    } = parse_macro_input!(input as ServiceExport);

    let last_segment = impl_path.segments.last().unwrap();
    let ident = &last_segment.ident;
    let static_name = format!("{}_{}", SERVICE_PREFIX, &ident.to_string().to_uppercase());
    let static_ident = syn::Ident::new(&static_name, ident.span());
    let mut static_impl_path = impl_path.clone();

    // modify the path
    static_impl_path.segments.last_mut().unwrap().ident = static_ident;

    let output = quote! {
        toy_rpc::service::build_service(#instance_id, &*#static_impl_path)
    };

    output.into()
}

fn parse_impl_self_ty(self_ty: &syn::Type) -> Result<&syn::Ident, syn::Error> {
    match self_ty {
        syn::Type::Path(tp) => Ok(&tp.path.segments[0].ident),
        _ => Err(syn::Error::new_spanned(
            quote! {},
            "Compile Error: Self type",
        )),
    }
}

fn parse_stub_fn_name(ident: &Ident) -> Ident {
    let mut output_fn = String::new();
    for c in ident.to_string().chars() {
        if c.is_uppercase() {
            output_fn.push('_');
            output_fn.push_str(&c.to_lowercase().to_string());
        } else {
            output_fn.push(c);
        }
    }
    output_fn = output_fn.trim_start_matches('_')
        .trim_end_matches('_')
        .into();

    Ident::new(&output_fn, ident.span())
}

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
