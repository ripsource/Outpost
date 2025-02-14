use scrypto::prelude::*;

use crate::mint_factory::*;

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

// #[derive(ScryptoSbor, ScryptoEvent)]
// struct NewOpenTradeMint {
//     resource_address: ResourceAddress,
//     minting_component: ComponentAddress,
//     royalty_component: ComponentAddress,
// }

// #[derive(ScryptoSbor, ScryptoEvent)]
// struct NewOpenTradeReveal {
//     resource_address: ResourceAddress,
//     minting_component: ComponentAddress,
//     royalty_component: ComponentAddress,
// }

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
    }
    }

    struct RoyalNFTs {
        nft_manager: NonFungibleResourceManager,
        nft_creator_admin_manager: NonFungibleResourceManager,
        nft_creator_admin: ResourceAddress,

        // NFT data
        preview_image_url: String,
        description: String,
        reveal_step: bool,
        initial_sale_cap: u64,
        mint_enabled_after: Option<Instant>,
        // reveal data to be uploaded by creator
        metadata: KeyValueStore<NonFungibleLocalId, NFTData>,

        // the admin address input required to sync with the OpenTrader system
        depositer_admin: ResourceAddress,

        // deposit admin rule - for royalty enabled collections
        depositer_rule: AccessRule,

        /// The price to mint a Royal NFT NFT
        mint_price: Decimal,

        /// The selected currrency for minting Royal NFTs, e.g. XRD
        mint_currency: ResourceAddress,

        /// The current mint ID for integer NFTs minted
        mint_id: u64,

        /// The vault for storing mint payments
        mint_payments_vault: Vault,

        /// All the royalty payments that have been made for different currencies
        royalty_vaults: KeyValueStore<ResourceAddress, Vault>,

        /// The address of the royalty component (which in this case, is this same component)
        royalty_component: ComponentAddress,

        /// The creator royalty settings
        royalty_config: RoyaltyConfig,

        /// virtual_account temp admin
        virtual_account_admin: Option<Global<Account>>,

        /// virtual mint badge
        virtual_admin_minting_badge: FungibleResourceManager,

        /// enable temp admin for minting
        temp_admin: bool,

        /// internal creator admin
        internal_creator_admin: Vault,

        /// Specify minting venue/marketplace - i.e. specific marketplaces that can mint the NFTs.
        /// This is useful if a creator wants to allow minting of their NFTs on a specific marketplace.
        minting_venue: KeyValueStore<ResourceAddress, ()>,
        /// Latest transaction tracker for confirming the transaction has been cleared
        latest_transaction: Option<(ResourceAddress, Vec<NonFungibleLocalId>, Global<Account>)>,
        /// Transient tokens for clearing transactions
        transient_tokens: Vault,
        /// Transient token address
        transient_token_address: ResourceAddress,
        /// transient admin vault (seperate admin required beyond global caller as depositer rules are buggy in scrypto for component auth + deposit restrictions)
        transient_admin_vault: Vault,
    }

    impl RoyalNFTs {
        pub fn start_minting_nft(
            // top-level resource metadata
            name: String,
            description: String,
            icon_url: String,

            // preview image used prior to revealing a collection
            preview_image_url: String,

            // generic minting inputs (could be any set up for minting the collection)
            mint_price: Decimal,
            mint_currency: ResourceAddress,
            initial_sale_cap: u64,

            // NFT rule settings
            rules: Vec<bool>,
            // 0. burnable: bool,
            // 1. burn_locked: bool,
            // 2. metadata_updatable: bool,
            // 3. metadata_locked: bool,
            // (reccommend setting to false and later locking the configuration if desired)
            // 4. royalty_config_locked: bool,

            // Required to enable trader accounts to interact with royalty NFTs
            depositer_admin: ResourceAddress,

            // royalty settings input
            royalties_enabled: bool,
            royalty_percent: Decimal,
            maximum_royalty_percent: Decimal,

            // These represent some advanced setting that creators can enable to heighten the level of royalty enforcement
            // and use to create new reactive/dynamic features for their NFTs.
            limits: Vec<bool>,
            // 0. limit_buyers: bool,
            // 1. limit_currencies: bool,
            // 2. limit_dapps: bool,
            // 3. limit_private_trade: bool,
            // 4. minimum_royalties: bool,

            // This is relevant for transfers of an NFT to a component/Dapp - not for trading the NFTs.
            permissioned_dapps_input: HashMap<ComponentAddress, ResourceAddress>,

            // Only applicable if limit buyers is set to true
            permissioned_buyers_input: Vec<ResourceAddress>,

            // only applicable if you want to restrict the currencies that can be used to pay royalties
            restricted_currencies_input: Vec<ResourceAddress>,
            // if restricting the currencies you can then also add minimum amounts for how much royalty you should receive.
            // This is set so that if you require 20 XRD as a minimum, and your %fee is 10% - then atleast a 200 XRD sale would be required.
            minimum_royalty_amounts_input: HashMap<ResourceAddress, Decimal>,
        ) -> (Global<RoyalNFTs>, NonFungibleBucket, ResourceAddress) {
            let (nft_address_reservation, royalty_component_address) =
                Runtime::allocate_component_address(RoyalNFTs::blueprint_id());

            assert!(
                royalty_percent <= Decimal::from(1),
                "Royalty percent must be less than 100%"
            );

            assert!(
                royalty_percent <= maximum_royalty_percent,
                "Royalty percent must be less than maximum royalty"
            );

            let permissioned_dapps: KeyValueStore<ComponentAddress, ResourceAddress> =
                KeyValueStore::new();
            let permissioned_buyers: KeyValueStore<ResourceAddress, ()> = KeyValueStore::new();
            let permitted_currencies: KeyValueStore<ResourceAddress, ()> = KeyValueStore::new();
            let minimum_royalty_amounts: KeyValueStore<ResourceAddress, Decimal> =
                KeyValueStore::new();

            // 0. limit_buyers: bool,
            // 1. limit_currencies: bool,
            // 2. limit_dapps: bool,
            // 3. limit_private_trade: bool,
            // 4. minimum_royalties: bool,

            if limits[2] {
                for component_address in permissioned_dapps_input {
                    permissioned_dapps.insert(component_address.0, component_address.1);
                }
            }

            if limits[0] {
                for resource_address in permissioned_buyers_input {
                    permissioned_buyers.insert(resource_address, ());
                }
            }

            if limits[1] {
                for currency in restricted_currencies_input {
                    permitted_currencies.insert(currency, ());
                }
                for (currency, amount) in minimum_royalty_amounts_input {
                    minimum_royalty_amounts.insert(currency, amount);
                }
            }

            // create the royalty config
            let royalty_config = RoyaltyConfig {
                royalty_percent,
                maximum_royalty_percent,
                limit_buyers: limits[0],
                limit_currencies: limits[1],
                limit_dapps: limits[2],
                limit_private_trade: limits[3],
                minimum_royalties: limits[4],
                permitted_currencies,
                minimum_royalty_amounts,
                permissioned_dapps,
                permissioned_buyers,
                royalty_configuration_locked: rules[4],
            };

            let admin_name = format!("{} OP Admin", name);

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
                    collection: name.clone(),
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

            if royalties_enabled {
                depositer_admin_rule = rule!(
                    require_amount(1, depositer_admin)
                        || require_amount(1, internal_creator_admin.resource_address())
                        || require(nft_creator_admin.resource_address())
                );
            } else {
                depositer_admin_rule = rule!(allow_all);
            }

            let burn_rule: AccessRule;
            if rules[0] {
                burn_rule = creator_admin_rule.clone();
            } else {
                burn_rule = rule!(deny_all);
            }

            let burn_locked_rule: AccessRule;
            if rules[1] {
                burn_locked_rule = rule!(deny_all);
            } else {
                burn_locked_rule = creator_admin_rule.clone();
            }

            let metadata_updatable_rule: AccessRule;
            if rules[2] {
                metadata_updatable_rule = global_caller_badge_rule.clone();
            } else {
                metadata_updatable_rule = rule!(deny_all);
            }

            let metadata_locked_rule: AccessRule;
            if rules[3] {
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
                depositor_updater => depositer_admin_rule;
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
                    "name" => name.to_owned(), updatable;
                    "description" => description.to_owned(), updatable;
                    "icon_url" => Url::of(icon_url.clone()), updatable;
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

            let component_name = format!("{} OP", name);

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
                transient_manager
                    .set_depositable(rule!(require(transient_admin_badge.resource_address())));
            });

            let component_adresss = Self {
                nft_manager,
                royalty_component: royalty_component_address.clone(),
                nft_creator_admin_manager: nft_creator_admin.resource_manager(),
                nft_creator_admin: nft_creator_admin.resource_address(),
                depositer_rule: rule!(
                    require_amount(1, depositer_admin)
                        || require(global_caller(royalty_component_address))
                ),
                preview_image_url,
                description,
                reveal_step: false,
                mint_enabled_after: None,
                initial_sale_cap,
                metadata: KeyValueStore::new(),
                depositer_admin,
                mint_price,
                mint_currency: mint_currency.clone(),
                mint_id: 0,
                mint_payments_vault: Vault::new(mint_currency),
                royalty_vaults: KeyValueStore::new(),
                royalty_config,
                virtual_account_admin,
                internal_creator_admin: Vault::with_bucket(internal_creator_admin.into()),
                temp_admin: false,
                virtual_admin_minting_badge,
                minting_venue: KeyValueStore::new(),
                latest_transaction: None,
                transient_token_address,
                transient_tokens: transient_vault,
                transient_admin_vault: Vault::with_bucket(transient_admin_badge.into()),
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
                    "dapp_definition" => royalty_component_address, locked;
                    "icon_url" => Url::of(icon_url), locked;
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
            self.nft_creator_admin
        }

        pub fn mint_temp_admin(&mut self) -> FungibleBucket {
            self.virtual_admin_minting_badge.mint(1)
        }

        pub fn withdraw_from_mint_vault(&mut self) -> Bucket {
            self.mint_payments_vault.take_all()
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
            if auth_proof.resource_address() == self.nft_creator_admin {
                auth_proof.check(self.nft_creator_admin);
            } else {
                if self.temp_admin {
                    auth_proof.check(self.virtual_admin_minting_badge.address());
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

                let nflid = NonFungibleLocalId::Integer(self.mint_id.into());

                let mint = self.nft_manager.mint_non_fungible(&nflid, nft);

                return_buckets.push(mint.into());

                self.mint_id += 1;
            }

            if recipient.is_some() {
                let mut direct_deposit = recipient.unwrap();
                // for some reason component caller auth was failing, I could just be dumb - probably dumb - yee think was a frontend issue, ffs - leaving it cause it might as well and works. Ta da
                let internal_admin = self.internal_creator_admin.take(1);
                internal_admin.as_fungible().authorize_with_amount(1, || {
                    direct_deposit.try_deposit_batch_or_abort(return_buckets, None);
                });
                self.internal_creator_admin.put(internal_admin);
                None
            } else {
                Some(return_buckets)
            }
        }

        pub fn cancel_public_mint(&mut self) {
            self.reveal_step = false;
            let cancel_mint = CancelMint {
                mint_component: self.royalty_component,
                resource_address: self.nft_manager.address(),
            };
            Runtime::emit_event(cancel_mint);
        }

        // if the NFTs being minted will have a buy - then - reveal step
        pub fn enable_mint_reveal(
            &mut self,
            initial_sale_cap: u64,
            enable_mint_after: Instant,
            mint_price: Decimal,
            minting_venues: Vec<ResourceAddress>,
        ) {
            self.initial_sale_cap = initial_sale_cap;
            self.mint_price = mint_price;
            self.reveal_step = true;
            for venue in minting_venues {
                self.minting_venue.insert(venue, ());
            }
            self.mint_enabled_after = Some(enable_mint_after);

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
            self.transient_token_address
        }

        pub fn add_permissioned_mint_buyer(&mut self, buyer: ResourceAddress) {
            self.minting_venue.insert(buyer, ());
        }

        pub fn remove_permissioned_mint_buyer(&mut self, buyer: ResourceAddress) {
            self.minting_venue.remove(&buyer);
        }
        pub fn mint_standard_preview_nft(
            &mut self,
            mut payment: Bucket,
            no_editions: u64,
            permission: Proof,
        ) -> (Vec<NonFungibleBucket>, Bucket) {
            assert!(
                self.reveal_step == true,
                "[Mint Reveal] : This NFT doesn't have a reveal step enabled"
            );
            assert!(
                payment.amount() >= self.mint_price,
                "[Mint Preview NFT] : Insufficient funds to mint NFT"
            );
            assert!(
                payment.resource_address() == self.mint_currency,
                "[Mint Preview NFT] : Incorrect currency to mint NFT"
            );

            assert!(
                self.mint_id < self.initial_sale_cap,
                "[Mint Preview NFT] : sale cap reached"
            );

            if self.mint_id == self.initial_sale_cap {
                let reveal_mint = MintComplete {
                    mint_component: self.royalty_component,
                    resource_address: self.nft_manager.address(),
                };

                Runtime::emit_event(reveal_mint);
            }
            assert!(
                self.minting_venue
                    .get(&permission.resource_address())
                    .is_some(),
                "Buy not via authorized seller"
            );

            assert!(
                no_editions <= self.initial_sale_cap,
                "Exceeds the initial sale cap"
            );

            let time_now = Clock::current_time_rounded_to_seconds();
            assert!(
                self.mint_enabled_after.is_some() && time_now >= self.mint_enabled_after.unwrap(),
                "[Mint Preview NFT] : Minting not enabled yet"
            );

            let nft = NFT {
                name: self.mint_id.to_string(),
                description: self.description.to_string(),
                key_image_url: Url::of(self.preview_image_url.clone()),
                ipfs_uri: None,
                attributes: vec![],
            };

            let mut minted_editions: Vec<NonFungibleBucket> = vec![];

            for _ in 0..no_editions {
                let minted_edition = self.nft_manager.mint_non_fungible(
                    &NonFungibleLocalId::Integer(self.mint_id.into()),
                    nft.clone(),
                );
                minted_editions.push(minted_edition);
                self.mint_id += 1;
            }
            let dec_amount = Decimal::from(no_editions);
            self.mint_payments_vault
                .put(payment.take(dec_amount.checked_mul(self.mint_price).unwrap()));

            (minted_editions, payment)
        }

        /// This function allows users to buy a preview of an NFT before it is minted. This acts as a mechanism for random minting.
        /// Users pay for the mint cost and only a certain limit set by the cap can be minted.
        /// After the desired number of NFTs have been minted, then the creator can update the metadata on all or some of the NFTs.
        pub fn mint_preview_nft(
            &mut self,
            mut payment: Bucket,
            no_editions: u64,
            account: Global<Account>,
            permission: Proof,
        ) -> (Bucket, Vec<NonFungibleBucket>, Bucket) {
            assert!(
                self.reveal_step == true,
                "[Mint Reveal] : This NFT doesn't have a reveal step enabled"
            );
            assert!(
                payment.amount() >= self.mint_price,
                "[Mint Preview NFT] : Insufficient funds to mint NFT"
            );
            assert!(
                payment.resource_address() == self.mint_currency,
                "[Mint Preview NFT] : Incorrect currency to mint NFT"
            );

            assert!(
                self.mint_id < self.initial_sale_cap,
                "[Mint Preview NFT] : Collection cap reached"
            );

            let time_now = Clock::current_time_rounded_to_seconds();
            assert!(
                self.mint_enabled_after.is_some() && time_now >= self.mint_enabled_after.unwrap(),
                "[Mint Preview NFT] : Minting not enabled yet"
            );

            assert!(
                self.minting_venue
                    .get(&permission.resource_address())
                    .is_some(),
                "Buy not via authorized seller"
            );

            assert!(
                no_editions <= self.initial_sale_cap,
                "Exceeds the initial sale cap"
            );

            let mut minted_editions: Vec<NonFungibleBucket> = vec![];

            let nft = NFT {
                name: self.mint_id.to_string(),
                description: self.description.to_string(),
                key_image_url: Url::of(self.preview_image_url.clone()),
                ipfs_uri: None,
                attributes: vec![],
            };

            for _ in 0..no_editions {
                let minted_edition = self.nft_manager.mint_non_fungible(
                    &NonFungibleLocalId::Integer(self.mint_id.into()),
                    nft.clone(),
                );
                minted_editions.push(minted_edition);
                self.mint_id += 1;

                if self.mint_id == self.initial_sale_cap {
                    let mint_complete = MintComplete {
                        mint_component: self.royalty_component,
                        resource_address: self.nft_manager.address(),
                    };

                    Runtime::emit_event(mint_complete);
                    self.reveal_step = false;
                }
            }

            let transient_token = self.transient_tokens.take(1);

            let nflid_vec = minted_editions
                .iter()
                .map(|nft| nft.non_fungible_local_id())
                .collect::<Vec<NonFungibleLocalId>>();

            self.latest_transaction =
                Some((self.nft_manager.address(), nflid_vec, account.clone()));

            self.nft_manager.set_depositable(rule!(allow_all));

            let dec_amount = Decimal::from(no_editions);
            self.mint_payments_vault
                .put(payment.take(dec_amount.checked_mul(self.mint_price).unwrap()));

            // we return any change from the transaction and the preview NFT
            (payment, minted_editions, transient_token.into())
        }

        pub fn cleared(&mut self, transient_token: FungibleBucket) {
            assert!(
                transient_token.amount() == dec!(1),
                "Transient token amount must be 1"
            );

            assert!(
                transient_token.resource_address() == self.transient_token_address,
                "Transient token address must match"
            );

            assert!(self.latest_transaction.is_some(), "No transaction to clear");

            let (nft_resource, nft_local, account_recipient) =
                self.latest_transaction.clone().unwrap();

            for local_id in nft_local {
                assert!(
                    account_recipient.has_non_fungible(nft_resource, local_id),
                    "NFT not received by expected account {:?}",
                    account_recipient,
                );
            }

            let admin = self.transient_admin_vault.take(1);

            admin.authorize_with_all(|| {
                self.transient_tokens.put(transient_token.into());
            });

            self.transient_admin_vault.put(admin);

            // panic!("debug deposit");

            self.nft_manager.set_depositable(rule!(
                require_amount(1, self.depositer_admin)
                    || require(global_caller(self.royalty_component))
            ));
        }

        pub fn add_virtual_account_admin(&mut self, account: Global<Account>) {
            self.virtual_account_admin = Some(account);
        }

        pub fn remove_virtual_account_admin(&mut self) {
            let remove_account: Option<Global<Account>> = None;
            self.virtual_account_admin = remove_account;
        }

        // this functions allows the creator to upload the metadata for the NFTs to conduct the reveal
        pub fn upload_metadata(
            &mut self,
            auth_proof: Proof,
            data: Vec<(NonFungibleLocalId, NFTData)>,
        ) {
            if auth_proof.resource_address() == self.nft_creator_admin {
                auth_proof.check(self.nft_creator_admin);
            } else {
                if self.temp_admin {
                    auth_proof.check(self.virtual_admin_minting_badge.address());
                } else {
                    panic!("Unauthorized");
                }
            }

            for (nft_id, metadata) in data {
                self.metadata.insert(nft_id, metadata);
            }
        }

        // this function updates the metadata on an NFT that has already been minted to reveal the collection
        pub fn mint_reveal(&mut self, auth_proof: Proof, data: Vec<(NonFungibleLocalId, NFTData)>) {
            if auth_proof.resource_address() == self.nft_creator_admin {
                auth_proof.check(self.nft_creator_admin);
            } else {
                if self.temp_admin {
                    auth_proof.check(self.virtual_admin_minting_badge.address());
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
                require_amount(1, self.depositer_admin)
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
            self.temp_admin = !self.temp_admin;
        }
    }
}
