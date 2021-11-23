use super::*;

/// transform impl block to meet the signature of service function
///
#[cfg(feature = "server")]
pub(crate) fn transform_impl(
    input: syn::ItemImpl,
) -> (syn::ItemImpl, Vec<String>, Vec<syn::Ident>) {
    let mut names = Vec::new();
    let mut idents = Vec::new();
    let self_ty = input.self_ty.clone();
    let mut methods = exported_impl_items(input);

    methods.iter_mut()
        // first filter out method
        .filter_map(|item| match item {
            syn::ImplItem::Method(f) => Some(f),
            _ => None,
        })
        .for_each(|f| {
            names.push(f.sig.ident.to_string());
            transform_impl_item(f);
            idents.push(f.sig.ident.clone());
        });

    let mut output: syn::ItemImpl = syn::parse_quote!{
        impl #self_ty {
            
        }
    };
    // println!("{:?}", &output);
    output.items = methods;

    (output, names, idents)
}

/// transform method to meet the signature of service function
#[cfg(feature = "server")]
pub(crate) fn transform_impl_item(f: &mut syn::ImplItemMethod) {
    // change function ident
    let ident = f.sig.ident.clone();
    let concat_name = format!("{}_{}", &ident.to_string(), HANDLER_SUFFIX);
    let handler_ident = syn::Ident::new(&concat_name, ident.span());

    // change asyncness
    f.sig.asyncness = None;

    // transform function request type
    if let syn::FnArg::Typed(pt) = f.sig.inputs.last().unwrap() {
        let req_ty = &pt.ty;
        let ret_ty = unwrap_return_type(&f.sig.output);

        f.block = syn::parse_quote!({
            toy_rpc_handler!(self, #ident, deserializer, #req_ty, #ret_ty)
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
// #[cfg(any(
//     feature = "server",
//     feature = "client"
// ))]
pub(crate) fn remove_export_attr_from_impl(mut input: syn::ItemImpl) -> syn::ItemImpl {
    input.items.iter_mut().for_each(|item| {
        // clear the attributes for now
        if let syn::ImplItem::Method(f) = item {
            f.attrs.retain(|attr| !is_exported(attr))
        }
    });

    input
}

/// Generate implementation of the `toy_rpc::util::RegisterService` trait.
///
/// The static hashmap of handlers will be returned by `handlers()` method.
/// The service struct name will be returned by `default_name()` method.
///
#[cfg(feature = "server")]
pub(crate) fn impl_register_service_for_struct(
    type_path: &syn::TypePath,
    names: Vec<String>,
    handler_idents: Vec<syn::Ident>,
) -> impl quote::ToTokens {
    let type_ident = parse_type_ident_from_type_path(type_path).unwrap();
    let service_name = type_ident.to_string();
    let ret = quote::quote! {
        impl toy_rpc::util::RegisterService for #type_path {
            fn handlers() -> std::collections::HashMap<&'static str, toy_rpc::service::AsyncHandler<Self>> {
                let mut map = std::collections::HashMap::<&'static str, toy_rpc::service::AsyncHandler<#type_path>>::new();
                #(map.insert(#names, #type_path::#handler_idents);)*;
                map
            }

            fn default_name() -> &'static str {
                #service_name
            }
        }
    };

    ret
}

#[cfg(any(feature = "server", all(feature = "client", feature = "runtime")))]
pub(crate) fn retain_exported_impl_items(input: syn::ItemImpl) -> syn::ItemImpl {
    let mut output = input;
    output.items.retain(|item| match item {
        syn::ImplItem::Method(f) => f.attrs.iter().any(|attr| is_exported(attr)),
        _ => false,
    });
    output
}

#[cfg(any(feature = "server", all(feature = "client", feature = "runtime")))]
pub(crate) fn exported_impl_items(input: syn::ItemImpl) -> Vec<syn::ImplItem> {
    let mut output = input;
    output.items.retain(|item| match item {
        syn::ImplItem::Method(f) => f.attrs.iter().any(|attr| is_exported(attr)),
        _ => false,
    });
    output.items
}

#[cfg(all(feature = "client", feature = "runtime"))]
pub(crate) fn generate_service_client_for_struct(
    type_path: &syn::TypePath,
    input: &syn::ItemImpl,
) -> (syn::Item, proc_macro2::TokenStream) {
    let type_ident = parse_type_ident_from_type_path(type_path).unwrap();
    let concat_name = format!("{}{}", &type_ident.to_string(), CLIENT_SUFFIX);
    let client_ident = syn::Ident::new(&concat_name, type_ident.span());

    let client_struct = syn::parse_quote!(
        pub struct #client_ident<'c, AckMode> {
            client: &'c toy_rpc::client::Client<AckMode>,
            service_name: &'c str,
        }
    );

    let client_impl = client_stub_impl_for_struct(type_ident, &client_ident, input);
    (client_struct, client_impl)
}

/// Generate client stub implementation that allows, conveniently, type checking with the RPC argument
#[cfg(all(feature = "client", feature = "runtime"))]
fn client_stub_impl_for_struct(
    service_ident: &syn::Ident,
    client_ident: &syn::Ident,
    input: &syn::ItemImpl,
) -> proc_macro2::TokenStream {
    let input = retain_exported_impl_items(input.clone());
    let mut generated_items = Vec::new();
    input.items.iter().for_each(|item| {
        if let syn::ImplItem::Method(f) = item {
            if let Some(method) = generate_client_stub_for_struct_method(service_ident, &f) {
                generated_items.push(method);
            }
        }
    });

    let output = quote::quote!(
        impl<'c, AckMode> #client_ident<'c, AckMode> {
            #(#generated_items;)*
        }
    );

    // output.items = generated_items;
    output
}

#[cfg(all(feature = "client", feature = "runtime"))]
pub(crate) fn generate_client_stub_for_struct_method(
    service_ident: &syn::Ident,
    f: &syn::ImplItemMethod,
) -> Option<proc_macro2::TokenStream> {
    if let syn::FnArg::Typed(pt) = f.sig.inputs.last().unwrap() {
        let fn_ident = &f.sig.ident;
        let req_ty = &pt.ty;
        let ret_ty = &f.sig.output;

        return Some(generate_client_stub_for_struct_method_impl(
            service_ident,
            fn_ident,
            &req_ty,
            &ret_ty,
        ));
    }

    None
}

/// Generate client stub for the service impl block
#[cfg(all(feature = "client", feature = "runtime"))]
pub(crate) fn generate_client_stub_for_struct(
    type_path: &syn::TypePath,
) -> (syn::Item, syn::ItemImpl) {
    let type_ident = parse_type_ident_from_type_path(type_path).unwrap();
    let concat_name = format!("{}{}", &type_ident.to_string(), CLIENT_SUFFIX);
    let client_ident = syn::Ident::new(&concat_name, type_ident.span());

    // client stub
    let concat_name = format!("{}{}", &type_ident.to_string(), CLIENT_STUB_SUFFIX);
    let stub_ident = syn::Ident::new(&concat_name, type_ident.span());
    let stub_fn = parse_stub_fn_name(type_ident);

    let stub_trait = syn::parse_quote!(
        pub trait #stub_ident<AckMode> {
            fn #stub_fn<'c>(&'c self) -> #client_ident<AckMode>;
        }
    );

    let service_name = type_ident.to_string();
    let stub_impl: syn::ItemImpl = syn::parse_quote!(
        impl<AckMode> #stub_ident<AckMode> for toy_rpc::client::Client<AckMode> {
            fn #stub_fn<'c>(&'c self) -> #client_ident<AckMode> {
                #client_ident {
                    client: self,
                    service_name: #service_name,
                }
            }
        }
    );

    (stub_trait, stub_impl)
}
