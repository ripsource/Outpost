use scrypto::prelude::*;

/// Overview
// This blueprint is an example of a way to mint a new Royalty NFT collection, including a random mint/reveal process.
// However, not all of this is required for a new collection and creators can choose to implement only the parts that
// are necessary for their use case. This blueprint combines the minting process including a reveal step
// to allow a creator to reveal the collection after minting, and a royalty payment vault/method to allow collection of royalties.
// It's possible that the minting process could be separated from the royalty payment process, but this blueprint combines them for simplicity.

// For a minting process - its likely this could be made into a factory component for no-code creators - however anyone could bring their own
// component and just add in the deposit rules and resource top-level metadata required for the royalty system. In fact, some interesting
// opportunites are available for creators to design reactive traits/features based on the trading activity and interaction of components with their NFTs.

///
/// The royalty config struct holds all the settings a creator can modify in relation to royalties on their NFTs.
/// There are a bunch of options you can enable and fine tuning you can do - in general, I would expect launchpad platforms to offer some pre-made config options.
/// Then an advanced mode for creators to fine-tune their settings. It's important to note that once you have a basic understanding of the core features,
/// you can easily extend the functionality and add new features to the royalty system. As long as some basic principles are followed, it will still be
/// compatible with the rest of the OpenTrade system.

#[derive(ScryptoSbor)]
struct RoyaltyConfig {
    pub restricted_movement: bool,
    /// The royalty percentage to be paid to the creator of the Royal NFTs (e.g. 0.1 = 10% - maximum value is 1)
    royalty_percent: Decimal,
    /// The maximum royalty percentage that can be set - once set can not be increased. It can be decreased though.
    maximum_royalty_percent: Decimal,
    /// Offers an option for a creator to only allow trading of their assets in certain currencies (currencies selected in the permitted_currencies field)
    limit_currencies: bool,
    /// Currencies that the creator can receive royalties in/an NFT can be traded in (e.g. XRD)
    permitted_currencies: KeyValueStore<ResourceAddress, ()>,
    /// Set minimum fixed amounts of royalties for each permitted currency
    /// this is useful if a creator wants to allow private sales, but still ensure they receive royalties.
    minimum_royalties: bool,
    /// Minimum royalty amounts for each currency
    minimum_royalty_amounts: KeyValueStore<ResourceAddress, Decimal>,
    // Permissioned dApps - Dapps that you want to allow your NFTs to interact with/be deposited to.
    limit_dapps: bool,
    /// A permission list of components an NFT can be transferred to
    /// They both need a component address which is given permission for the NFTs to be transferred to.
    /// As well as a badge resource address that they will need internally in order to deposit the NFT back to a user.
    permissioned_dapps: KeyValueStore<ComponentAddress, ResourceAddress>,
    /// This is useful because private traders could trade the NFTs without paying royalties, so this closes that loophole.
    /// However, this can be turned off if the creator wants to allow any trader to trade the NFTs. If a creator wants to allow private sales,
    /// but still receive royalties - they can set a minimum royalty amount for each currency.
    limit_buyers: bool,
    /// A permission list for marketplaces/individual buyers that can trade the NFTs
    /// This requires that a certain badge is shown by the buyer or marketplace in order to purchase an NFT.
    permissioned_buyers: KeyValueStore<ResourceAddress, ()>,
    /// A method is exposed for transfering the NFT to another account via this royalty component.
    /// A user could use this deposit_via_router method to transfer an NFT freely to another user account if set to false.
    /// However, if a user wants to turn this off, we need to still allow permissioned dapps to interact with the NFTs/send them back to users.
    /// If set true, then the deposit_via_router can only be used for dapps with explicit permission in the permission dapps keyvalue.
    limit_private_trade: bool,
    /// lock royalty configuration: Option can give traders confidence that the royalty percentage/settings will not change.
    /// There's no method to undo this once set to true. However, right now creators can always take steps to make their
    /// royalties more relaxed even if locked - i.e. remove mininimum royalties, allow all buyers, etc.
    royalty_configuration_locked: bool,
}

#[derive(ScryptoSbor, NonFungibleData, Clone)]
struct NFT {
    #[mutable]
    name: String,
    #[mutable]
    description: String,
    #[mutable]
    key_image_url: Url,
    #[mutable]
    attributes: Vec<HashMap<String, String>>,
    #[mutable]
    ipfs_uri: Option<String>,
}

#[derive(ScryptoSbor)]
pub struct NFTData {
    pub name: String,
    pub description: String,
    pub key_image_url: Url,
    pub attributes: Vec<HashMap<String, String>>,
    pub ipfs_uri: Option<String>,
}

#[derive(ScryptoSbor, NonFungibleData)]
struct CreatorKey {
    collection: String,
    authority: String,
    minting_component: ComponentAddress,
    royalty_component: ComponentAddress,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct RevealMint {
    pub mint_component: ComponentAddress,
    pub resource_address: ResourceAddress,
    pub mint_start: i64,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct MintComplete {
    pub mint_component: ComponentAddress,
    pub resource_address: ResourceAddress,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct CancelMint {
    pub mint_component: ComponentAddress,
    pub resource_address: ResourceAddress,
}

#[derive(ScryptoSbor)]
pub struct NFTMetadata {
    pub name: String,
    pub description: String,
    pub icon_url: String,
    pub preview_image_url: String,
}
#[derive(ScryptoSbor)]
pub struct MintingConfig {
    pub mint_price: Decimal,
    pub mint_currency: ResourceAddress,
    pub initial_sale_cap: u64,
    pub rules: NFTRules,
}

#[derive(ScryptoSbor)]
pub struct RoyaltyLimits {
    pub limit_buyers: bool,
    pub limit_currencies: bool,
    pub limit_dapps: bool,
    pub limit_private_trade: bool,
    pub minimum_royalties: bool,
}

#[derive(ScryptoSbor)]
pub struct NFTRules {
    pub burnable: bool,
    pub burn_locked: bool,
    pub metadata_updatable: bool,
    pub metadata_locked: bool,
    pub royalty_config_locked: bool,
}

#[derive(ScryptoSbor)]
pub struct RoyaltyConfigInput {
    pub depositer_admin: ResourceAddress,
    pub royalties_enabled: bool,
    pub royalty_percent: Decimal,
    pub maximum_royalty_percent: Decimal,
}

#[derive(ScryptoSbor)]
pub struct CollectionInfo {
    pub name: String,
    pub description: String,
    pub collection_image: String,
    pub preview_image_url: String,
    pub metadata: KeyValueStore<NonFungibleLocalId, NFTData>,
}

#[derive(ScryptoSbor)]
pub struct MintingSettings {
    pub mint_price: Decimal,
    pub mint_currency: ResourceAddress,
    pub initial_sale_cap: u64,
    pub mint_id: u64,
    pub mint_enabled_after: Option<Instant>,
    pub reveal_step: bool,
    pub mint_payments_vault: Vault,
    pub minting_venue: KeyValueStore<ResourceAddress, ()>,
    pub allow_list: KeyValueStore<Global<Account>, (u64, u64)>,
    pub restrict_mint: bool,
}

#[derive(ScryptoSbor)]
pub struct AdminConfig {
    pub nft_creator_admin_manager: NonFungibleResourceManager,
    pub nft_creator_admin: ResourceAddress,
    pub depositer_admin: ResourceAddress,
    pub depositer_rule: AccessRule,
    pub virtual_account_admin: Option<Global<Account>>,
    pub virtual_admin_minting_badge: FungibleResourceManager,
    pub temp_admin: bool,
    pub internal_creator_admin: Vault,
}

#[derive(ScryptoSbor)]
pub struct TransactionTracking {
    pub latest_transaction: Option<(ResourceAddress, Vec<NonFungibleLocalId>, Global<Account>)>,
    pub transient_tokens: Vault,
    pub transient_token_address: ResourceAddress,
    pub transient_admin_vault: Vault,
}

#[blueprint]
#[events(RevealMint, MintComplete, CancelMint)]
mod royal_nft {

    enable_method_auth! {
    roles {
        admin => updatable_by: [];
    },
    methods {
        mint_preview_nft => PUBLIC;
        direct_mint => PUBLIC;
        enable_mint_reveal => restrict_to: [admin];
        upload_metadata => PUBLIC;
        creator_admin => PUBLIC;
        mint_reveal => PUBLIC;
        pay_royalty => PUBLIC;
        transfer_royalty_nft_to_dapp => PUBLIC;
        change_royalty_percentage_fee => restrict_to: [admin];
        lower_maximum_royalty_percentage => restrict_to: [admin];
        restrict_currencies_true => restrict_to: [admin];
        restrict_currencies_false => restrict_to: [admin];
        add_permitted_currency => restrict_to: [admin];
        remove_permitted_currency => restrict_to: [admin];
        enable_minimum_royalties => restrict_to: [admin];
        disable_minimum_royalties => restrict_to: [admin];
        set_minimum_royalty_amount => restrict_to: [admin];
        remove_minimum_royalty_amount => restrict_to: [admin];
        add_permissioned_buyer => restrict_to: [admin];
        remove_permissioned_buyer => restrict_to: [admin];
        limit_dapps_false => restrict_to: [admin];
        limit_dapps_true => restrict_to: [admin];
        add_permissioned_dapp => restrict_to: [admin];
        remove_permissioned_dapp => restrict_to: [admin];
        allow_all_buyers => restrict_to: [admin];
        deny_all_buyers => restrict_to: [admin];
        lock_royalty_configuration => restrict_to: [admin];
        resource_address => PUBLIC;
        deposit_via_router => PUBLIC;
        add_virtual_account_admin => restrict_to: [admin];
        remove_virtual_account_admin => restrict_to: [admin];
        cleared => PUBLIC;
        get_transient_token_address => PUBLIC;
        get_nft_address => PUBLIC;
        toggle_temp_admin => restrict_to: [admin];
        mint_temp_admin => restrict_to: [admin];
        mint_standard_preview_nft => PUBLIC;
        add_permissioned_mint_buyer => restrict_to: [admin];
        remove_permissioned_mint_buyer => restrict_to: [admin];
        cancel_public_mint => restrict_to: [admin];
        withdraw_from_mint_vault => restrict_to: [admin];
        withdraw_from_royalty_vault => restrict_to: [admin];
        remove_royalty_config => restrict_to: [admin];
        add_to_allow_list => restrict_to: [admin];
        remove_from_allow_list => restrict_to: [admin];
        restrict_mint_list => restrict_to: [admin];
    }
    }

    struct RoyalNFTs {
        nft_manager: NonFungibleResourceManager,
        mint_component: ComponentAddress,
        collection_info: CollectionInfo,
        minting_settings: MintingSettings,
        admin_config: AdminConfig,
        royalty_config: RoyaltyConfig,
        royalty_vaults: KeyValueStore<ResourceAddress, Vault>,
        royalty_component: ComponentAddress,
        transaction_tracking: TransactionTracking,
    }

    impl RoyalNFTs {
        pub fn start_minting_nft(
            setup_metadata: NFTMetadata,
            minting_config: MintingConfig,
            royalty_config_input: RoyaltyConfigInput,
            dapp_deff: ComponentAddress,
        ) -> (Global<RoyalNFTs>, NonFungibleBucket, ResourceAddress) {
            let (nft_address_reservation, royalty_component_address) =
                Runtime::allocate_component_address(RoyalNFTs::blueprint_id());

            assert!(
                royalty_config_input.royalty_percent <= Decimal::from(1),
                "Royalty percent must be less than 100%"
            );

            assert!(
                royalty_config_input.royalty_percent
                    <= royalty_config_input.maximum_royalty_percent,
                "Royalty percent must be less than maximum royalty"
            );

            // create the royalty config
            let royalty_config = RoyaltyConfig {
                restricted_movement: royalty_config_input.royalties_enabled,
                royalty_percent: royalty_config_input.royalty_percent,
                maximum_royalty_percent: royalty_config_input.maximum_royalty_percent,
                limit_buyers: false,
                limit_currencies: false,
                limit_dapps: false,
                limit_private_trade: false,
                minimum_royalties: false,
                permitted_currencies: KeyValueStore::new(),
                minimum_royalty_amounts: KeyValueStore::new(),
                permissioned_dapps: KeyValueStore::new(),
                permissioned_buyers: KeyValueStore::new(),
                royalty_configuration_locked: false,
            };

            let admin_name = format!("{} OP Admin", setup_metadata.name);

            let local_id_string = StringNonFungibleLocalId::new("creator_key".to_owned()).unwrap();
            // create the unique badge for the creator of the collection

            let nft_creator_admin = ResourceBuilder::new_string_non_fungible::<CreatorKey>(OwnerRole::None)
                .metadata(metadata! {
                    roles {
                        metadata_locker => rule!(deny_all);
                        metadata_locker_updater => rule!(deny_all);
                        metadata_setter => rule!(deny_all);
                        metadata_setter_updater => rule!(deny_all);
                    },
                    init {
                        "name" => admin_name.to_owned(), locked;
                        "type" => "OP Creator Key".to_owned(), locked;
                        "icon_url" => Url::of("https://outpostdocs.netlify.app/img/outpost_symbol.png"), locked;
                        "royalty_component" => royalty_component_address, locked;
                    }
                })
                .mint_initial_supply([(local_id_string, CreatorKey {
                    collection: setup_metadata.name.clone(),
                    authority: "Admin".to_owned(),
                    minting_component: royalty_component_address,
                    royalty_component: royalty_component_address,
                })]);

            let internal_creator_admin = ResourceBuilder::new_fungible(OwnerRole::None)
                .divisibility(0)
                .mint_initial_supply(1);

            // create the rules for the creator of the collection
            let creator_admin_rule = rule!(require_amount(
                dec!(1),
                nft_creator_admin.resource_address()
            ));

            // create the rules for the global caller badge
            let global_caller_badge_rule = rule!(require(global_caller(royalty_component_address)));

            // This is the key rule that allows trader accounts to trade royalty NFTs.
            // In this example, we're allowing the component and trader accounts to deposit NFT NFTs.
            let depositer_admin_rule: AccessRule;

            let change_movement_restrictions_rule: AccessRule;

            if royalty_config_input.royalties_enabled {
                depositer_admin_rule = rule!(
                    require_amount(1, royalty_config_input.depositer_admin)
                        || require_amount(1, internal_creator_admin.resource_address())
                        || require(nft_creator_admin.resource_address())
                );
                change_movement_restrictions_rule = rule!(
                    require_amount(1, royalty_config_input.depositer_admin)
                        || require_amount(1, internal_creator_admin.resource_address())
                        || require(nft_creator_admin.resource_address())
                );
            } else {
                depositer_admin_rule = rule!(allow_all);
                change_movement_restrictions_rule =
                    rule!(require(nft_creator_admin.resource_address()));
            }

            let burn_rule: AccessRule;
            if minting_config.rules.burnable {
                burn_rule = creator_admin_rule.clone();
            } else {
                burn_rule = rule!(deny_all);
            }

            let burn_locked_rule: AccessRule;
            if minting_config.rules.burn_locked {
                burn_locked_rule = rule!(deny_all);
            } else {
                burn_locked_rule = creator_admin_rule.clone();
            }

            let metadata_updatable_rule: AccessRule;
            if minting_config.rules.metadata_updatable {
                metadata_updatable_rule = global_caller_badge_rule.clone();
            } else {
                metadata_updatable_rule = rule!(deny_all);
            }

            let metadata_locked_rule: AccessRule;
            if minting_config.rules.metadata_locked {
                metadata_locked_rule = rule!(deny_all);
            } else {
                metadata_locked_rule = creator_admin_rule.clone();
            }

            let nft_manager = ResourceBuilder::new_integer_non_fungible::<NFT>(OwnerRole::Fixed(
                creator_admin_rule.clone(),
            ))
            .mint_roles(mint_roles! {
                minter => global_caller_badge_rule.clone();
                minter_updater => creator_admin_rule.clone();
            })
            .burn_roles(burn_roles! {
                burner => burn_rule;
                burner_updater => burn_locked_rule;
            })
            //**** REQUIRED FOR ROYALTY COMPATABILITY */
            // This rule creates the restriction that stops the NFTs from being traded without a royalty payment.
            // Only the royalty component can bypass this rule and trader accounts can bypass this rule.
            // If a creator wishes to leave the system completey - they can update the rules via methods on this component.
            .deposit_roles(deposit_roles! {
                depositor => depositer_admin_rule.clone();
                depositor_updater => change_movement_restrictions_rule;
            })
            .non_fungible_data_update_roles(non_fungible_data_update_roles! {
                non_fungible_data_updater => metadata_updatable_rule;
                non_fungible_data_updater_updater => metadata_locked_rule;
            })
            .metadata(metadata! {
                roles {
                    metadata_locker => creator_admin_rule.clone();
                    metadata_locker_updater => creator_admin_rule.clone();
                    metadata_setter => creator_admin_rule.clone();
                    metadata_setter_updater => creator_admin_rule.clone();
                },
                init {
                    "name" => setup_metadata.name.to_owned(), updatable;
                    "description" => setup_metadata.description.to_owned(), updatable;
                    "icon_url" => Url::of(setup_metadata.icon_url.clone()), updatable;
                    "metadata_standard" => "Outpost V1".to_owned(), updatable;
                    "info_url" => Url::of("https://trove.tools"), updatable;
                    "social_urls" => vec![Url::of("https://x.com/TroveEco")], updatable;
                    //**** REQUIRED FOR ROYALTY COMPATABILITY */
                    // We include the royalty component address in the NFTs top-level metadata.
                    // This is important as it means we don't need to programmatically find royalty components on the dApp.
                    // Instead we can dynamically find the component on the NFTs Resource metadata.
                    // It's important we don't place this component address on the individual NFTs because
                    // that would require us knowing the exact NFT Metadata structure to fetch/handle this data within Scrypto.
                    "royalty_component" => royalty_component_address, updatable;


                }
            })
            .create_with_no_initial_supply();

            let component_name = format!("{} OP", setup_metadata.name);

            let virtual_account_admin: Option<Global<Account>> = None;

            let virtual_admin_minting_badge = ResourceBuilder::new_fungible(OwnerRole::None)
                .divisibility(0)
                .metadata(metadata! {
                    roles {
                        metadata_setter => rule!(deny_all);
                        metadata_setter_updater => rule!(deny_all);
                        metadata_locker => rule!(deny_all);
                        metadata_locker_updater => rule!(deny_all);
                    },
                    init {
                        "name" => "Virtual Admin Minting Badge".to_owned(), locked;
                        "description" => "Virtual Admin Minting Badge".to_owned(), locked;
                        "icon_url" => Url::of("https://outpostdocs.netlify.app/img/outpost_symbol.png"), locked;
                    }
                })
                .mint_roles(mint_roles! {
                    minter => global_caller_badge_rule.clone();
                    minter_updater => creator_admin_rule.clone();
                })
                .burn_roles(burn_roles! {
                    burner => rule!(allow_all);
                    burner_updater => creator_admin_rule.clone();
                })
                .recall_roles(recall_roles! {
                    recaller => creator_admin_rule.clone();
                    recaller_updater => creator_admin_rule.clone();
                })
                .create_with_no_initial_supply();

            let transient_token = ResourceBuilder::new_fungible(OwnerRole::None)
                .divisibility(0)
                .deposit_roles(deposit_roles! {
                    depositor => rule!(allow_all);
                    depositor_updater => creator_admin_rule.clone();
                })
                .mint_initial_supply(1);

            let transient_admin_badge = ResourceBuilder::new_fungible(OwnerRole::None)
                .divisibility(0)
                .mint_initial_supply(1);

            let transient_manager =
                ResourceManager::from_address(transient_token.resource_address());

            let transient_token_address = transient_token.resource_address();

            let transient_vault: Vault = Vault::with_bucket(transient_token.into());

            nft_creator_admin.authorize_with_all(|| {
                transient_manager.set_depositable(rule!(require_amount(
                    1,
                    transient_admin_badge.resource_address()
                )));
            });

            let collection_info = CollectionInfo {
                name: setup_metadata.name.clone(),
                description: setup_metadata.description.clone(),
                collection_image: setup_metadata.icon_url.clone(),
                preview_image_url: setup_metadata.preview_image_url.clone(),
                metadata: KeyValueStore::new(),
            };

            let minting_settings = MintingSettings {
                mint_price: minting_config.mint_price,
                mint_currency: minting_config.mint_currency.clone(),
                initial_sale_cap: minting_config.initial_sale_cap,
                mint_id: 0,
                mint_enabled_after: None,
                reveal_step: false,
                mint_payments_vault: Vault::new(minting_config.mint_currency),
                minting_venue: KeyValueStore::new(),
                allow_list: KeyValueStore::new(),
                restrict_mint: false,
            };

            let admin_config = AdminConfig {
                nft_creator_admin_manager: nft_creator_admin.resource_manager(),
                nft_creator_admin: nft_creator_admin.resource_address(),
                depositer_admin: royalty_config_input.depositer_admin,
                depositer_rule: depositer_admin_rule,
                virtual_account_admin,
                virtual_admin_minting_badge,
                temp_admin: false,
                internal_creator_admin: Vault::with_bucket(internal_creator_admin.into()),
            };

            let transaction_tracking = TransactionTracking {
                latest_transaction: None,
                transient_tokens: transient_vault,
                transient_token_address,
                transient_admin_vault: Vault::with_bucket(transient_admin_badge.into()),
            };

            let component_adresss = Self {
                nft_manager,
                mint_component: royalty_component_address,
                collection_info,
                minting_settings,
                admin_config,
                royalty_config,
                royalty_vaults: KeyValueStore::new(),
                royalty_component: royalty_component_address.clone(),
                transaction_tracking,
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::None)
            .with_address(nft_address_reservation)
            .metadata(metadata! (
                roles {
                    metadata_setter => rule!(deny_all);
                    metadata_setter_updater => rule!(deny_all);
                    metadata_locker => rule!(deny_all);
                    metadata_locker_updater => rule!(deny_all);
                },
                init {
                    "name" => component_name.to_owned(), locked;
                    "description" => "An NFT minting and royalty component.".to_owned(), locked;
                    "dapp_definition" => dapp_deff, locked;
                    "icon_url" => Url::of(setup_metadata.icon_url), locked;
                }
            ))
            .roles(roles!(
                admin => rule!(require(nft_creator_admin.resource_address()));

            ))
            .globalize();

            (component_adresss, nft_creator_admin, nft_manager.address())
        }

        // helper method for tests
        pub fn resource_address(&self) -> ResourceAddress {
            self.nft_manager.address()
        }

        pub fn creator_admin(&self) -> ResourceAddress {
            self.admin_config.nft_creator_admin
        }

        pub fn mint_temp_admin(&mut self) -> FungibleBucket {
            self.admin_config.virtual_admin_minting_badge.mint(1)
        }

        pub fn withdraw_from_mint_vault(&mut self) -> Bucket {
            self.minting_settings.mint_payments_vault.take_all()
        }

        pub fn withdraw_from_royalty_vault(&mut self, currency: ResourceAddress) -> Bucket {
            let mut royalty_vault = self.royalty_vaults.get_mut(&currency).unwrap();

            royalty_vault.take_all()
        }

        //admin protect direct mint, returns to creator without any payment required.
        pub fn direct_mint(
            &mut self,
            auth_proof: Proof,
            recipient: Option<Global<Account>>,
            data: Vec<NFTData>,
        ) -> Option<Vec<Bucket>> {
            if auth_proof.resource_address() == self.admin_config.nft_creator_admin {
                auth_proof.check(self.admin_config.nft_creator_admin);
            } else {
                if self.admin_config.temp_admin {
                    auth_proof.check(self.admin_config.virtual_admin_minting_badge.address());
                } else {
                    panic!("Unauthorized");
                }
            }

            let mut return_buckets: Vec<Bucket> = vec![];

            for item in data {
                let name = item.name.clone();
                let description = item.description.clone();
                let key_image = item.key_image_url.clone();
                let mut ipfs_uri: Option<String> = None;
                if item.ipfs_uri.is_some() {
                    ipfs_uri = item.ipfs_uri.clone();
                }

                let nft = NFT {
                    name,
                    description,
                    key_image_url: key_image,
                    ipfs_uri,
                    attributes: item.attributes,
                };

                let nflid = NonFungibleLocalId::Integer(self.minting_settings.mint_id.into());

                let mint = self.nft_manager.mint_non_fungible(&nflid, nft);

                return_buckets.push(mint.into());

                self.minting_settings.mint_id += 1;
            }

            if recipient.is_some() {
                let mut direct_deposit = recipient.unwrap();
                // for some reason component caller auth was failing, I could just be dumb - probably dumb - yee think was a frontend issue, ffs - leaving it cause it might as well and works. Ta da
                let internal_admin = self.admin_config.internal_creator_admin.take(1);
                internal_admin.as_fungible().authorize_with_amount(1, || {
                    direct_deposit.try_deposit_batch_or_abort(return_buckets, None);
                });
                self.admin_config.internal_creator_admin.put(internal_admin);
                None
            } else {
                Some(return_buckets)
            }
        }

        pub fn cancel_public_mint(&mut self) {
            self.minting_settings.reveal_step = false;
            let cancel_mint = CancelMint {
                mint_component: self.royalty_component,
                resource_address: self.nft_manager.address(),
            };
            Runtime::emit_event(cancel_mint);
        }

        pub fn add_to_allow_list(&mut self, accounts: Vec<(Global<Account>, (u64, u64))>) {
            for (account, (start, end)) in accounts {
                self.minting_settings
                    .allow_list
                    .insert(account, (start, end));
            }
        }

        pub fn remove_from_allow_list(&mut self, accounts: Vec<Global<Account>>) {
            for account in accounts {
                self.minting_settings.allow_list.remove(&account);
            }
        }

        pub fn restrict_mint_list(&mut self, restrict: bool) {
            self.minting_settings.restrict_mint = restrict;
        }

        // if the NFTs being minted will have a buy - then - reveal step
        pub fn enable_mint_reveal(
            &mut self,
            initial_sale_cap: u64,
            enable_mint_after: Instant,
            mint_price: Decimal,
            minting_venues: Vec<ResourceAddress>,
        ) {
            self.minting_settings.initial_sale_cap = initial_sale_cap;
            self.minting_settings.mint_price = mint_price;
            self.minting_settings.mint_price = mint_price;
            self.minting_settings.reveal_step = true;

            for venue in minting_venues {
                self.minting_settings.minting_venue.insert(venue, ());
            }

            self.minting_settings.mint_enabled_after = Some(enable_mint_after);

            let reveal_mint = RevealMint {
                mint_component: self.royalty_component,
                resource_address: self.nft_manager.address(),
                mint_start: enable_mint_after.seconds_since_unix_epoch,
            };

            Runtime::emit_event(reveal_mint);
        }

        pub fn get_nft_address(&self) -> ResourceAddress {
            self.nft_manager.address()
        }

        pub fn get_transient_token_address(&self) -> ResourceAddress {
            self.transaction_tracking.transient_token_address
        }

        pub fn add_permissioned_mint_buyer(&mut self, buyer: ResourceAddress) {
            self.minting_settings.minting_venue.insert(buyer, ());
        }

        pub fn remove_permissioned_mint_buyer(&mut self, buyer: ResourceAddress) {
            self.minting_settings.minting_venue.remove(&buyer);
        }
        pub fn mint_standard_preview_nft(
            &mut self,
            mut payment: Bucket,
            no_editions: u64,
            permission: Proof,
        ) -> (Vec<NonFungibleBucket>, Bucket) {
            assert!(
                self.minting_settings.reveal_step == true,
                "[Mint Reveal] : This NFT doesn't have a reveal step enabled"
            );
            assert!(
                payment.amount() >= self.minting_settings.mint_price,
                "[Mint Preview NFT] : Insufficient funds to mint NFT"
            );

            assert!(
                payment.resource_address() == self.minting_settings.mint_currency,
                "[Mint Preview NFT] : Incorrect currency to mint NFT"
            );

            assert!(
                self.minting_settings.mint_id < self.minting_settings.initial_sale_cap,
                "[Mint Preview NFT] : sale cap reached"
            );

            if self.minting_settings.mint_id == self.minting_settings.initial_sale_cap {
                let reveal_mint = MintComplete {
                    mint_component: self.royalty_component,
                    resource_address: self.nft_manager.address(),
                };

                Runtime::emit_event(reveal_mint);
            }
            assert!(
                self.minting_settings
                    .minting_venue
                    .get(&permission.resource_address())
                    .is_some(),
                "Buy not via authorized seller"
            );

            assert!(
                no_editions <= self.minting_settings.initial_sale_cap,
                "Exceeds the initial sale cap"
            );

            let time_now = Clock::current_time_rounded_to_seconds();
            assert!(
                self.minting_settings.mint_enabled_after.is_some()
                    && time_now >= self.minting_settings.mint_enabled_after.unwrap(),
                "[Mint Preview NFT] : Minting not enabled yet"
            );

            let nft = NFT {
                name: self.minting_settings.mint_id.to_string(),
                description: self.collection_info.description.to_string(),
                key_image_url: Url::of(self.collection_info.preview_image_url.clone()),
                ipfs_uri: None,
                attributes: vec![],
            };

            let mut minted_editions: Vec<NonFungibleBucket> = vec![];

            for _ in 0..no_editions {
                let minted_edition = self.nft_manager.mint_non_fungible(
                    &NonFungibleLocalId::Integer(self.minting_settings.mint_id.into()),
                    nft.clone(),
                );
                minted_editions.push(minted_edition);
                self.minting_settings.mint_id += 1;
            }
            let dec_amount = Decimal::from(no_editions);
            self.minting_settings.mint_payments_vault.put(
                payment.take(
                    dec_amount
                        .checked_mul(self.minting_settings.mint_price)
                        .unwrap(),
                ),
            );

            (minted_editions, payment)
        }

        /// This function allows users to buy a preview of an NFT before it is minted. This acts as a mechanism for random minting.
        /// Users pay for the mint cost and only a certain limit set by the cap can be minted.
        /// After the desired number of NFTs have been minted, then the creator can update the metadata on all or some of the NFTs.
        pub fn mint_preview_nft(
            &mut self,
            mut payment: Bucket,
            amount: u64,
            account: Option<Global<Account>>,
            permission: Proof,
        ) -> (Bucket, Vec<NonFungibleBucket>, Option<Bucket>) {
            assert!(
                self.minting_settings.reveal_step == true,
                "[Mint Reveal] : This NFT doesn't have a reveal step enabled"
            );

            assert!(
                self.minting_settings
                    .minting_venue
                    .get(&permission.resource_address())
                    .is_some(),
                "Buy not via authorized seller"
            );

            let payment_required = amount * self.minting_settings.mint_price;

            assert!(
                payment.amount() >= payment_required,
                "[Mint Preview NFT] : Insufficient funds to mint NFT"
            );

            let no_editions: u64 = amount;

            assert!(
                no_editions > 0,
                "Payment must be greater than or equal to the mint price",
            );

            assert!(
                payment.resource_address() == self.minting_settings.mint_currency,
                "[Mint Preview NFT] : Incorrect currency to mint NFT"
            );

            self.minting_settings
                .mint_payments_vault
                .put(payment.take(no_editions * self.minting_settings.mint_price));

            if self.minting_settings.restrict_mint {
                assert!(
                    self.minting_settings
                        .allow_list
                        .get(&account.unwrap())
                        .is_some(),
                    "Buy not on allowlist"
                );

                let allow_min_max = self
                    .minting_settings
                    .allow_list
                    .get(&account.unwrap())
                    .unwrap()
                    .clone();

                let amount_user_can_buy = allow_min_max.1 - allow_min_max.0;
                assert!(no_editions <= amount_user_can_buy, "Exceeded mint limit");

                self.minting_settings.allow_list.insert(
                    account.unwrap(),
                    (allow_min_max.0 + no_editions, allow_min_max.1),
                );
            }

            assert!(
                self.minting_settings.mint_id <= self.minting_settings.initial_sale_cap,
                "[Mint Preview NFT] : Collection cap reached"
            );

            let time_now = Clock::current_time_rounded_to_seconds();

            assert!(
                self.minting_settings.mint_enabled_after.is_some()
                    && time_now >= self.minting_settings.mint_enabled_after.unwrap(),
                "[Mint Preview NFT] : Minting not enabled yet"
            );

            assert!(
                no_editions <= self.minting_settings.initial_sale_cap,
                "Exceeds the initial sale cap"
            );

            let mut minted_editions: Vec<NonFungibleBucket> = vec![];

            let nft = NFT {
                name: self.minting_settings.mint_id.to_string(),
                description: self.collection_info.description.to_string(),
                key_image_url: Url::of(self.collection_info.preview_image_url.clone()),
                ipfs_uri: None,
                attributes: vec![],
            };

            for _ in 0..no_editions {
                let minted_edition = self.nft_manager.mint_non_fungible(
                    &NonFungibleLocalId::Integer(self.minting_settings.mint_id.into()),
                    nft.clone(),
                );
                minted_editions.push(minted_edition);
                self.minting_settings.mint_id += 1;

                if self.minting_settings.mint_id == self.minting_settings.initial_sale_cap {
                    let mint_complete = MintComplete {
                        mint_component: self.royalty_component,
                        resource_address: self.nft_manager.address(),
                    };

                    Runtime::emit_event(mint_complete);
                    self.minting_settings.reveal_step = false;
                }
            }

            let mut transient_bucket: Option<Bucket> = None;

            if account.is_some() {
                transient_bucket = Some(self.transaction_tracking.transient_tokens.take(1));

                let nflid_vec = minted_editions
                    .iter()
                    .map(|nft| nft.non_fungible_local_id())
                    .collect::<Vec<NonFungibleLocalId>>();

                self.transaction_tracking.latest_transaction = Some((
                    self.nft_manager.address(),
                    nflid_vec,
                    account.unwrap().clone(),
                ));
                self.admin_config
                    .internal_creator_admin
                    .as_fungible()
                    .authorize_with_amount(1, || {
                        self.nft_manager.set_depositable(rule!(allow_all));
                    })
            }

            // we return any change from the transaction and the preview NFT
            (payment, minted_editions, transient_bucket)
        }

        pub fn cleared(&mut self, transient_token: FungibleBucket) {
            assert!(
                transient_token.amount() == dec!(1),
                "Transient token amount must be 1"
            );

            assert!(
                transient_token.resource_address()
                    == self.transaction_tracking.transient_token_address,
                "Transient token address must match"
            );

            assert!(
                self.transaction_tracking.latest_transaction.is_some(),
                "No transaction to clear"
            );

            let (nft_resource, nft_local, account_recipient) = self
                .transaction_tracking
                .latest_transaction
                .clone()
                .unwrap();

            for local_id in nft_local {
                assert!(
                    account_recipient.has_non_fungible(nft_resource, local_id),
                    "NFT not received by expected account {:?}",
                    account_recipient,
                );
            }

            let admin = self.transaction_tracking.transient_admin_vault.take(1);

            admin.as_fungible().authorize_with_amount(1, || {
                self.transaction_tracking
                    .transient_tokens
                    .put(transient_token.into());
            });

            self.transaction_tracking.transient_admin_vault.put(admin);

            // panic!("debug deposit");
            self.admin_config
                .internal_creator_admin
                .as_fungible()
                .authorize_with_amount(1, || {
                    self.nft_manager
                        .set_depositable(self.admin_config.depositer_rule.clone());
                });
        }

        pub fn add_virtual_account_admin(&mut self, account: Global<Account>) {
            self.admin_config.virtual_account_admin = Some(account);
        }

        pub fn remove_virtual_account_admin(&mut self) {
            let remove_account: Option<Global<Account>> = None;
            self.admin_config.virtual_account_admin = remove_account;
        }

        // this functions allows the creator to upload the metadata for the NFTs to conduct the reveal
        pub fn upload_metadata(
            &mut self,
            auth_proof: Proof,
            data: Vec<(NonFungibleLocalId, NFTData)>,
        ) {
            if auth_proof.resource_address() == self.admin_config.nft_creator_admin {
                auth_proof.check(self.admin_config.nft_creator_admin);
            } else {
                if self.admin_config.temp_admin {
                    auth_proof.check(self.admin_config.virtual_admin_minting_badge.address());
                } else {
                    panic!("Unauthorized");
                }
            }

            for (nft_id, metadata) in data {
                self.collection_info.metadata.insert(nft_id, metadata);
            }
        }

        // this function updates the metadata on an NFT that has already been minted to reveal the collection
        pub fn mint_reveal(&mut self, auth_proof: Proof, data: Vec<(NonFungibleLocalId, NFTData)>) {
            if auth_proof.resource_address() == self.admin_config.nft_creator_admin {
                auth_proof.check(self.admin_config.nft_creator_admin);
            } else {
                if self.admin_config.temp_admin {
                    auth_proof.check(self.admin_config.virtual_admin_minting_badge.address());
                } else {
                    panic!("Unauthorized");
                }
            }

            for (nft_id, nfdata) in data {
                self.nft_manager.update_non_fungible_data(
                    &nft_id,
                    "attributes",
                    nfdata.attributes.clone(),
                );

                self.nft_manager
                    .update_non_fungible_data(&nft_id, "name", nfdata.name.clone());

                self.nft_manager.update_non_fungible_data(
                    &nft_id,
                    "description",
                    nfdata.description.clone(),
                );

                self.nft_manager.update_non_fungible_data(
                    &nft_id,
                    "key_image_url",
                    nfdata.key_image_url.clone(),
                );

                if nfdata.ipfs_uri.is_some() {
                    self.nft_manager.update_non_fungible_data(
                        &nft_id,
                        "ipfs_uri",
                        nfdata.ipfs_uri.unwrap().clone(),
                    );
                }
            }
        }

        // This function can be called by trader accounts when an NFT from this collection is purchased.
        // It takes the payment and an option for an account to send the NFT to.
        // It uses the royalty percentage set by the creator to determine how much of the payment to take.
        // We use a keyvaluestore of vaults so that we can store multiple currencies.
        // We take the NFT as an argument so that we can determine at this point whether we want to enforce advanced royalties settings
        // where only an account component can own the NFT - in which case we just sent the NFT directly to the input account.
        // Otherwise, we send the NFT back to the trading account component, where a it could be sent on to another component.
        pub fn pay_royalty(
            &mut self,
            nft: ResourceAddress,
            mut payment: Bucket,
            buyer: ResourceAddress,
        ) -> Bucket {
            let payment_amount = payment.amount();

            // check the correct NFT for this royalty component has been passed
            assert!(
                nft == self.nft_manager.address(),
                "[pay_royalty] Incorrect resource passed"
            );

            if self.royalty_config.limit_buyers {
                assert!(
                    self.royalty_config
                        .permissioned_buyers
                        .get(&buyer)
                        .is_some(),
                    "This buyer is not permissioned to trade this NFT"
                );
            }

            let currency = payment.resource_address();
            let limit_currencies = self.royalty_config.limit_currencies;

            if limit_currencies {
                assert!(
                    self.royalty_config
                        .permitted_currencies
                        .get(&currency)
                        .is_some(),
                    "This currency is not permitted for royalties"
                );
            }

            // send the royalty to the royalty vault

            let vault_exists = self.royalty_vaults.get(&currency).is_some();

            if !vault_exists {
                // check the correct amount has been passed
                let royalty = payment.take_advanced(
                    payment_amount
                        .checked_mul(self.royalty_config.royalty_percent)
                        .unwrap(),
                    WithdrawStrategy::Rounded(RoundingMode::ToZero),
                );

                if limit_currencies {
                    if self.royalty_config.minimum_royalties {
                        let minimum_royalty = self
                            .royalty_config
                            .minimum_royalty_amounts
                            .get(&currency)
                            .unwrap();
                        assert!(
                            royalty.amount() >= minimum_royalty.clone(),
                            "Royalty amount is below the minimum required"
                        );
                    }
                }

                self.royalty_vaults
                    .insert(currency.clone(), Vault::with_bucket(royalty));
            } else {
                // check the correct amount has been passed
                let royalty = payment.take_advanced(
                    payment_amount
                        .checked_mul(self.royalty_config.royalty_percent)
                        .unwrap(),
                    WithdrawStrategy::Rounded(RoundingMode::ToZero),
                );

                if limit_currencies {
                    if self.royalty_config.minimum_royalties {
                        let minimum_royalty = self
                            .royalty_config
                            .minimum_royalty_amounts
                            .get(&currency)
                            .unwrap();
                        assert!(
                            royalty.amount() >= minimum_royalty.clone(),
                            "Royalty amount is below the minimum required"
                        );
                    }
                }
                self.royalty_vaults.get_mut(&currency).unwrap().put(royalty);
            }

            // payment minus royalty returned to the trading account that called this method
            payment
        }

        /// Possibility to transfer the royalty NFT to a dApp if permissions are set for advanced royalty enforcement - requires the dApp to be permissioned - transfer occurs here.
        /// If the royalty config allows it, then any dApp can interact with the NFT.
        /// We allow an optional return of a vector of buckets which should cover most use cases for dApps.
        ///
        /// As long as the code remains relatively similar - developers can use this method to have some reactive logic for when their NFTs interact with certain dApps.
        pub fn transfer_royalty_nft_to_dapp(
            &mut self,
            nft: Bucket,
            dapp: ComponentAddress,
            custom_method: String,
        ) -> Option<Vec<Bucket>> {
            if self.royalty_config.limit_dapps {
                assert!(
                    self.royalty_config.permissioned_dapps.get(&dapp).is_some(),
                    "This dApp has not been permissioned by the collection creator"
                );
            }

            let call_address: Global<AnyComponent> = Global(ObjectStub::new(
                ObjectStubHandle::Global(GlobalAddress::from(dapp)),
            ));

            let manfiest_method: &str = &custom_method;

            self.nft_manager.set_depositable(rule!(allow_all));

            // send nft to dapp
            let optional_returned_buckets =
                call_address.call_raw::<Option<Vec<Bucket>>>(manfiest_method, scrypto_args!(nft));

            self.nft_manager.set_depositable(rule!(
                require_amount(1, self.admin_config.depositer_admin)
                    || require(global_caller(self.royalty_component))
            ));

            optional_returned_buckets
        }

        pub fn deposit_via_router(
            &mut self,
            nft: Bucket,
            permission: Proof,
            dapp: ComponentAddress,
            mut account: Global<Account>,
        ) -> Bucket {
            if self.royalty_config.limit_private_trade {
                assert!(
                    self.royalty_config.permissioned_dapps.get(&dapp).is_some(),
                    "This dApp has not been permissioned by the collection creator"
                );

                let badge = self
                    .royalty_config
                    .permissioned_dapps
                    .get(&dapp)
                    .unwrap()
                    .clone();

                permission.check(badge);
            }
            // we can now deposit to a user

            let resource_image: Url = ResourceManager::from_address(nft.resource_address())
                .get_metadata("icon_url")
                .unwrap()
                .unwrap();

            let resource_name: String = ResourceManager::from_address(nft.resource_address())
                .get_metadata("name")
                .unwrap()
                .unwrap();

            let receipt_name = format!(
                "{} : {}",
                resource_name,
                nft.as_non_fungible().non_fungible_local_id().to_string()
            );

            let receipt = ResourceBuilder::new_fungible(OwnerRole::None)
        .burn_roles(burn_roles! {
            burner => rule!(allow_all);
            burner_updater => rule!(deny_all);
        })
            .metadata(metadata! {
                roles {
                    metadata_locker => rule!(deny_all);
                    metadata_locker_updater => rule!(deny_all);
                    metadata_setter => rule!(deny_all);
                    metadata_setter_updater => rule!(deny_all);
                },
                init {
                    "name" => receipt_name.to_owned(), locked;
                    "icon_url" => resource_image, locked;
                    "resource_address" => nft.resource_address(), locked;
                    "local_id" => nft.as_non_fungible().non_fungible_local_id().to_string(), locked;
                    "receipt" => "This is a display receipt to show the NFT being transferred to your account in this transaction. You will see this NFT in your wallet after the transaction. You can burn this token if you wish to remove the receipt from your wallet.".to_owned(), locked;
                }
            })
            .mint_initial_supply(1);

            account.try_deposit_or_abort(nft.into(), None);

            receipt.into()
        }

        //
        // These set of methods offer the ability for the creator modify their royalty settings.
        //

        /// Only possible if the royalty configuration is not locked
        /// New percentage fee must be below the maximum set.

        pub fn remove_royalty_config(&mut self) {
            self.royalty_config.restricted_movement = false;
            self.royalty_config.royalty_percent = dec!(0);

            self.admin_config
                .internal_creator_admin
                .as_fungible()
                .authorize_with_amount(1, || {
                    self.nft_manager.set_depositable(rule!(allow_all));
                });
        }

        pub fn change_royalty_percentage_fee(&mut self, new_royalty_percent: Decimal) {
            assert!(
                !self.royalty_config.royalty_configuration_locked,
                "Royalty configuration is locked"
            );

            assert!(
                new_royalty_percent <= self.royalty_config.maximum_royalty_percent,
                "New royalty percentage is greater than maximum allowed"
            );

            self.royalty_config.royalty_percent = new_royalty_percent;
        }

        /// you can always lower the maximum royalty percentage - even if the configuration is locked.
        pub fn lower_maximum_royalty_percentage(&mut self, new_max_royalty_percent: Decimal) {
            assert!(
                new_max_royalty_percent >= self.royalty_config.royalty_percent,
                "New maximum royalty percentage is less than current royalty percentage"
            );

            self.royalty_config.maximum_royalty_percent = new_max_royalty_percent;
        }

        /// Only possible if the royalty configuration is not locked.
        /// You can always turn this setting off even if the configuration is locked.
        pub fn restrict_currencies_true(&mut self) {
            assert!(
                !self.royalty_config.royalty_configuration_locked,
                "Royalty configuration is locked"
            );
            self.royalty_config.limit_currencies = true;
        }

        pub fn restrict_currencies_false(&mut self) {
            self.royalty_config.limit_currencies = false;
        }

        // You can only add restricted currencies if the restricted currency setting is turned on.
        // You can add even if the configuration is locked.
        pub fn add_permitted_currency(&mut self, currency: ResourceAddress) {
            assert!(
                self.royalty_config.limit_currencies,
                "Restricted currency setting is not turned on"
            );
            self.royalty_config
                .permitted_currencies
                .insert(currency, ());
        }

        // You can only remove restricted currencies if the restricted currency setting is turned on.
        // You can't remove currencies if the configuration is locked.
        pub fn remove_permitted_currency(&mut self, currency: ResourceAddress) {
            assert!(
                self.royalty_config.limit_currencies,
                "Restricted currency setting is not turned on"
            );
            assert!(
                !self.royalty_config.royalty_configuration_locked,
                "Royalty configuration is locked"
            );
            self.royalty_config.permitted_currencies.remove(&currency);
        }

        // You can only set minimum royalty amounts if the restricted currency setting is turned on.

        // enable minimum royalties

        pub fn enable_minimum_royalties(&mut self) {
            assert!(
                self.royalty_config.limit_currencies,
                "Restricted currency setting is not turned on"
            );
            self.royalty_config.minimum_royalties = true;
        }

        pub fn disable_minimum_royalties(&mut self) {
            self.royalty_config.minimum_royalties = false;
        }

        // You can't set minimum amounts if the configuration is locked.
        pub fn set_minimum_royalty_amount(
            &mut self,
            currency: ResourceAddress,
            minimum_royalty_amount: Decimal,
        ) {
            assert!(
                self.royalty_config.limit_currencies,
                "Restricted currency setting is not turned on"
            );
            assert!(
                !self.royalty_config.royalty_configuration_locked,
                "Royalty configuration is locked"
            );
            self.royalty_config
                .minimum_royalty_amounts
                .insert(currency, minimum_royalty_amount);
        }

        // You can only remove minimum royalty amounts if the restricted currency setting is turned on.
        // You can remove even if the configuration is locked.
        pub fn remove_minimum_royalty_amount(&mut self, currency: ResourceAddress) {
            assert!(
                self.royalty_config.limit_currencies,
                "Restricted currency setting is not turned on"
            );
            self.royalty_config
                .minimum_royalty_amounts
                .remove(&currency);
        }

        // Permissioned dapps settings only work with limit dapps enabled.

        pub fn limit_dapps_true(&mut self) {
            assert!(
                !self.royalty_config.royalty_configuration_locked,
                "Royalty configuration is locked"
            );
            self.royalty_config.limit_dapps = true;
        }

        pub fn limit_dapps_false(&mut self) {
            self.royalty_config.limit_dapps = false;
        }

        // You can add even if the configuration is locked.
        pub fn add_permissioned_dapp(&mut self, dapp: ComponentAddress, badge: ResourceAddress) {
            self.royalty_config.permissioned_dapps.insert(dapp, badge);
        }

        // You can't remove dapps if the configuration is locked.
        pub fn remove_permissioned_dapp(&mut self, dapp: ComponentAddress) {
            assert!(
                !self.royalty_config.royalty_configuration_locked,
                "Royalty configuration is locked"
            );
            self.royalty_config.permissioned_dapps.remove(&dapp);
        }

        // Permissioned buyers settings only work with advanced royalty enforcement settings.
        // You can always add more permissioned buyers even if the configuration is locked.
        pub fn add_permissioned_buyer(&mut self, buyer: ResourceAddress) {
            self.royalty_config.permissioned_buyers.insert(buyer, ());
        }

        // You can't remove buyers if the configuration is locked.
        pub fn remove_permissioned_buyer(&mut self, buyer: ResourceAddress) {
            assert!(
                !self.royalty_config.royalty_configuration_locked,
                "Royalty configuration is locked"
            );
            self.royalty_config.permissioned_buyers.remove(&buyer);
        }

        // You can't change to deny_all buyers if the configuration is locked.
        pub fn deny_all_buyers(&mut self) {
            assert!(
                !self.royalty_config.royalty_configuration_locked,
                "Royalty configuration is locked"
            );
            self.royalty_config.limit_buyers = true;
        }

        // You can allow all buyers even if the configuration is locked
        pub fn allow_all_buyers(&mut self) {
            self.royalty_config.limit_buyers = false;
        }

        pub fn lock_royalty_configuration(&mut self) {
            self.royalty_config.royalty_configuration_locked = true;
        }

        pub fn toggle_temp_admin(&mut self) {
            self.admin_config.temp_admin = !self.admin_config.temp_admin;
        }
    }
}

// #[derive(ScryptoSbor)]
// pub enum RoyaltyAction {
//     SetRoyaltyPercent(Decimal),
//     SetMaxRoyaltyPercent(Decimal),
//     EnableCurrencyRestrictions(bool),
//     AddPermittedCurrency(ResourceAddress),
//     RemovePermittedCurrency(ResourceAddress),
//     EnableMinimumRoyalties(bool),
//     SetMinimumRoyalty(ResourceAddress, Decimal),
//     RemoveMinimumRoyalty(ResourceAddress),
// }

// #[derive(ScryptoSbor)]
// pub enum PermissionAction {
//     EnableDappLimits(bool),
//     AddDapp(ComponentAddress, ResourceAddress),
//     RemoveDapp(ComponentAddress),
//     EnableBuyerLimits(bool),
//     AddBuyer(ResourceAddress),
//     RemoveBuyer(ResourceAddress),
// }

// impl RoyalNFTs {
//     pub fn update_royalty_config(&mut self, action: RoyaltyAction) {
//         if self.royalty_config.royalty_configuration_locked {
//             // Only allow specific actions when locked
//             match action {
//                 RoyaltyAction::SetMaxRoyaltyPercent(new_max) if new_max < self.royalty_config.maximum_royalty_percent => {
//                     self.royalty_config.maximum_royalty_percent = new_max;
//                 },
//                 RoyaltyAction::EnableCurrencyRestrictions(false) => {
//                     self.royalty_config.limit_currencies = false;
//                 },
//                 RoyaltyAction::AddPermittedCurrency(currency) if self.royalty_config.limit_currencies => {
//                     self.royalty_config.permitted_currencies.insert(currency, ());
//                 },
//                 _ => panic!("Action not allowed when config is locked")
//             }
//             return;
//         }

//         // Handle all actions when not locked
//         match action {
//             RoyaltyAction::SetRoyaltyPercent(percent) => {
//                 assert!(percent <= self.royalty_config.maximum_royalty_percent);
//                 self.royalty_config.royalty_percent = percent;
//             },
//             // ... other matches for each action
//         }
//     }

//     pub fn update_permissions(&mut self, action: PermissionAction) {
//         if self.royalty_config.royalty_configuration_locked {
//             // Only allow specific actions when locked
//             match action {
//                 PermissionAction::AddDapp(dapp, badge) => {
//                     self.royalty_config.permissioned_dapps.insert(dapp, badge);
//                 },
//                 PermissionAction::AddBuyer(buyer) => {
//                     self.royalty_config.permissioned_buyers.insert(buyer, ());
//                 },
//                 _ => panic!("Action not allowed when config is locked")
//             }
//             return;
//         }

//         // Handle all actions when not locked
//         match action {
//             PermissionAction::EnableDappLimits(enable) => {
//                 self.royalty_config.limit_dapps = enable;
//             },
//             // ... other matches for each action
//         }
//     }
// }
