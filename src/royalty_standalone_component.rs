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
    honoured: bool,
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
pub struct RoyaltyConfigInput {
    pub depositer_admin: ResourceAddress,
    pub royalties_enabled: bool,
    pub royalty_percent: Decimal,
    pub maximum_royalty_percent: Decimal,
    pub honoured: bool,
}

#[derive(ScryptoSbor)]
pub struct AdminConfig {
    pub nft_creator_admin_manager: NonFungibleResourceManager,
    pub nft_creator_admin: ResourceAddress,
    pub depositer_admin: ResourceAddress,
    pub depositer_rule: AccessRule,
   
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


#[derive(ScryptoSbor, NonFungibleData)]
struct CreatorKey {
    collection: String,
    authority: String,
   
    royalty_component: ComponentAddress,
}

#[blueprint]
mod royal_nft {


    enable_method_auth! {
    roles {
        admin => updatable_by: [];
    },
    methods {

        creator_admin => PUBLIC;

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

        deposit_via_router => PUBLIC;

        cleared => PUBLIC;
        get_transient_token_address => PUBLIC;

        withdraw_from_royalty_vault => restrict_to: [admin];
        remove_royalty_config => restrict_to: [admin];

        pay_royalty_basic => PUBLIC;
    }
    }

    struct RoyalNFTs {
        admin_config: AdminConfig,
        royalty_config: RoyaltyConfig,
        royalty_vaults: KeyValueStore<ResourceAddress, Vault>,
        royalty_component: ComponentAddress,
        transaction_tracking: TransactionTracking,
        nft_manager: ResourceManager,
    }

    impl RoyalNFTs {
        pub fn start_royalty_nft(
            name: String,
            resource_address: ResourceAddress,
            royalty_config_input: RoyaltyConfigInput,
            dapp_deff: GlobalAddress,
        ) -> (Global<RoyalNFTs>, NonFungibleBucket) {
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
                honoured: royalty_config_input.honoured,
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
                        "icon_url" => Url::of("https://www.outpost.trade/img/outpost_symbol.png"), locked;
                        "royalty_component" => royalty_component_address, locked;
                    }
                })
                .mint_initial_supply([(local_id_string, CreatorKey {
                    collection: name.clone(),
                    authority: "Admin".to_owned(),
                  
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
                if royalty_config_input.honoured {
                    depositer_admin_rule = rule!(allow_all);
                    change_movement_restrictions_rule =
                        rule!(require(nft_creator_admin.resource_address()));
                } else {
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
                }
            } else {
                depositer_admin_rule = rule!(allow_all);
                change_movement_restrictions_rule =
                    rule!(require(nft_creator_admin.resource_address()));
            }

            let nft_manager = ResourceManager::from_address(resource_address);

           

            let component_name = format!("{} OP", name);

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

            let admin_config = AdminConfig {
                nft_creator_admin_manager: nft_creator_admin.resource_manager(),
                nft_creator_admin: nft_creator_admin.resource_address(),
                depositer_admin: royalty_config_input.depositer_admin,
                depositer_rule: depositer_admin_rule,
               
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
                admin_config,
                royalty_config,
                royalty_vaults: KeyValueStore::new(),
                royalty_component: royalty_component_address.clone(),
                transaction_tracking,
                nft_manager,
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
                    "icon_url" => Url::of("https://www.outpost.trade/img/outpost_symbol.png"), locked;
                }
            ))
            .roles(roles!(
                admin => rule!(require(nft_creator_admin.resource_address()));

            ))
            .globalize();

            (component_adresss, nft_creator_admin)
        }

        // helper method for tests

        pub fn creator_admin(&self) -> ResourceAddress {
            self.admin_config.nft_creator_admin
        }

        pub fn withdraw_from_royalty_vault(&mut self, currency: ResourceAddress) -> Bucket {
            let mut royalty_vault = self.royalty_vaults.get_mut(&currency).unwrap();

            royalty_vault.take_all()
        }

        pub fn get_transient_token_address(&self) -> ResourceAddress {
            self.transaction_tracking.transient_token_address
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
            local_ids: indexmap::IndexSet<NonFungibleLocalId>,
            mut payment: Bucket,
            buyer: ResourceAddress,
            account: Global<Account>,
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

        pub fn pay_royalty_basic(
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

            self.nft_manager
                .set_depositable(self.admin_config.depositer_rule.clone());

            optional_returned_buckets
        }

        pub fn deposit_via_router(
            &mut self,
            nft: Bucket,
            permission: Proof,
            dapp: ComponentAddress,
            mut account: Global<Account>,
        ) {
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

            account.try_deposit_or_abort(nft.into(), None);
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
    }
}
