#[cfg(feature = "server")]
const REGISTRY_SUFFIX: &str = "Registry";

use super::*;

#[cfg(feature = "server")]
pub(crate) fn transform_trait(
    input: syn::ItemTrait,
) -> (syn::ItemTrait, syn::ItemImpl, Vec<String>, Vec<syn::Ident>) {
    let mut names: Vec<String> = Vec::new();
    let mut idents: Vec<syn::Ident> = Vec::new();
    let mut handler_idents = Vec::new();
    let input = filter_exported_trait_items(input.clone());
    let trait_ident = &input.ident;

    let concat_name = format!("{}{}", &input.ident.to_string(), EXPORTED_TRAIT_SUFFIX);
    let transformed_trait_ident = syn::Ident::new(&&concat_name, input.ident.span());
    input.items.iter().for_each(|item| {
        if let syn::TraitItem::Method(f) = item {
            names.push(f.sig.ident.to_string());
            idents.push(f.sig.ident.clone());
        }
    });
    let mut transformed_trait: syn::ItemTrait = syn::parse_quote!(
        pub trait #transformed_trait_ident: Send + Sync {

        }
    );
    for item in input.items.iter() {
        if let syn::TraitItem::Method(f) = item {
            let f_gen = generate_transformed_trait_item(&f.sig.ident);
            handler_idents.push(f_gen.sig.ident.clone());
            let item_gen = syn::TraitItem::Method(f_gen);
            transformed_trait.items.push(item_gen);
        }
    }

    let transformed_trait_impl: syn::ItemImpl = syn::parse_quote!(
        impl<T: #trait_ident + Send + Sync + 'static> #transformed_trait_ident for T {

        }
    );
    let transformed_trait_impl =
        impl_transformed_trait(transformed_trait_impl, &transformed_trait, &input);

    (
        transformed_trait,
        transformed_trait_impl,
        names,
        handler_idents,
    )
}

#[cfg(feature = "server")]
fn generate_transformed_trait_item(ident: &syn::Ident) -> syn::TraitItemMethod {
    let concat_name = format!("{}_{}", &ident.to_string(), HANDLER_SUFFIX);
    let handler_ident = syn::Ident::new(&concat_name, ident.span());

    syn::parse_quote!(
        fn #handler_ident(
            self: std::sync::Arc<Self>,
            deserializer: Box<dyn toy_rpc::erased_serde::Deserializer<'static> + Send>
        ) -> toy_rpc::service::HandlerResultFut;
    )
}

#[cfg(feature = "server")]
fn impl_transformed_trait(
    mut trait_impl: syn::ItemImpl,
    handler_trait: &syn::ItemTrait,
    orig_trait: &syn::ItemTrait,
) -> syn::ItemImpl {
    let handler_items = handler_trait.items.iter().filter_map(|item| {
        if let syn::TraitItem::Method(f) = item {
            Some(f)
        } else {
            None
        }
    });
    let orig_items = orig_trait.items.iter().filter_map(|item| {
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
            // let ret_ty: syn::Type = match &orig_item.sig.output {
            //     syn::ReturnType::Default => syn::parse_quote! {()},
            //     syn::ReturnType::Type(_, ty) => syn::parse_quote! {#ty}
            // };
            let handler_ident = &handler_item.sig.ident;
            let orig_ident = &orig_item.sig.ident;

            let ret_ty = unwrap_return_type(&orig_item.sig.output );

            let f: syn::ImplItemMethod = syn::parse_quote!(
                fn #handler_ident(
                    self: std::sync::Arc<Self>,
                    mut deserializer: Box<dyn toy_rpc::erased_serde::Deserializer<'static> + Send>
                ) -> toy_rpc::service::HandlerResultFut
                {
                    // Box::pin(
                    //     async move {
                    //         let req: #req_ty = toy_rpc::erased_serde::deserialize(&mut deserializer)
                    //             .map_err(|e| toy_rpc::error::Error::ParseError(Box::new(e)))?;
                    //         let outcome = self.#orig_ident(req).await;
                    //         Ok(Box::new(outcome) as toy_rpc::service::Outcome)
                    //     }
                    // )
                    toy_rpc_handler!(self, #orig_ident, deserializer, #req_ty, #ret_ty)
                }
            );
            trait_impl.items.push(syn::ImplItem::Method(f));
        }
    }

    trait_impl
}

// #[cfg(any(
//     feature = "server",
//     feature = "client"
// ))]
pub(crate) fn remove_export_attr_from_trait(mut input: syn::ItemTrait) -> syn::ItemTrait {
    input.items.iter_mut().for_each(|item| {
        if let syn::TraitItem::Method(f) = item {
            f.attrs.retain(|attr| !is_exported(attr))
        }
    });

    input
}

#[cfg(feature = "server")]
pub(crate) fn impl_local_registry_for_trait(
    orig_trait_ident: &syn::Ident,
    transformed_trait_ident: &syn::Ident,
    names: Vec<String>,
    handler_idents: Vec<syn::Ident>,
) -> impl quote::ToTokens {
    let service_name = orig_trait_ident.to_string();
    let concat_name = format!("{}{}", transformed_trait_ident.to_string(), REGISTRY_SUFFIX);
    let registry_ident = syn::Ident::new(&concat_name, transformed_trait_ident.span());
    let ret = quote::quote! {
        pub trait #registry_ident {
            fn handlers() -> std::collections::HashMap<&'static str, toy_rpc::service::AsyncHandler<Self>>;
            fn default_name() -> &'static str;
        }

        impl<T> #registry_ident for T
        where
            T: #transformed_trait_ident + Send + Sync + 'static
        {
            fn handlers() -> std::collections::HashMap<&'static str, toy_rpc::service::AsyncHandler<Self>> {
                let mut map = std::collections::HashMap::<&'static str, toy_rpc::service::AsyncHandler<Self>>::new();
                #(map.insert(#names, Self::#handler_idents);)*;
                map
            }

            fn default_name() -> &'static str {
                #service_name
            }
        }
    };
    ret
}

#[cfg(feature = "server")]
pub(crate) fn impl_register_service_for_trait_impl(
    trait_path: &syn::Path,
    type_path: &syn::TypePath,
) -> impl quote::ToTokens {
    let trait_ident = &trait_path
        .segments
        .last()
        .expect("Expecting a trait identifier")
        .ident;
    let concat_name = format!("{}{}", &trait_ident.to_string(), EXPORTED_TRAIT_SUFFIX);
    let transformed_trait_ident = syn::Ident::new(&&concat_name, trait_ident.span());
    let registry_name = format!("{}{}", transformed_trait_ident, REGISTRY_SUFFIX);
    let registry_ident = syn::Ident::new(&registry_name, transformed_trait_ident.span());

    let ret = quote::quote! {
        impl toy_rpc::util::RegisterService for #type_path {
            fn handlers() -> std::collections::HashMap<&'static str, toy_rpc::service::AsyncHandler<Self>> {
                <Self as #registry_ident>::handlers()
            }

            fn default_name() -> &'static str {
                <Self as #registry_ident>::default_name()
            }
        }
    };
    ret
}

// #[cfg(feature = "server")]
// pub(crate) fn get_trait_ident_from_item_impl(input: &syn::ItemImpl) -> Option<syn::Ident> {

//     if let Some((_, ref path, _)) = input.trait_ {
//         println!("path: {:?}", path);
//         path.get_ident().map(|id| id.clone())
//     } else {
//         None
//     }
// }

#[cfg(any(feature = "server", all(feature = "client", feature = "runtime",)))]
pub(crate) fn filter_exported_trait_items(input: syn::ItemTrait) -> syn::ItemTrait {
    let mut output = input;
    output.items.retain(|item| match item {
        syn::TraitItem::Method(f) => f.attrs.iter().any(|attr| is_exported(attr)),
        _ => false,
    });

    output
}

#[cfg(all(feature = "client", feature = "runtime"))]
pub(crate) fn generate_service_client_for_trait(
    trait_ident: &syn::Ident,
    input: &syn::ItemTrait,
) -> (syn::Item, proc_macro2::TokenStream) {
    let concat_name = format!("{}{}", &trait_ident.to_string(), CLIENT_SUFFIX);
    let client_ident = syn::Ident::new(&&concat_name, trait_ident.span());

    let client_struct: syn::Item = syn::parse_quote!(
        pub struct #client_ident<'c, AckMode> {
            client: &'c toy_rpc::client::Client<AckMode>,
            service_name: &'c str,
        }
    );
    let client_impl = client_stub_impl_for_trait(trait_ident, &client_ident, input);
    (client_struct, client_impl)
}

#[cfg(all(feature = "client", feature = "runtime"))]
fn client_stub_impl_for_trait(
    service_ident: &syn::Ident,
    client_ident: &syn::Ident,
    input: &syn::ItemTrait,
) -> proc_macro2::TokenStream {
    let input = filter_exported_trait_items(input.clone());
    let mut generated_items = Vec::new();
    input.items.iter().for_each(|item| {
        if let syn::TraitItem::Method(f) = item {
            if let Some(method) = generate_client_stub_for_trait_method(service_ident, f) {
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
fn generate_client_stub_for_trait_method(
    service_ident: &syn::Ident,
    f: &syn::TraitItemMethod,
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

#[cfg(all(feature = "client", feature = "runtime"))]
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
        pub trait #stub_ident<AckMode> {
            fn #stub_fn<'c>(&'c self) -> #client_ident<AckMode>;
        }
    );

    let service_name = trait_ident.to_string();
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

#[cfg(all(feature = "client", feature = "runtime"))]
pub fn generate_trait_impl_for_client(input: &syn::ItemTrait) -> syn::ItemImpl {
    let service_ident = &input.ident;
    let input = filter_exported_trait_items(input.clone());
    let mut generated_items: Vec<syn::ImplItem> = Vec::new();
    input.items.iter().for_each(|item| {
        if let syn::TraitItem::Method(f) = item {
            generated_items.push(syn::ImplItem::Method(
                generate_trait_method_impl_for_client(service_ident, f),
            ))
        }
    });
    let mut output: syn::ItemImpl = syn::parse_quote!(
        impl<AckMode: Send + Sync> #service_ident for toy_rpc::client::Client<AckMode> {

        }
    );
    output.items = generated_items;
    output
}

///
/// PANIC: panics if the argument ident is not found
#[cfg(all(feature = "client", feature = "runtime"))]
fn generate_trait_method_impl_for_client(
    service_ident: &syn::Ident,
    method: &syn::TraitItemMethod,
) -> syn::ImplItemMethod {
    use std::ops::Deref;

    let method_ident = &method.sig.ident;
    let arg = method.sig.inputs.last().unwrap();
    let arg_ident = match arg {
        syn::FnArg::Typed(pt) => {
            if let syn::Pat::Ident(pat_id) = pt.pat.deref() {
                &pat_id.ident
            } else {
                panic!("Argument ident not found")
            }
        }
        _ => panic!("Argument ident not found"),
    };
    let service_method = format!("{}.{}", service_ident, method_ident);
    let ret_ty = unwrap_return_type(&method.sig.output);
    

    let block: syn::Block = syn::parse_quote!(
        {
            // Box::pin(
            //     async move {
            //         let success = self.call(#service_method, #arg_ident).await?;
            //         Ok(success)
            //     }
            // )
            toy_rpc_client_stub!(self, #service_method, #arg_ident, #ret_ty)
        }
    );

    syn::ImplItemMethod {
        attrs: method.attrs.clone(),
        vis: syn::Visibility::Inherited,
        defaultness: None,
        sig: method.sig.clone(),
        block,
    }
}
