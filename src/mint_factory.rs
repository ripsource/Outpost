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
        admin: ResourceAddress,
    }

    impl MintFactory {
        pub fn start_mint_factory(
            dapp_definition: ComponentAddress,
            admin: ResourceAddress,
        ) -> (Global<MintFactory>, Bucket) {
            let (address_reservation, _component_address) =
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
                    "icon_url" => Url::of("https://www.outpost.trade/img/outpost_symbol.png"), locked;
                }
              ))
                .divisibility(0)
                .mint_initial_supply(1).into();

            let admin_rule = rule!(require(mint_factory_admin.resource_address()));

            (
            Self {
               dapp_deff: dapp_definition,
               admin
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
                    "icon_url" => Url::of("https://www.outpost.trade/img/outpost_symbol.png"), updatable;
                }
            ))
            .with_address(address_reservation)
            .globalize(), mint_factory_admin)
        }

        pub fn create_royal_nft(
            &mut self,
            setup_metadata: NFTMetadata,
            minting_config: MintingConfig,
            royalty_config_input: RoyaltyConfigInput,
        ) -> (Global<RoyalNFTs>, NonFungibleBucket, ResourceAddress) {
            let name_string = format!("OP Mint {}", setup_metadata.name);

            let dapp_def_account =
                Blueprint::<Account>::create_advanced(OwnerRole::Updatable(rule!(allow_all)), None); // will reset owner role after dapp def metadata has been set
            dapp_def_account.set_metadata("account_type", String::from("dapp definition"));
            dapp_def_account.set_metadata("name", name_string);
            dapp_def_account.set_metadata(
                "description",
                "A minting and royalty logic component".to_string(),
            );
            dapp_def_account.set_metadata("collection", setup_metadata.name.clone());
            dapp_def_account.set_metadata("info_url", Url::of("https://www.outpost.trade/"));
            dapp_def_account.set_metadata(
                "icon_url",
                Url::of("https://www.outpost.trade/img/outpost_symbol.png"),
            );

            let dapp_def_address = GlobalAddress::from(dapp_def_account.address());

            let fresh_mint: (Global<RoyalNFTs>, NonFungibleBucket, ResourceAddress) =
                RoyalNFTs::start_minting_nft(
                    setup_metadata,
                    minting_config,
                    royalty_config_input,
                    dapp_def_address,
                );

            dapp_def_account.set_metadata(
                "claimed_entities",
                vec![GlobalAddress::from(fresh_mint.0.address())],
            );
            dapp_def_account.set_owner_role(rule!(require(self.admin)));

            Runtime::emit_event(FreshMint {
                mint_component: fresh_mint.0.address(),
                resource_address: fresh_mint.2,
            });

            fresh_mint
        }
    }
}
