use super::*;

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
pub(crate) fn transform_impl(input: syn::ItemImpl) -> (syn::ItemImpl, Vec<String>, Vec<syn::Ident>) {
    let mut names = Vec::new();
    let mut idents = Vec::new();
    let mut output = filter_exported_impl_items(input);

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
            transform_impl_item(f);
            idents.push(f.sig.ident.clone());
        });

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

        f.block = syn::parse_quote!({
            Box::pin(
                async move {
                    let req: #req_ty = toy_rpc::erased_serde::deserialize(&mut deserializer)
                        .map_err(|e| toy_rpc::error::Error::ParseError(Box::new(e)))?;
                    self.#ident(req).await
                        .map(|r| Box::new(r) as Box<dyn toy_rpc::erased_serde::Serialize + Send + Sync + 'static>)
                        .map_err(|err| err.into())
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
// #[cfg(any(
//     feature = "server", 
//     feature = "client"
// ))]
pub(crate) fn remove_export_attr_from_impl(mut input: syn::ItemImpl) -> syn::ItemImpl {
    input
        .items
        .iter_mut()
        .for_each(|item| {
            // clear the attributes for now
            if let syn::ImplItem::Method(f) = item {
                f.attrs.retain(|attr| {
                    !is_exported(attr)
                })
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
    struct_ident: &syn::Ident, 
    names: Vec<String>, 
    handler_idents: Vec<syn::Ident>
) -> impl quote::ToTokens {
    let service_name = struct_ident.to_string();
    let ret = quote::quote! {
        impl toy_rpc::util::RegisterService for #struct_ident {
            fn handlers() -> std::collections::HashMap<&'static str, toy_rpc::service::AsyncHandler<Self>> {
                let mut map = std::collections::HashMap::<&'static str, toy_rpc::service::AsyncHandler<#struct_ident>>::new();
                #(map.insert(#names, #struct_ident::#handler_idents);)*;
                map
            }

            fn default_name() -> &'static str {
                #service_name
            }
        }
    };

    ret
}

#[cfg(any(
    feature = "server", 
    all(
        feature = "client",
        feature = "runtime"
    )
))]
pub(crate) fn filter_exported_impl_items(input: syn::ItemImpl) -> syn::ItemImpl {
    let mut output = input;
    output.items.retain(|item| match item {
        syn::ImplItem::Method(f) => {
            f.attrs.iter()
                .any(|attr| is_exported(attr))
        }
        _ => false,
    });
    output
}

#[cfg(all(
    feature = "client",
    feature = "runtime"
))]
pub(crate) fn generate_service_client_for_struct(
    struct_ident: &syn::Ident,
    input: &syn::ItemImpl,
) -> (syn::Item, syn::ItemImpl)
{
    let concat_name = format!("{}{}", &struct_ident.to_string(), CLIENT_SUFFIX);
    let client_ident = syn::Ident::new(&concat_name, struct_ident.span());

    let client_struct = syn::parse_quote!(
        pub struct #client_ident<'c> {
            client: &'c toy_rpc::client::Client,
            service_name: &'c str,
        }
    );

    let client_impl = client_stub_impl_for_struct(struct_ident, &client_ident, input);
    (client_struct, client_impl)
}

/// Generate client stub implementation that allows, conveniently, type checking with the RPC argument
#[cfg(all(
    feature = "client",
    feature = "runtime"
))]
fn client_stub_impl_for_struct(
    service_ident: &syn::Ident,
    client_ident: &syn::Ident,
    input: &syn::ItemImpl,
) -> syn::ItemImpl {
    let input = filter_exported_impl_items(input.clone());
    let mut generated_items: Vec<syn::ImplItem> = Vec::new();
    input
        .items
        .iter()
        .for_each(|item| {
            if let syn::ImplItem::Method(f) = item {
                if let Some(method) = generate_client_stub_for_struct_method(service_ident, f) {
                    generated_items.push(syn::ImplItem::Method(method));
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

#[cfg(all(
    feature = "client",
    feature = "runtime"
))]
pub(crate) fn generate_client_stub_for_struct_method(
    service_ident: &syn::Ident,
    f: &syn::ImplItemMethod,
) -> Option<syn::ImplItemMethod> {
    if let syn::FnArg::Typed(pt) = f.sig.inputs.last().unwrap() {
        let fn_ident = &f.sig.ident;
        let req_ty = &pt.ty;

        if let syn::ReturnType::Type(_, ret_ty) = f.sig.output.clone() {
            let ok_ty = get_ok_ident_from_type(ret_ty)?;
            return Some(generate_client_stub_for_struct_method_impl(
                service_ident,
                fn_ident,
                &req_ty,
                &ok_ty,
            ));
        }
    }

    None
}

/// Generate client stub for the service impl block
#[cfg(all(
    feature = "client",
    feature = "runtime"
))]
pub(crate) fn generate_client_stub_for_struct(
    struct_ident: &syn::Ident,
) -> (syn::Item, syn::ItemImpl) {
    let concat_name = format!("{}{}", &struct_ident.to_string(), CLIENT_SUFFIX);
    let client_ident = syn::Ident::new(&concat_name, struct_ident.span());

    // client stub
    let concat_name = format!("{}{}", &struct_ident.to_string(), CLIENT_STUB_SUFFIX);
    let stub_ident = syn::Ident::new(&concat_name, struct_ident.span());
    let stub_fn = parse_stub_fn_name(struct_ident);

    let stub_trait = syn::parse_quote!(
        pub trait #stub_ident {
            fn #stub_fn<'c>(&'c self) -> #client_ident;
        }
    );

    let service_name = struct_ident.to_string();
    let stub_impl: syn::ItemImpl = syn::parse_quote!(
        impl #stub_ident for toy_rpc::client::Client {
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
