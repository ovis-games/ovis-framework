#![feature(proc_macro_quote)]

use proc_macro::{TokenStream, quote};
use proc_macro2::Span;
use syn::__private::ToTokens;

#[proc_macro_attribute]
pub fn resource(attribute: TokenStream, item: TokenStream) -> TokenStream {
    // if let Ok(item_type) = syn::parse::<syn::ItemType>(item.clone()) {
    //     let identifier = item_type.ident.to_token_stream();
    //     let ty = item_type.ty.to_token_stream();

    //     return quote!(
    //         #[resource($attribute)]
    //         pub struct $identifier {
    //             inner: $ty,
    //         }

    //         impl From<$ty> for $identifier {
    //             fn from(value: $ty) -> Self {
    //                 return Self { inner: value };
    //             }
    //         }

    //         impl std::ops::Deref for $identifier {
    //             type Target = $ty;

    //             fn deref(&self) -> &Self::Target {
    //                 return &self.inner;
    //             }
    //         }

    //         impl std::ops::DerefMut for $identifier {
    //             fn deref_mut(&mut self) -> &mut Self::Target {
    //                 return &mut self.inner;
    //             }
    //         }
    //     );
    // } else 
    if let Ok(struct_type) = syn::parse::<syn::ItemStruct>(item.clone()) {
        let identifier = struct_type.ident.to_string();
        let resource_ident = struct_type.ident;
        let resource_id_ident = syn::Ident::new(&format!("{}_ID", identifier.to_string().to_uppercase()), Span::call_site()).to_token_stream();
        let resource_ident = resource_ident.to_token_stream();
        let resource_label = format!("{}_{}", std::env::var("CARGO_CRATE_NAME").unwrap(), resource_ident);
        let resource_label = syn::LitStr::new(&resource_label, Span::call_site()).to_token_stream();
        

        return quote!(
            $item

            use ovis_core::{Resource, ResourceId, ResourceKind, IdMappedResourceStorage, EntityId, register_resource, ResourceStorage, Gpu};
            use std::sync::Arc;
            static mut $resource_id_ident: ResourceId = ResourceId::from_index_and_version(0, 0);

            impl Resource for $resource_ident {
                fn id() -> ResourceId { unsafe { $resource_id_ident } }
                fn kind() -> ResourceKind { ResourceKind::$attribute }
                fn label() -> &'static str { $resource_label }
                fn register() { unsafe { $resource_id_ident = register_resource::<Self>(); } }
                fn make_storage(gpus: &[Arc<Gpu>]) -> ResourceStorage {
                    return ResourceStorage::EntityComponent(Box::new(IdMappedResourceStorage::<EntityId, $resource_ident>::new(gpus, Self::id())));
                }
            }
        );
    } else {
        panic!("expected type");
    }
}


#[proc_macro_attribute]
pub fn array_resource(attribute: TokenStream, item: TokenStream) -> TokenStream {
    if let Ok(struct_type) = syn::parse::<syn::ItemStruct>(item.clone()) {
        let identifier = struct_type.ident.to_string();
        let resource_ident = struct_type.ident;
        let resource_id_ident = syn::Ident::new(&format!("{}_ID", identifier.to_string().to_uppercase()), Span::call_site()).to_token_stream();
        let resource_ident = resource_ident.to_token_stream();
        let resource_label = format!("{}_{}", std::env::var("CARGO_CRATE_NAME").unwrap(), resource_ident);
        let resource_label = syn::LitStr::new(&resource_label, Span::call_site()).to_token_stream();

        return quote!(
            $item

            use ovis_core::{Resource, ResourceId, ResourceKind, IdMappedResourceSliceStorage, EntityId, register_resource, ResourceStorage, Gpu};
            use std::sync::Arc;
            static mut $resource_id_ident: ResourceId = ResourceId::from_index_and_version(0, 0);

            impl Resource for $resource_ident {
                fn id() -> ResourceId { unsafe { $resource_id_ident } }
                fn kind() -> ResourceKind { ResourceKind::$attribute }
                fn label() -> &'static str { $resource_label }
                fn register() { unsafe { $resource_id_ident = register_resource::<Self>(); } }
                fn make_storage(gpus: &[Arc<Gpu>]) -> ResourceStorage {
                    return ResourceStorage::EntityComponent(Box::new(IdMappedResourceSliceStorage::<EntityId, $resource_ident>::new(gpus, Self::id())));
                }
            }
        );
    } else {
        panic!("expected type");
    }
}

#[proc_macro_attribute]
pub fn job(attribute: TokenStream, item: TokenStream) -> TokenStream {
    return item;
    // if let Ok(struct_type) = syn::parse::<syn::ItemFn>(item.clone()) {
    //     let identifier = struct_type.ident.to_string();
    //     let resource_ident = struct_type.ident;
    //     let resource_id_ident = syn::Ident::new(&format!("{}_ID", identifier.to_string().to_uppercase()), Span::call_site()).to_token_stream();
    //     let resource_ident = resource_ident.to_token_stream();
    //     let resource_label = format!("{}_{}", std::env::var("CARGO_CRATE_NAME").unwrap(), resource_ident);
    //     let resource_label = syn::LitStr::new(&resource_label, Span::call_site()).to_token_stream();

    //     return quote!(
    //         $item

    //         use ovis_core::{Resource, ResourceId, ResourceKind, IdMappedResourceSliceStorage, EntityId, register_resource, ResourceStorage, Gpu};
    //         use std::sync::Arc;
    //         static mut $resource_id_ident: ResourceId = ResourceId::from_index_and_version(0, 0);

    //         impl Resource for $resource_ident {
    //             fn id() -> ResourceId { unsafe { $resource_id_ident } }
    //             fn kind() -> ResourceKind { ResourceKind::$attribute }
    //             fn label() -> &'static str { $resource_label }
    //             fn register() { unsafe { $resource_id_ident = register_resource::<Self>(); } }
    //             fn make_storage(gpus: &[Arc<Gpu>]) -> ResourceStorage {
    //                 return ResourceStorage::EntityComponent(Box::new(IdMappedResourceSliceStorage::<EntityId, $resource_ident>::new(gpus, Self::id())));
    //             }
    //         }
    //     );
    // } else {
    //     panic!("expected type");
    // }
}
