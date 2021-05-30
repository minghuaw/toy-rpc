use super::*;

#[cfg(feature = "server")]
pub(crate) fn transform_trait(input: syn::ItemTrait) -> (syn::ItemTrait, syn::ItemImpl, Vec<String>, Vec<syn::Ident>) {
    let mut names: Vec<String> = Vec::new();
    let mut idents: Vec<syn::Ident> = Vec::new();
    let mut transformed_trait = filter_exported_trait_items(input.clone());
    let trait_ident = &input.ident;

    let concat_name = format!("{}{}", &transformed_trait.ident.to_string(), EXPORTED_TRAIT_SUFFIX);
    let transformed_ident = syn::Ident::new(&&concat_name, transformed_trait.ident.span());
    transformed_trait.ident = transformed_ident.clone();
    transformed_trait.items
        .iter_mut()
        .for_each(|item| 
            if let syn::TraitItem::Method(f) = item {
                names.push(f.sig.ident.to_string());
                transform_trait_item(f);
                idents.push(f.sig.ident.clone());
            }
        );

    let transformed_trait_impl: syn::ItemImpl = syn::parse_quote!(
        impl<T: #trait_ident> #transformed_ident for T {

        }
    );
    let transformed_trait_impl = impl_transformed_trait(
        transformed_trait_impl, 
        &transformed_trait, 
        &input
    );

    (transformed_trait, transformed_trait_impl, names, idents)
}

#[cfg(feature = "server")]
fn transform_trait_item(f: &mut syn::TraitItemMethod) {
    let ident = f.sig.ident.clone();
    let concat_name = format!("{}_{}", &ident.to_string(), HANDLER_SUFFIX);
    let handler_ident = syn::Ident::new(&concat_name, ident.span());

    // change asyncness
    f.sig.asyncness = None;

    if let syn::FnArg::Typed(_pt) = f.sig.inputs.last().unwrap() {
        f.sig.inputs = syn::parse_quote!(
            self: std::sync::Arc<Self>, deserializer: Box<dyn toy_rpc::erased_serde::Deserializer<'static> + Send>
        );
        f.sig.output = syn::parse_quote!(
            -> toy_rpc::service::HandlerResultFut
        );
    }

    f.sig.ident = handler_ident;
}

#[cfg(feature = "server")]
fn impl_transformed_trait(
    mut trait_impl: syn::ItemImpl, 
    handler_trait: &syn::ItemTrait,
    orig_trait: &syn::ItemTrait
) -> syn::ItemImpl {
    let handler_items = handler_trait.items.iter()
        .filter_map(|item| 
            if let syn::TraitItem::Method(f) = item {
                Some(f)
            } else {
                None
            }
        );
    let orig_items = orig_trait.items.iter()
        .filter_map(|item| {
            if let syn::TraitItem::Method(f) = item {
                Some(f)
            } else {
                None
            }
        });
    let items = handler_items.zip(orig_items);
    for (handler_item, orig_item) in items {
        if let syn::FnArg::Typed(pt) = orig_item.sig.inputs.last().unwrap() {
            let req_ty = &pt.ty;
            let handler_ident = &handler_item.sig.ident;
            let orig_ident = &orig_item.sig.ident;

            let f: syn::ImplItemMethod = syn::parse_quote!(
                fn #handler_ident(
                    self: std::sync::Arc<Self>, 
                    mut deserializer: Box<dyn toy_rpc::erased_serde::Deserializer<'static> + Send>
                ) -> toy_rpc::service::HandlerResultFut
                {
                    Box::pin(
                        async move {
                            let req: #req_ty = toy_rpc::erased_serde::deserialize(&mut deserializer)
                                .map_err(|e| toy_rpc::error::Error::ParseError(Box::new(e)))?;
                            self.#orig_ident(req).await 
                                .map(|r| Box::new(r) as Box<dyn toy_rpc::erased_serde::Serialize + Send + Sync + 'static>)
                                .map_err(|e| toy_rpc::error::Error::ExecutionError(e.to_string()));
                        }
                    )
                }
            );
            trait_impl.items.push(syn::ImplItem::Method(f));
        }
    }

    trait_impl
}

#[cfg(any(feature = "server", feature = "client"))]
pub(crate) fn remove_export_attr_from_trait(mut input: syn::ItemTrait) -> syn::ItemTrait {
    input 
        .items
        .iter_mut()
        .for_each(|item| {
            if let syn::TraitItem::Method(f) = item {
                f.attrs.retain(|attr|
                    !is_exported(attr)
                )
            }
        });

    input
}

#[cfg(feature = "server")]
pub(crate) fn impl_register_service_for_trait(
    orig_trait_ident: &syn::Ident,
    names: Vec<String>,
    handler_idents: Vec<syn::Ident>
) -> impl quote::ToTokens {
    let service_name = orig_trait_ident.to_string();
    let ret = quote::quote! {
        impl<T: #orig_trait_ident> toy_rpc::util::RegisterService for T {
            fn handlers() -> std::collections::HashMap<&'static str, toy_rpc::service::AsyncHandler<T>> {
                let mut map = std::collections::HashMap::<&'static str, toy_rpc::service::AsyncHandler<T>>::new();
                #(map.insert(#names, T::#handler_idents);)*;
                map
            }

            fn default_name() -> &'static str {
                #service_name
            }
        }
    };
    ret
}

#[cfg(any(feature = "server", feature = "client"))]
pub(crate) fn filter_exported_trait_items(input: syn::ItemTrait) -> syn::ItemTrait {
    let mut output = input;
    output.items.retain(|item| match item {
        syn::TraitItem::Method(f) => {
            f.attrs.iter()
                .any(|attr| is_exported(attr))
        },
        _ => false
    });

    output
}

#[cfg(feature = "client")]
pub(crate) fn generate_service_client_for_trait(
    trait_ident: &syn::Ident,
    input: &syn::ItemTrait
) -> (syn::Item, syn::ItemImpl)
{
    let concat_name = format!("{}{}", &trait_ident.to_string(), CLIENT_SUFFIX);
    let client_ident = syn::Ident::new(&&concat_name, trait_ident.span());

    let client_struct: syn::Item = syn::parse_quote!(
        pub struct #client_ident<'c> {
            client: &'c toy_rpc::client::Client<toy_rpc::client::Connected>,
            service_name: &'c str,
        }
    );
    let client_impl = client_stub_impl_for_trait(trait_ident, &client_ident, input);
    (client_struct, client_impl)
}

#[cfg(feature = "client")]
fn client_stub_impl_for_trait(
    service_ident: &syn::Ident,
    client_ident: &syn::Ident,
    input: &syn::ItemTrait,
) -> syn::ItemImpl {
    let input = filter_exported_trait_items(input.clone());
    let mut generated_items: Vec<syn::ImplItem> = Vec::new();
    input.items.iter()
        .for_each(|item| {
            if let syn::TraitItem::Method(f) = item {
                if let Some(method) = generate_client_stub_for_trait_method(service_ident, f) {
                    generated_items.push(syn::ImplItem::Method(method))
                }
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
fn generate_client_stub_for_trait_method(
    service_ident: &syn::Ident,
    f: &syn::TraitItemMethod
) -> Option<syn::ImplItemMethod> {
    if let syn::FnArg::Typed(pt) = f.sig.inputs.last().unwrap() {
        let fn_ident = &f.sig.ident;
        let req_ty = &pt.ty;

        if let syn::ReturnType::Type(_, ret_ty) = f.sig.output.clone() {
            let ok_ty = get_ok_ident_from_type(ret_ty)?;
            return Some(generate_client_stub_for_struct_method_impl(
                service_ident, fn_ident, &req_ty, &ok_ty))
        }
    }

    None
}

#[cfg(feature = "client")]
pub(crate) fn generate_client_stub_for_trait(
    trait_ident: &syn::Ident,
) -> (syn::Item, syn::ItemImpl) {
    let concat_name = format!("{}{}", &trait_ident.to_string(), CLIENT_SUFFIX);
    let client_ident = syn::Ident::new(&&concat_name, trait_ident.span());

    // client stub
    let concat_name = format!("{}{}", &trait_ident.to_string(), CLIENT_STUB_SUFFIX);
    let stub_ident = syn::Ident::new(&concat_name, trait_ident.span());
    let stub_fn = parse_stub_fn_name(trait_ident);

    let stub_trait: syn::Item = syn::parse_quote!(
        pub trait #stub_ident {
            fn #stub_fn<'c>(&'c self) -> #client_ident;
        }
    );

    let service_name = trait_ident.to_string();
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

    (stub_trait, stub_impl)
}