
#[cfg(all(feature = "client", feature = "runtime",))]
use super::{CLIENT_STUB_SUFFIX, CLIENT_SUFFIX};
#[cfg(feature = "server")]
use super::{EXPORTED_TRAIT_SUFFIX, HANDLER_SUFFIX};
// #[cfg(any(feature = "server", feature = "client"))]
use super::ATTR_EXPORT_METHOD;

pub mod item_impl;

pub mod item_trait;

// #[cfg(all(feature = "client", feature = "runtime"))]
// pub(crate) fn get_ok_ident_from_type(ty: Box<syn::Type>) -> Option<syn::GenericArgument> {
//     let ty = Box::leak(ty);
//     let arg = syn::GenericArgument::Type(ty.to_owned());
//     recursively_get_result_from_generic_arg(&arg)
// }

// #[cfg(all(feature = "client", feature = "runtime"))]
// pub(crate) fn recursively_get_result_from_generic_arg(
//     arg: &syn::GenericArgument,
// ) -> Option<syn::GenericArgument> {
//     match &arg {
//         syn::GenericArgument::Type(ty) => recusively_get_result_from_type(&ty),
//         syn::GenericArgument::Binding(binding) => recusively_get_result_from_type(&binding.ty),
//         _ => None,
//     }
// }

pub(crate) fn unwrap_return_type(ty: &syn::ReturnType) -> syn::Type {
    // use syn::{Type};
    // println!("{:?}", &ty);
    let ret_ty: syn::Type = match ty {
        syn::ReturnType::Default => syn::parse_quote! {()},
        syn::ReturnType::Type(_, ty) => syn::parse_quote! {#ty}
    };

    let ty = unwrap_type(ret_ty);
    ty
}

pub(crate) fn unwrap_type(ty: syn::Type) -> syn::Type {
    let tp = match ty {
        syn::Type::Path(tp) => tp,
        _ => return ty
    };

    match unwrap_type_path_for_async_trait(&tp) {
        Some(ty) => ty.clone(),
        None => syn::Type::Path(tp.clone())
    }
}

pub(crate) fn unwrap_type_path_for_async_trait(tp: &syn::TypePath) -> Option<&syn::Type> {
    // println!("{:?}", tp.path.segments);

    let pin_args = unwrap_till_ident(tp.path.segments.iter(), "Pin")?;
    let ty = unwrap_angled_path_args(pin_args)?;
    let segments = match ty {
        syn::Type::Path(tp) => tp.path.segments.iter(),
        _ => return None
    };
    let box_args = unwrap_till_ident(segments, "Box")?;
    let ty = unwrap_angled_path_args(box_args)?;
    let objs = match ty {
        syn::Type::TraitObject(tobj) => &tobj.bounds,
        _ => return None
    };
    let segments = unwrap_trait_obj(objs.iter())?;
    let fut_args = unwrap_till_ident(segments, "Future")?;
    let ty = unwrap_angled_path_args(fut_args)?;

    Some(ty)
}

pub(crate) fn unwrap_till_ident<'a>(segments: impl Iterator<Item=&'a syn::PathSegment>, ident: &'a str) -> Option<&'a syn::PathArguments> {
    for seg in segments {
        // println!("{:?}", seg.ident);
        if seg.ident == ident {
            return Some(&seg.arguments)
        }
    }
    None
}

pub(crate) fn unwrap_angled_path_args(path_args: &syn::PathArguments) -> Option<&syn::Type> {
    if let syn::PathArguments::AngleBracketed(aargs) = path_args {
        if let Some(garg) = aargs.args.first() {
            match garg {
                syn::GenericArgument::Type(ty) => return Some(ty),
                syn::GenericArgument::Binding(binding) => return Some(&binding.ty),
                _ => return None
            }
        }
    }

    None
}


pub(crate) fn unwrap_trait_obj<'a>(objs: impl Iterator<Item=&'a syn::TypeParamBound>) -> Option<impl Iterator<Item=&'a syn::PathSegment>> {
    for obj in objs {
        match obj {
            syn::TypeParamBound::Trait(tb) => {
                return Some(tb.path.segments.iter())
            },
            _ => {}
        }
    }
    None
}

// #[cfg(all(feature = "client", feature = "runtime"))]
// pub(crate) fn recusively_get_result_from_type(ty: &syn::Type) -> Option<syn::GenericArgument> {
//     match ty {
//         syn::Type::Path(ref path) => {
//             let ident = &path.path.segments.last()?.ident.to_string()[..];
//             match &path.path.segments.last()?.arguments {
//                 syn::PathArguments::AngleBracketed(angle_bracket) => {
//                     if ident == "Result" {
//                         return angle_bracket.args.first().map(|g| g.to_owned());
//                     }
//                     recursively_get_result_from_generic_arg(angle_bracket.args.first()?)
//                 }
//                 _ => None,
//             }
//         }
//         syn::Type::TraitObject(ref tobj) => {
//             if let syn::TypeParamBound::Trait(bound) = tobj.bounds.first()? {
//                 match &bound.path.segments.last()?.arguments {
//                     syn::PathArguments::AngleBracketed(angle_bracket) => {
//                         return recursively_get_result_from_generic_arg(angle_bracket.args.first()?)
//                     }
//                     _ => return None,
//                 }
//             }
//             None
//         }
//         _ => None,
//     }
// }

// #[cfg(any(feature = "server", all(feature = "client", feature = "runtime",)))]
// pub(crate) fn parse_impl_self_ty(self_ty: &syn::Type) -> Result<&syn::Ident, syn::Error> {
//     match self_ty {
//         syn::Type::Path(tp) => Ok(&tp.path.segments[0].ident),
//         _ => Err(syn::Error::new_spanned(
//             quote::quote! {},
//             "Compile Error: Self type",
//         )),
//     }
// }

#[cfg(any(feature = "server", all(feature = "client", feature = "runtime",)))]
pub(crate) fn parse_impl_self_ty_path(self_ty: &syn::Type) -> Result<&syn::TypePath, syn::Error> {
    match self_ty {
        syn::Type::Path(tp) => Ok(tp),
        _ => Err(syn::Error::new_spanned(
            quote::quote! {},
            "Compile Error: Self type",
        )),
    }
}

#[cfg(any(feature = "server", all(feature = "client", feature = "runtime",)))]
pub(crate) fn parse_type_ident_from_type_path(
    path: &syn::TypePath,
) -> Result<&syn::Ident, syn::Error> {
    match path.path.segments.last() {
        Some(seg) => Ok(&seg.ident),
        None => Err(syn::Error::new_spanned(
            quote::quote! {},
            "Expecting type ident",
        )),
    }
}

#[cfg(all(feature = "client", feature = "runtime"))]
pub(crate) fn parse_stub_fn_name(ident: &syn::Ident) -> syn::Ident {
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

fn is_exported(attr: &syn::Attribute) -> bool {
    if let Some(ident) = attr.path.get_ident() {
        ident == ATTR_EXPORT_METHOD
    } else {
        false
    }
}

#[cfg(all(feature = "client", feature = "runtime"))]
pub(crate) fn generate_client_stub_for_struct_method_impl(
    service_ident: &syn::Ident,
    fn_ident: &syn::Ident,
    req_ty: &syn::Type,
    ret_ty: &syn::ReturnType,
) -> proc_macro2::TokenStream {
    let service = service_ident.to_string();
    let method = fn_ident.to_string();
    let service_method = format!("{}.{}", service, method);
    let ret_ty = unwrap_return_type(ret_ty);
    quote::quote!{
        // pub fn #fn_ident<A>(&'c self, args: A) -> toy_rpc::client::Call<#ret_ty>
        // where
        //     A: std::borrow::Borrow<#req_ty> + Send + Sync + toy_rpc::serde::Serialize + 'static,
        // {
        //     self.client.call(#service_method, args)
        // }
        toy_rpc_client_stub!(self, #fn_ident, #service_method, #req_ty, #ret_ty)
    }
}

pub(crate) fn macro_rules_handler() -> proc_macro2::TokenStream {
    quote::quote! {
        macro_rules! toy_rpc_handler {
            // handler when return type is a Result
            ($self:ident, $method:ident, $de:ident, $req_ty:ty, Result<$ok_ty:ty, $err_ty:ty>) => {
                Box::pin(
                    async move {
                        let req: $req_ty = toy_rpc::erased_serde::deserialize(&mut $de)
                            .map_err(|e| toy_rpc::error::Error::ParseError(Box::new(e)))?;
                        // let result: Result<$ok_ty, $err_ty> = $self.$method(req).await;
                        let outcome: $ok_ty = $self.$method(req).await?;
                        Ok(Box::new(outcome) as toy_rpc::service::Outcome)
                    }
                )
            };

            // handler when return type is either a toy_rpc or anyhow Result
            ($self:ident, $method:ident, $de:ident, $req_ty:ty, Result<$ok_ty:ty>) => {
                Box::pin(
                    async move {
                        let req: $req_ty = toy_rpc::erased_serde::deserialize(&mut $de)
                            .map_err(|e| toy_rpc::error::Error::ParseError(Box::new(e)))?;
                        let outcome: $ok_ty = $self.$method(req).await?;
                        Ok(Box::new(outcome) as toy_rpc::service::Outcome)
                    }
                )
            };

            // handler when return type is either a toy_rpc::Result
            ($self:ident, $method:ident, $de:ident, $req_ty:ty, toy_rpc::Result<$ok_ty:ty>) => {
                Box::pin(
                    async move {
                        let req: $req_ty = toy_rpc::erased_serde::deserialize(&mut $de)
                            .map_err(|e| toy_rpc::error::Error::ParseError(Box::new(e)))?;
                        let outcome: $ok_ty = $self.$method(req).await?;
                        Ok(Box::new(outcome) as toy_rpc::service::Outcome)
                    }
                )
            };

            // handler when return type is either a anyhow::Result
            ($self:ident, $method:ident, $de:ident, $req_ty:ty, anyhow::Result<$ok_ty:ty>) => {
                Box::pin(
                    async move {
                        let req: $req_ty = toy_rpc::erased_serde::deserialize(&mut $de)
                            .map_err(|e| toy_rpc::error::Error::ParseError(Box::new(e)))?;
                        let outcome: $ok_ty = $self.$method(req).await?;
                        Ok(Box::new(outcome) as toy_rpc::service::Outcome)
                    }
                )
            };

            // handler when return type is not Result
            ($self:ident, $method:ident, $de:ident, $req_ty:ty, $ret_ty:ty) => {
                Box::pin(
                    async move {
                        let req: $req_ty = toy_rpc::erased_serde::deserialize(&mut $de)
                            .map_err(|e| toy_rpc::error::Error::ParseError(Box::new(e)))?;
                        let outcome: $ret_ty = $self.$method(req).await;
                        Ok(Box::new(outcome) as toy_rpc::service::Outcome)
                    }
                )
            };
        }
    }
}

pub(crate) fn macro_rules_client_stub() -> proc_macro2::TokenStream {
    quote::quote! {
        macro_rules! toy_rpc_client_stub {
            // For Service Trait
            // Result return type
            ($self:ident, $service_method:expr, $req_id:ident, Result<$ok_ty:ty, $err_ty:ty>) => {
                Box::pin(
                    async move {
                        let success = $self.call($service_method, $req_id).await?;
                        Ok(success)
                    }
                )
            };

            ($self:ident, $service_method:expr, $req_id:ident, Result<$ok_ty:ty>) => {
                Box::pin(
                    async move {
                        let success = $self.call($service_method, $req_id).await?;
                        Ok(success)
                    }
                )
            };

            ($self:ident, $service_method:expr, $req_id:ident, toy_rpc::Result<$ok_ty:ty>) => {
                Box::pin(
                    async move {
                        let success = $self.call($service_method, $req_id).await?;
                        Ok(success)
                    }
                )
            };

            ($self:ident, $service_method:expr, $req_id:ident, anyhow::Result<$ok_ty:ty>) => {
                Box::pin(
                    async move {
                        let success = $self.call($service_method, $req_id).await?;
                        Ok(success)
                    }
                )
            };

            // non Result return type
            ($self:ident, $service_method:expr, $req_id:ident, $ret_ty:ty) => {
                // Box::pin(
                //     async move {
                //         let success = $self.call($service_method, $req_id).await;
                //         Ok(success)
                //     }
                // )
                compile_error!("`#[export_trait(impl_for_client)]` requires return type to be Result<_, _>");
            };

            // For Service Struct
            // Result return type
            ($self:ident, $func:ident, $service_method:expr, $req_ty:ty, Result<$ok_ty:ty, $err_ty:ty>) => {
                pub fn $func<A>(&'c $self, args: A) -> toy_rpc::client::Call<$ok_ty>
                where 
                    A: std::borrow::Borrow<$req_ty> + Send + Sync + toy_rpc::serde::Serialize + 'static,
                {
                    $self.client.call($service_method, args)
                }
            };

            ($self:ident, $func:ident, $service_method:expr, $req_ty:ty, Result<$ok_ty:ty>) => {
                pub fn $func<A>(&'c $self, args: A) -> toy_rpc::client::Call<$ok_ty>
                where 
                    A: std::borrow::Borrow<$req_ty> + Send + Sync + toy_rpc::serde::Serialize + 'static,
                {
                    $self.client.call($service_method, args)
                }
            };

            ($self:ident, $func:ident, $service_method:expr, $req_ty:ty, toy_rpc::Result<$ok_ty:ty>) => {
                pub fn $func<A>(&'c $self, args: A) -> toy_rpc::client::Call<$ok_ty>
                where 
                    A: std::borrow::Borrow<$req_ty> + Send + Sync + toy_rpc::serde::Serialize + 'static,
                {
                    $self.client.call($service_method, args)
                }
            };

            ($self:ident, $func:ident, $service_method:expr, $req_ty:ty, anyhow::Result<$ok_ty:ty>) => {
                pub fn $func<A>(&'c $self, args: A) -> toy_rpc::client::Call<$ok_ty>
                where 
                    A: std::borrow::Borrow<$req_ty> + Send + Sync + toy_rpc::serde::Serialize + 'static,
                {
                    $self.client.call($service_method, args)
                }
            };

            // non Result return type
            ($self:ident, $func:ident, $service_method:expr, $req_ty:ty, $ret_ty:ty) => {
                pub fn $func<A>(&'c $self, args: A) -> toy_rpc::client::Call<$ret_ty>
                where 
                    A: std::borrow::Borrow<$req_ty> + Send + Sync + toy_rpc::serde::Serialize + 'static,
                {
                    $self.client.call($service_method, args)
                }
            };
        }
    }
}