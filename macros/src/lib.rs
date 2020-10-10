use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Ident, parse_quote};

const SERVICE_PREFIX: &str = "STATIC_TOY_RPC_SERVICE";
const ATTR_EXPORT_METHOD: &str = "export_method";
const HANDLER_SUFFIX: &str = "handler";

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

// #[proc_macro_attribute]
// pub fn export_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
//     // parse item
//     let mut input = parse_macro_input!(item as syn::ItemImpl);
//     let (fn_idents, names) = filter_export_method(&mut input);

//     // extract Self type and use it for construct Ident for handler HashMap
//     let self_ty = &input.self_ty;
//     let ident = match parse_impl_self_ty(self_ty) {
//         Ok(i) => i,
//         Err(err) => return err.to_compile_error().into(),
//     };
//     let static_name = format!("{}_{}", SERVICE_PREFIX, ident.to_string().to_uppercase());
//     let static_ident = Ident::new(&static_name, ident.span());

//     // export a lazy_static HashMap of handlers
//     let export = quote! {
//         lazy_static::lazy_static! {
//             #[allow(non_upper_case_globals)]
//             static ref #static_ident:
//                 std::collections::HashMap<&'static str, toy_rpc::service::Handler<#self_ty>>
//                 = {
//                     let mut map = std::collections::HashMap::new();
//                     #(map.insert(#names, toy_rpc::service::wrap_method(#self_ty::#fn_idents));)*;

//                     map
//                 };
//         }
//     };

//     let output = quote! {
//         #input
//         #export
//     };
//     output.into()
// }

#[proc_macro_attribute]
pub fn export_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // parse item
    let input = parse_macro_input!(item as syn::ItemImpl);
    // let (fn_idents, names) = filter_async_export_method(&mut input);
    // let handler_idents = generate_handler_idents(&fn_idents);
    let (handler_impl, names, fn_idents) = transform_impl(input.clone());

    // extract Self type and use it for construct Ident for handler HashMap
    let self_ty = &input.self_ty;
    let ident = match parse_impl_self_ty(self_ty) {
        Ok(i) => i,
        Err(err) => return err.to_compile_error().into(),
    };
    let static_name = format!("{}_{}", SERVICE_PREFIX, ident.to_string().to_uppercase());
    let static_ident = Ident::new(&static_name, ident.span());

    let lazy = quote! {
        lazy_static::lazy_static! { 
            static ref #static_ident:
                std::collections::HashMap<&'static str, toy_rpc::service::ArcAsyncHandler<#self_ty>>
                = {
                    let mut map: std::collections::HashMap<&'static str, toy_rpc::service::ArcAsyncHandler<#self_ty>> 
                        = std::collections::HashMap::new();
                    #(map.insert(#names, std::sync::Arc::new(#self_ty::#fn_idents));)*;
                    map
                };
        }
    };
    
    let input = remove_export_method_attr(input);
    let handler_impl = remove_export_method_attr(handler_impl);

    let output = quote! {
        #input
        #handler_impl
        #lazy
    };
    output.into()
}

// fn filter_export_method(input: &mut syn::ItemImpl) -> (Vec<syn::Ident>, Vec<String>) {
//     let mut fn_idents: Vec<syn::Ident> = Vec::new();
//     let mut names: Vec<String> = Vec::new();
    
//     input.items.iter_mut()
//         // first filter out method
//         .filter_map(|item| match item {
//             syn::ImplItem::Method(f) => Some(f),
//             _ => None,
//         })
//         // find whether function has attributes
//         .filter(|f| {
//             // println!("{:?}", &f.attrs);
//             f.attrs.iter().any(|attr| {
//                 let ident = attr.path.get_ident().unwrap();
//                 ident == ATTR_EXPORT_METHOD
//             })
//         })
//         // append ident and names of function with attribute
//         .for_each(|f| {
//             fn_idents.push(f.sig.ident.clone());
//             names.push(f.sig.ident.to_string());

//             // clear the attributes for now
//             f.attrs.retain(|attr| {
//                 let ident = attr.path.get_ident().unwrap();
//                 ident != ATTR_EXPORT_METHOD
//             })
//         });
    
//     (fn_idents, names)
// }

// fn filter_async_export_method(input: &mut syn::ItemImpl) -> (Vec<syn::Ident>, Vec<String>) {
//     let mut fn_idents: Vec<syn::Ident> = Vec::new();
//     let mut names: Vec<String> = Vec::new();
    
//     input.items.iter_mut()
//         // first filter out method
//         .filter_map(|item| match item {
//             syn::ImplItem::Method(f) => Some(f),
//             _ => None,
//         })
//         // find whether function has attributes
//         .filter(|f| {
//             // println!("{:?}", &f.attrs);
//             f.attrs.iter().any(|attr| {
//                 let ident = attr.path.get_ident().unwrap();
//                 ident == ATTR_EXPORT_METHOD
//             })
//         })
//         // filter async function
//         .filter(|f| {
//             f.sig.asyncness.is_some()
//         })
//         // append ident and names of function with attribute
//         .for_each(|f| {
//             fn_idents.push(f.sig.ident.clone());
//             names.push(f.sig.ident.to_string());

//             // clear the attributes for now
//             f.attrs.retain(|attr| {
//                 let ident = attr.path.get_ident().unwrap();
//                 ident != ATTR_EXPORT_METHOD
//             })
//         });
    
//     (fn_idents, names)
// }

fn transform_impl(input: syn::ItemImpl) -> (syn::ItemImpl, Vec<String>, Vec<Ident>) {
    let mut names = Vec::new();
    let mut idents = Vec::new();
    let mut output = input;
    // let self_ty = &output.self_ty;
    output.items.retain(|item| 
        match item {
            syn::ImplItem::Method(f) => {
                let is_exported = f.attrs.iter().any(|attr| {
                    let ident = attr.path.get_ident().unwrap();
                    ident == ATTR_EXPORT_METHOD
                });

                // let is_async = f.sig.asyncness.is_some();

                // is_exported && is_async
                is_exported
            },
            _ => false
        }
    );

    output.trait_ = None;
    output.items.iter_mut()
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
                    let req: #req_ty = erased_serde::deserialize(&mut deserializer)
                        .map_err(|_| toy_rpc::error::Error::RpcError(toy_rpc::error::RpcError::InvalidRequest))?;
                    let res = self.#ident(req).await
                        .map(|r| Box::new(r) as Box<dyn erased_serde::Serialize + Send + Sync + 'static>)
                        .map_err(|e| toy_rpc::error::Error::RpcError(
                            toy_rpc::error::RpcError::ServerError(e.to_string())
                        ));
                    res
                }
            )
        });
        
        f.sig.inputs = parse_quote!(
            self: Arc<Self>, mut deserializer: Box<dyn erased_serde::Deserializer<'static> + Send>
        );

        f.sig.output = parse_quote!(
            -> toy_rpc::service::HandlerResultFut
        );
        
    };
    
    f.sig.ident = handler_ident;
}


fn remove_export_method_attr(mut input: syn::ItemImpl) -> syn::ItemImpl {
    input.items.iter_mut()
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

#[proc_macro]
pub fn service(input: TokenStream) -> TokenStream {
    let ServiceExport {
        instance_id,
        impl_path,
    } = parse_macro_input!(input as ServiceExport);
    let ident = impl_path.get_ident().unwrap();
    let static_name = format!("{}_{}", SERVICE_PREFIX, &ident.to_string().to_uppercase());
    let static_ident = syn::Ident::new(&static_name, ident.span());
    let mut static_impl_path = impl_path.clone();

    // modify the path
    static_impl_path.segments.first_mut().unwrap().ident = static_ident;

    let output = quote! {
        toy_rpc::service::build_service(#instance_id, &*#static_impl_path)
    };

    output.into()
}

// #[proc_macro_attribute]
// pub fn export_struct(attr: TokenStream, item: TokenStream) -> TokenStream {
//     let input = parse_macro_input!(item as ItemStruct);
//     let ident = &input.ident;
//     let static_name = format!("{}_{}", SERVICE_PREFIX, ident);
//     let static_ident = Ident::new(&static_name, ident.span());

//     // initialize
//     let static_map_output = quote! {
//         lazy_static::lazy_static! {
//             static ref #static_ident:
//                 std::sync::Mutex<std::collections::HashMap<&'static str, async_std::sync::Arc<toy_rpc::service::Handler<#ident>>>>
//             = {
//                 std::sync::Mutex::new(
//                     HashMap::new()
//                 )
//             };
//         }
//     };

//     let output = quote! {
//         #input

//         #static_map_output
//     };

//     output.into()
// }

fn parse_impl_self_ty(self_ty: &syn::Type) -> Result<&syn::Ident, syn::Error> {
    match self_ty {
        syn::Type::Path(tp) => Ok(&tp.path.segments[0].ident),
        _ => Err(syn::Error::new_spanned(
            quote! {},
            "Compile Error: Self type",
        )),
    }
}
