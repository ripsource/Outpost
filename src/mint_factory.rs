use scrypto::prelude::*;


use crate::minter::royal_nft::*;

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct FreshMint {
    pub mint_component: ComponentAddress,
    pub resource_address: ResourceAddress,
}


#[blueprint]
#[events(FreshMint)]
mod mint_factory {
    use crate::minter::{MintingConfig, NFTMetadata, RoyaltyConfigInput};


    struct MintFactory {
        dapp_deff: ComponentAddress,
    }

    impl MintFactory {
        pub fn start_mint_factory(dapp_definition: ComponentAddress) -> (Global<MintFactory>, Bucket) {


            let (address_reservation, component_address) =
                Runtime::allocate_component_address(MintFactory::blueprint_id());

            // let global_caller_badge_rule = rule!(require(global_caller(component_address)));

            let mint_factory_admin: Bucket = ResourceBuilder::new_fungible(OwnerRole::None)
              .metadata(metadata!(
                roles {
                  metadata_setter => rule!(deny_all);
                  metadata_setter_updater => rule!(deny_all);
                  metadata_locker => rule!(deny_all);
                  metadata_locker_updater => rule!(deny_all);
                },
                init {
                    "name" => "Mint Factory Admin".to_owned(), locked;
                    "description" => "Mint Factory Admin Badge".to_owned(), locked;
                    "icon_url" => Url::of("https://outpostdocs.netlify.app/img/outpost_symbol.png"), locked;
                }
              ))
                .divisibility(0)
                .mint_initial_supply(1).into();

            let admin_rule = rule!(require(mint_factory_admin.resource_address()));

(
            Self {
               dapp_deff: dapp_definition
               
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::None)
            .metadata(metadata! (
                roles {
                    metadata_setter => admin_rule.clone();
                    metadata_setter_updater => admin_rule.clone();
                    metadata_locker => admin_rule.clone();
                    metadata_locker_updater => admin_rule.clone();
                },
                init {
                    "name" => "OP Mint Factory".to_owned(), updatable;
                    "description" => "The mint factory for Outpost Collections".to_owned(), updatable;
                    "dapp_definition" => dapp_definition, updatable;
                    "icon_url" => Url::of("https://outpostdocs.netlify.app/img/outpost_symbol.png"), updatable;
                }
            ))
            .with_address(address_reservation)
            .globalize(), mint_factory_admin)



        }

        pub fn create_royal_nft(&mut self,
            setup_metadata: NFTMetadata,
            minting_config: MintingConfig,
            royalty_config_input: RoyaltyConfigInput,       
        ) -> (Global<RoyalNFTs>, NonFungibleBucket, ResourceAddress) {


            let fresh_mint: (Global<RoyalNFTs>, NonFungibleBucket, ResourceAddress) = RoyalNFTs::start_minting_nft(
                setup_metadata,
                minting_config,
                royalty_config_input,
                self.dapp_deff
               
            );

            Runtime::emit_event(FreshMint {
                mint_component: fresh_mint.0.address(),
                resource_address: fresh_mint.2,
            });

            fresh_mint
        }
    
    }

}