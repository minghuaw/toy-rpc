use quote::quote;

#[cfg(all(feature = "client", feature = "runtime",))]
use super::{CLIENT_STUB_SUFFIX, CLIENT_SUFFIX};
#[cfg(feature = "server")]
use super::{EXPORTED_TRAIT_SUFFIX, HANDLER_SUFFIX};
// #[cfg(any(feature = "server", feature = "client"))]
use super::ATTR_EXPORT_METHOD;

pub mod item_impl;

pub mod item_trait;

#[cfg(all(feature = "client", feature = "runtime"))]
pub(crate) fn get_ok_ident_from_type(ty: Box<syn::Type>) -> Option<syn::GenericArgument> {
    let ty = Box::leak(ty);
    let arg = syn::GenericArgument::Type(ty.to_owned());
    recursively_get_result_from_generic_arg(&arg)
}

#[cfg(all(feature = "client", feature = "runtime"))]
pub(crate) fn recursively_get_result_from_generic_arg(
    arg: &syn::GenericArgument,
) -> Option<syn::GenericArgument> {
    match &arg {
        syn::GenericArgument::Type(ty) => recusively_get_result_from_type(&ty),
        syn::GenericArgument::Binding(binding) => recusively_get_result_from_type(&binding.ty),
        _ => None,
    }
}

#[cfg(all(feature = "client", feature = "runtime"))]
pub(crate) fn recusively_get_result_from_type(ty: &syn::Type) -> Option<syn::GenericArgument> {
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

pub(crate) fn macro_rules_handler() -> proc_macro2::TokenStream {
    quote! {
        macro_rules! handler {
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
            }
        }
    }
}