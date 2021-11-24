
#[cfg(all(feature = "client", feature = "runtime",))]
use super::{CLIENT_STUB_SUFFIX, CLIENT_SUFFIX};
#[cfg(feature = "server")]
use super::{EXPORTED_TRAIT_SUFFIX, HANDLER_SUFFIX};
// #[cfg(any(feature = "server", feature = "client"))]
use super::ATTR_EXPORT_METHOD;

pub mod item_impl;

pub mod item_trait;

#[cfg(any(feature = "server", all(feature = "client", feature = "runtime")))]
pub(crate) fn unwrap_return_type(ty: &syn::ReturnType) -> syn::Type {
    let ret_ty: syn::Type = match ty {
        syn::ReturnType::Default => syn::parse_quote! {()},
        syn::ReturnType::Type(_, ty) => syn::parse_quote! {#ty}
    };

    let ty = unwrap_type(ret_ty);
    ty
}

#[cfg(any(feature = "server", all(feature = "client", feature = "runtime")))]
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

#[cfg(any(feature = "server", all(feature = "client", feature = "runtime")))]
pub(crate) fn unwrap_type_path_for_async_trait(tp: &syn::TypePath) -> Option<&syn::Type> {
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

#[cfg(any(feature = "server", all(feature = "client", feature = "runtime")))]
pub(crate) fn unwrap_till_ident<'a>(segments: impl Iterator<Item=&'a syn::PathSegment>, ident: &'a str) -> Option<&'a syn::PathArguments> {
    for seg in segments {
        if seg.ident == ident {
            return Some(&seg.arguments)
        }
    }
    None
}

#[cfg(any(feature = "server", all(feature = "client", feature = "runtime")))]
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


#[cfg(any(feature = "server", all(feature = "client", feature = "runtime")))]
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
        toy_rpc_client_stub!(self, #fn_ident, #service_method, #req_ty, #ret_ty)
    }
}

#[cfg(feature = "server")]
pub(crate) fn macro_rules_handler() -> proc_macro2::TokenStream {
    quote::quote! {
        macro_rules! toy_rpc_handler {
            // handler when return type is a Result
             // handler when return type is either a toy_rpc or anyhow Result
             ($self:ident, $method:ident, $de:ident, $req_ty:ty, Result<$ok_ty:ty>) => {
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

            ($self:ident, $method:ident, $de:ident, $req_ty:ty, Result<$ok_ty:ty, $err_ty:ty>) => {
                toy_rpc_handler!($self, $method, $de, $req_ty, Result<$ok_ty>)
            };

            // handler when return type is either a toy_rpc::Result
            ($self:ident, $method:ident, $de:ident, $req_ty:ty, toy_rpc::Result<$ok_ty:ty>) => {
                toy_rpc_handler!($self, $method, $de, $req_ty, Result<$ok_ty>)
            };

            // handler when return type is either a anyhow::Result
            ($self:ident, $method:ident, $de:ident, $req_ty:ty, anyhow::Result<$ok_ty:ty>) => {
                toy_rpc_handler!($self, $method, $de, $req_ty, Result<$ok_ty>)
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

#[cfg(all(feature = "client", feature = "runtime"))]
pub(crate) fn macro_rules_client_stub() -> proc_macro2::TokenStream {
    quote::quote! {
        macro_rules! toy_rpc_client_stub {
            // For Service Trait ie. `Service::method(&client, args)`
            // Result return type
            ($self:ident, $service_method:expr, $req_id:ident, Result<$ok_ty:ty>) => {
                Box::pin(
                    async move {
                        let success = $self.call($service_method, $req_id).await?;
                        Ok(success)
                    }
                )
            };

            ($self:ident, $service_method:expr, $req_id:ident, Result<$ok_ty:ty, $err_ty:ty>) => {
                toy_rpc_client_stub!($self, $service_method, $req_id, Result<$ok_ty>)
            };

            ($self:ident, $service_method:expr, $req_id:ident, toy_rpc::Result<$ok_ty:ty>) => {
                toy_rpc_client_stub!($self, $service_method, $req_id, Result<$ok_ty>)
            };

            ($self:ident, $service_method:expr, $req_id:ident, anyhow::Result<$ok_ty:ty>) => {
                toy_rpc_client_stub!($self, $service_method, $req_id, Result<$ok_ty>)
            };

            // non Result return type
            ($self:ident, $service_method:expr, $req_id:ident, $ret_ty:ty) => {
                compile_error!("`#[export_trait(impl_for_client)]` requires return type to be Result<_, _>");
            };

            // For Service Struct ie. `client.service().method(args)`
            // Result return type
            ($self:ident, $func:ident, $service_method:expr, $req_ty:ty, Result<$ok_ty:ty>) => {
                pub fn $func<A>(&'c $self, args: A) -> toy_rpc::client::Call<$ok_ty>
                where 
                    A: std::borrow::Borrow<$req_ty> + Send + Sync + toy_rpc::serde::Serialize + 'static,
                {
                    $self.client.call($service_method, args)
                }
            };

            ($self:ident, $func:ident, $service_method:expr, $req_ty:ty, Result<$ok_ty:ty, $err_ty:ty>) => {
                toy_rpc_client_stub!($self, $func, $service_method, $req_ty, Result<$ok_ty>);
            };

            ($self:ident, $func:ident, $service_method:expr, $req_ty:ty, toy_rpc::Result<$ok_ty:ty>) => {
                toy_rpc_client_stub!($self, $func, $service_method, $req_ty, Result<$ok_ty>);
            };

            ($self:ident, $func:ident, $service_method:expr, $req_ty:ty, anyhow::Result<$ok_ty:ty>) => {
                toy_rpc_client_stub!($self, $func, $service_method, $req_ty, Result<$ok_ty>);
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