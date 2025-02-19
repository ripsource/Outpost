use crate::outpost_account::opentrader::OpenTrader;
use crate::outpost_event::event;
use crate::outpost_event::event::Event;
use scrypto::prelude::*;

// This blueprint creates all the open trader accounts. It creates emitter badges that are used to authenticate event emitters from each trader acccount and allows
// traders to buy and sell Royalty NFTs. It also creates a personal key for each trader account that is used to access their account/make listings, update listings,
// and cancel listings.

#[derive(ScryptoSbor, NonFungibleData)]
struct TraderKey {
    name: String,
    description: String,
    key_image_url: Url,
    #[mutable]
    hub: Option<ComponentAddress>,
}

#[derive(ScryptoSbor, NonFungibleData)]
struct EmitterKey {
    name: String,
}

#[derive(ScryptoSbor, ScryptoEvent)]
struct OutpostCreated {
    outpost_component: ComponentAddress,
    outpost_account: ComponentAddress,
}

#[blueprint]
#[events(OutpostCreated)]
mod openhub {

    struct OpenHub {
        /// The badge that is stored and locked in a trader account to authenticate event emitters
        emitter_trader_badge: NonFungibleResourceManager,
        /// The personal user badge that a user holds and uses to authenticate methods on their trading account
        outpost_account_badge: NonFungibleResourceManager,
        /// The badge that is used to allow trader accounts to hold and trade Royalty NFTs
        royal_nft_depositer_badge: FungibleResourceManager,
        /// Event emitter component
        event_manager: Global<event::Event>,
        /// Hub Component Address
        component_address: ComponentAddress,
        /// AccountLocker for all traders
        account_locker: Global<AccountLocker>,
        /// Created accounts
        registered_accounts: KeyValueStore<ComponentAddress, ComponentAddress>,
        // Transient Tokens
        transient_token_manager: FungibleResourceManager,
        // package admin
        admin: ResourceAddress,
    }

    impl OpenHub {
        /// Instantiation of the open hub component creates the resource managers of all the key badges used in the system
        /// which are minted when a user creates a trading account for themselves.
        pub fn start_open_hub(dapp_defintion: ComponentAddress) -> (Global<OpenHub>, Bucket) {
            let (address_reservation, component_address) =
                Runtime::allocate_component_address(OpenHub::blueprint_id());

            let _global_caller_badge_rule = rule!(require(global_caller(component_address)));

            let open_hub_admin: Bucket = ResourceBuilder::new_fungible(OwnerRole::None)
              .metadata(metadata!(
                roles {
                  metadata_setter => rule!(deny_all);
                  metadata_setter_updater => rule!(deny_all);
                  metadata_locker => rule!(deny_all);
                  metadata_locker_updater => rule!(deny_all);
                },
                init {
                    "name" => "Outpost Admin".to_owned(), locked;
                    "description" => "Outpost Admin Badge".to_owned(), locked;
                    "icon_url" => Url::of("https://outpostdocs.netlify.app/img/outpost_symbol.png"), locked;
                }
              ))
                .divisibility(0)
                .mint_initial_supply(1).into();

            let global_caller_badge_rule = rule!(require(global_caller(component_address)));

            let admin_rule = rule!(require(open_hub_admin.resource_address()));

            let emitter_trader_badge =
                ResourceBuilder::new_ruid_non_fungible::<EmitterKey>(OwnerRole::None)
                .metadata(metadata!(
                    roles {
                      metadata_setter => rule!(deny_all);
                      metadata_setter_updater => rule!(deny_all);
                      metadata_locker => rule!(deny_all);
                      metadata_locker_updater => rule!(deny_all);
                    },
                    init {
                        "name" => "Outpost Internal Services".to_owned(), locked;
                        "description" => "Outpost Internal Services".to_owned(), locked;
                        "icon_url" => Url::of("https://outpostdocs.netlify.app/img/outpost_symbol.png"), locked;
                    }
                  ))
                    .mint_roles(mint_roles! {
                        minter => global_caller_badge_rule.clone();
                        minter_updater => admin_rule.clone();
                    })
                    .create_with_no_initial_supply();

            let outpost_account_badge =
                ResourceBuilder::new_ruid_non_fungible::<TraderKey>(OwnerRole::None)
                .metadata(metadata!(
                    roles {
                      metadata_setter => rule!(deny_all);
                      metadata_setter_updater => rule!(deny_all);
                      metadata_locker => rule!(deny_all);
                      metadata_locker_updater => rule!(deny_all);
                    },
                    init {
                        "name" => "Outpost Key".to_owned(), locked;
                        "description" => "The key to managing your listings at your outpost".to_owned(), locked;
                        "icon_url" => Url::of("https://outpostdocs.netlify.app/img/outpost_symbol.png"), locked;
                    }
                  ))
                    .mint_roles(mint_roles! {
                        minter => global_caller_badge_rule.clone();
                        minter_updater => admin_rule.clone();
                    })
                    .withdraw_roles(withdraw_roles! {
                        withdrawer => rule!(deny_all);
                        withdrawer_updater => admin_rule.clone();
                    })
                    .non_fungible_data_update_roles(non_fungible_data_update_roles! {
                        non_fungible_data_updater => global_caller_badge_rule.clone();
                        non_fungible_data_updater_updater => admin_rule.clone();
                    })
                    .create_with_no_initial_supply();

            let royal_nft_depositer_badge = ResourceBuilder::new_fungible(OwnerRole::None)
            .metadata(metadata!(
                roles {
                  metadata_setter => rule!(deny_all);
                  metadata_setter_updater => rule!(deny_all);
                  metadata_locker => rule!(deny_all);
                  metadata_locker_updater => rule!(deny_all);
                },
                init {
                    "name" => "Royalty Internal Services".to_owned(), locked;
                    "description" => "Royalty control for outpost internal services".to_owned(), locked;
                    "icon_url" => Url::of("https://outpostdocs.netlify.app/img/outpost_symbol.png"), locked;
                }
              ))
                .mint_roles(mint_roles! {
                    minter => global_caller_badge_rule.clone();
                    minter_updater => admin_rule.clone();
                })
                .divisibility(0)
                .create_with_no_initial_supply();

            let transient_token_manager = ResourceBuilder::new_fungible(OwnerRole::None)
            .metadata(metadata!(
                roles {
                  metadata_setter => rule!(deny_all);
                  metadata_setter_updater => rule!(deny_all);
                  metadata_locker => rule!(deny_all);
                  metadata_locker_updater => rule!(deny_all);
                },
                init {
                    "name" => "Internal transient badge".to_owned(), locked;
                    "description" => "Internal transient badge".to_owned(), locked;
                    "icon_url" => Url::of("https://outpostdocs.netlify.app/img/outpost_symbol.png"), locked;
                }
              ))
                .divisibility(0)
                .mint_roles(mint_roles! {
                    minter => global_caller_badge_rule.clone();
                    minter_updater => admin_rule.clone();
                })
                .deposit_roles(deposit_roles! {
                    depositor => rule!(require(royal_nft_depositer_badge.address()));
                    depositor_updater => rule!(require(royal_nft_depositer_badge.address()));
                })
                .create_with_no_initial_supply();

            let event_manager = Event::create_event_listener(emitter_trader_badge.address());

            let locker_badge_rule = rule!(require(emitter_trader_badge.address()));

            let locker = Blueprint::<AccountLocker>::instantiate(
                OwnerRole::None,    // owner
                locker_badge_rule,  // storer
                admin_rule.clone(), // storer_updater
                rule!(deny_all),    // recoverer
                rule!(deny_all),    // recoverer_updater
                None,               // address_reservation
            );

            (Self {
                emitter_trader_badge,
                outpost_account_badge,
                royal_nft_depositer_badge,
                event_manager,
                component_address,
                account_locker: locker,
                registered_accounts: KeyValueStore::new(),
                transient_token_manager,
                admin: open_hub_admin.resource_address()
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
                    "name" => "Outpost Hub".to_owned(), updatable;
                    "description" => "Outpost Hub".to_owned(), updatable;
                    "dapp_definition" => dapp_defintion, updatable;
                    "icon_url" => Url::of("https://outpostdocs.netlify.app/img/outpost_symbol.png"), updatable;
                }
            ))
            .with_address(address_reservation)
            .globalize(), open_hub_admin)
        }

        /// Creates a new open trader account with a emitter badge, personal key, and a badge to hold and trade Royalty NFTs
        pub fn create_outpost(&self, my_account: Global<Account>) -> (NonFungibleGlobalId, Bucket) {
            {
                // Getting the owner role of the account.
                let owner_role = my_account.get_owner_role();

                // Assert against it.
                Runtime::assert_access_rule(owner_role.rule);

                // Assertion passed - the caller is the owner of the account.
            }

            if self
                .registered_accounts
                .get(&my_account.address())
                .is_some()
            {
                panic!("Account already has created an OT Trading Hub - check your wallet for your hub key.");
            }

            let dapp_def_account =
                Blueprint::<Account>::create_advanced(OwnerRole::Updatable(rule!(allow_all)), None); // will reset owner role after dapp def metadata has been set
            dapp_def_account.set_metadata("account_type", String::from("dapp definition"));
            dapp_def_account.set_metadata("name", "Outpost".to_string());
            dapp_def_account.set_metadata(
                "description",
                "An extension of your account for managing your NFTs across Radix".to_string(),
            );
            dapp_def_account.set_metadata("info_url", Url::of("https://outpostdocs.netlify.app/"));
            dapp_def_account.set_metadata(
                "icon_url",
                Url::of("https://outpostdocs.netlify.app/img/outpost_symbol.png"),
            );

            let dapp_def_address = GlobalAddress::from(dapp_def_account.address());

            let emitter_badge = self
                .emitter_trader_badge
                .mint_ruid_non_fungible(EmitterKey {
                    name: "emitter".to_string(),
                });

            let hub_address = None as Option<ComponentAddress>;

            let personal_trading_account_badge = self
                .outpost_account_badge
                .mint_ruid_non_fungible(TraderKey {
                    name: "Outpost Key".to_string(),
                    description: "Your key for listing and managing your NFTs across marketplaces and with other users.".to_string(),
                    key_image_url: Url::of("https://outpostdocs.netlify.app/img/outpost_symbol.png"),
                    hub: hub_address,
                });

            let nfgid = NonFungibleGlobalId::new(
                personal_trading_account_badge.resource_address(),
                personal_trading_account_badge.non_fungible_local_id(),
            );

            let depositer_permission_badge = self.royal_nft_depositer_badge.mint(1);

            let transient_token = self.transient_token_manager.mint(1);

            // Instantiation of a trading account via the outpost_account blueprint, passing in badges that will be locked in the accounts.
            let new_hub_component = OpenTrader::create_trader(
                nfgid.clone(),
                my_account,
                emitter_badge.into(),
                depositer_permission_badge.into(),
                self.event_manager,
                dapp_def_address,
                self.account_locker.clone(),
                transient_token,
            );

            let hub_component_address = new_hub_component.address();

            dapp_def_account.set_metadata(
                "claimed_entities",
                vec![GlobalAddress::from(hub_component_address.clone())],
            );
            dapp_def_account.set_owner_role(rule!(require(self.admin)));

            self.outpost_account_badge.update_non_fungible_data(
                nfgid.clone().local_id(),
                "hub",
                Some(hub_component_address.clone()),
            );

            Runtime::emit_event(OutpostCreated {
                outpost_component: hub_component_address.clone(),
                outpost_account: my_account.address(),
            });

            self.registered_accounts
                .insert(my_account.clone().address(), hub_component_address);

            // return the personal trading account badge (and the nfgid of the account for testing purposes)
            (nfgid, personal_trading_account_badge.into())
        }

        pub fn fetch_virt_badge(&mut self) -> ResourceAddress {
            self.emitter_trader_badge.address()
        }

        pub fn fetch_royal_nft_depositer_badge(&mut self) -> ResourceAddress {
            self.royal_nft_depositer_badge.address()
        }
    }
}
