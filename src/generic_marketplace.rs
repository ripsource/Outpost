use scrypto::prelude::*;

#[derive(ScryptoSbor, NonFungibleData)]
struct MarketPlacePermission {
    name: String,
}

#[derive(ScryptoSbor, NonFungibleData)]
struct AdminKey {}

#[blueprint]
#[types(ResourceAddress, Vault, MarketPlacePermission, AdminKey)]
mod generic_marketplace {

    struct GenericMarketplace {
        marketplace_listing_key_vault: Vault,
        marketplace_key_manager: ResourceManager,
        marketplace_admin: NonFungibleResourceManager,
        marketplace_fee: Decimal,
        fee_vaults: KeyValueStore<ResourceAddress, Vault>,
        mint_fee: Decimal,
    }

    impl GenericMarketplace {
        pub fn start_marketplace(
            marketplace_fee: Decimal,
            mint_fee: Decimal,
            dapp_definition: ComponentAddress,
        ) -> (Global<GenericMarketplace>, Bucket) {
            let (marketplace_address_reservation, marketplace_component_address) =
                Runtime::allocate_component_address(GenericMarketplace::blueprint_id());

            let global_caller_badge_rule =
                rule!(require(global_caller(marketplace_component_address)));

            let admin_key = ResourceBuilder::new_integer_non_fungible_with_registered_type::<
                AdminKey,
            >(OwnerRole::None)
            .mint_initial_supply([(1u64.into(), AdminKey {})]);

            let marketplace_listing_key =
                ResourceBuilder::new_integer_non_fungible_with_registered_type::<
                    MarketPlacePermission,
                >(OwnerRole::None)
                .mint_roles(mint_roles! {
                    minter => global_caller_badge_rule;
                    minter_updater => rule!(deny_all);
                })
                .metadata(metadata! {
                    init {
                    "marketplace_fee" => marketplace_fee, updatable;
                    "marketplace_address" => marketplace_component_address, updatable;
                    }
                })
                .mint_initial_supply([(
                    1u64.into(),
                    MarketPlacePermission {
                        name: "Generic Marketplace".to_string(),
                    },
                )]);

            let key_manager =
                ResourceManager::from_address(marketplace_listing_key.resource_address());

            let component_address = Self {
                marketplace_listing_key_vault: Vault::with_bucket(marketplace_listing_key.into()),
                marketplace_key_manager: key_manager,
                marketplace_admin: admin_key.resource_manager(),
                marketplace_fee,
                fee_vaults: KeyValueStore::<ResourceAddress, Vault>::new_with_registered_type(),
                mint_fee,
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::Fixed(rule!(require(
                admin_key.resource_address()
            ))))
            .metadata(metadata! (
                roles {
                    metadata_setter => rule!(require(
                        admin_key.resource_address()
                    ));
                    metadata_setter_updater => rule!(require(
                        admin_key.resource_address()
                    ));
                    metadata_locker => rule!(require(
                        admin_key.resource_address()
                    ));
                    metadata_locker_updater => rule!(require(
                        admin_key.resource_address()
                    ));
                },
                init {
                    "name" => "Trove".to_owned(), locked;
                    "description" => "Trove Aggregator".to_owned(), locked;
                    "dapp_definition" => dapp_definition, updatable;
                    "icon_url" => Url::of("https://trove.tools/trove%20square.png"), locked;
                }
            ))
            .with_address(marketplace_address_reservation)
            .globalize();

            (component_address, admin_key.into())
        }

        pub fn purchase_royal_listing(
            &mut self,
            nfgid: NonFungibleGlobalId,
            payment: FungibleBucket,
            open_sale_address: Global<AnyComponent>,
            account_recipient: Global<Account>,
        ) -> (Bucket, Bucket) {
            let nflid = NonFungibleLocalId::integer(1u64.into());
            let proof_creation: Proof = self
                .marketplace_listing_key_vault
                .as_non_fungible()
                .create_proof_of_non_fungibles(&indexset![nflid])
                .into();

            let fee_and_nft: (Bucket, Bucket, Option<Bucket>) =
                open_sale_address.call_raw::<(Bucket, Bucket, Option<Bucket>)>(
                    "purchase_royal_listing",
                    scrypto_args!(nfgid, payment, proof_creation, account_recipient),
                );

            let is_fee_returned = fee_and_nft.2.is_some();

            if is_fee_returned {
                let fee_returned = fee_and_nft.2.unwrap();

                let fee_resource = fee_returned.resource_address();

                let fee_vault_exists = self.fee_vaults.get(&fee_resource).is_some();

                if fee_vault_exists {
                    self.fee_vaults
                        .get_mut(&fee_resource)
                        .unwrap()
                        .put(fee_returned);
                } else {
                    let fee_vault = Vault::with_bucket(fee_returned);
                    self.fee_vaults.insert(fee_resource, fee_vault);
                }
            }

            // returns the nft and the transient token
            (fee_and_nft.0, fee_and_nft.1)
        }

        pub fn purchase_multi_royal_listing(
            &mut self,
            orders: Vec<(Global<AnyComponent>, NonFungibleGlobalId, Decimal)>,
            mut full_payment: FungibleBucket,
            account_recipient: Global<Account>,
        ) -> Vec<Bucket> {
            let nflid = NonFungibleLocalId::integer(1u64.into());
            let proof_creation: Proof = self
                .marketplace_listing_key_vault
                .as_non_fungible()
                .create_proof_of_non_fungibles(&indexset![nflid])
                .into();

            let mut grouped_orders: HashMap<
                Global<AnyComponent>,
                Vec<(NonFungibleGlobalId, Decimal)>,
            > = HashMap::new();

            let mut all_nfts: Vec<Bucket> = Vec::new();

            for (address, nfgid, payment) in orders {
                grouped_orders
                    .entry(address)
                    .or_insert_with(Vec::new)
                    .push((nfgid, payment));
            }

            // Process each group of orders
            for (address, order_group) in grouped_orders {
                if order_group.len() == 1 {
                    // Single purchase case
                    let (nfgid, payment_amount) = order_group.into_iter().next().unwrap();
                    let payment = full_payment.take(payment_amount);
                    // let mut result = address.call_raw::<(Bucket, Bucket, Option<Bucket>)>(
                    //     "purchase_royal_listing",
                    //     scrypto_args!(nfgid, payment, proof_creation.clone(), account_recipient),
                    // );

                    let result = address.call_raw::<(Bucket, Bucket, Option<Bucket>)>(
                        "purchase_multi_royal_listings",
                        scrypto_args!([nfgid], payment, account_recipient, proof_creation.clone()),
                    );

                    if result.2.is_some() {
                        let fee_returned = result.2.unwrap();
                        let fee_resource = fee_returned.resource_address();

                        let fee_vault_exists = self.fee_vaults.get(&fee_resource).is_some();

                        if fee_vault_exists {
                            self.fee_vaults
                                .get_mut(&fee_resource)
                                .unwrap()
                                .put(fee_returned);
                        } else {
                            let fee_vault = Vault::with_bucket(fee_returned);
                            self.fee_vaults.insert(fee_resource, fee_vault);
                        }
                    }
                    // Collect NFTs
                    all_nfts.push(result.0);
                    all_nfts.push(result.1);
                } else {
                    // Multi purchase case
                    let nfgids: Vec<NonFungibleGlobalId> =
                        order_group.iter().map(|(nfgid, _)| nfgid.clone()).collect();

                    // Combine all payments into one bucket
                    let total_payment: Decimal = order_group
                        .into_iter()
                        .map(|(_, payment)| payment)
                        .reduce(|acc, payment| acc + payment)
                        .unwrap();

                    let combined_payment = full_payment.take(total_payment);

                    let result = address.call_raw::<(Bucket, Bucket, Option<Bucket>)>(
                        "purchase_multi_royal_listings",
                        scrypto_args!(
                            nfgids,
                            combined_payment,
                            account_recipient,
                            proof_creation.clone()
                        ),
                    );

                    if result.2.is_some() {
                        let fee_returned = result.2.unwrap();
                        let fee_resource = fee_returned.resource_address();

                        let fee_vault_exists = self.fee_vaults.get(&fee_resource).is_some();

                        if fee_vault_exists {
                            self.fee_vaults
                                .get_mut(&fee_resource)
                                .unwrap()
                                .put(fee_returned);
                        } else {
                            let fee_vault = Vault::with_bucket(fee_returned);
                            self.fee_vaults.insert(fee_resource, fee_vault);
                        }
                    }

                    // Collect NFTs
                    all_nfts.push(result.0);
                    all_nfts.push(result.1);
                }
            }

            all_nfts.push(full_payment.into());
            all_nfts
        }

        pub fn multi_listing_purchase(
            &mut self,
            orders: Vec<(Global<AnyComponent>, NonFungibleGlobalId, Decimal)>,
            mut full_payment: FungibleBucket,
        ) -> Vec<Bucket> {
            // Group orders by trader account address
            let mut grouped_orders: HashMap<
                Global<AnyComponent>,
                Vec<(NonFungibleGlobalId, Decimal)>,
            > = HashMap::new();

            for (address, nfgid, payment) in orders {
                grouped_orders
                    .entry(address)
                    .or_insert_with(Vec::new)
                    .push((nfgid, payment));
            }

            let mut all_nfts: Vec<Bucket> = Vec::new();

            let nflid = NonFungibleLocalId::integer(1u64.into());
            let proof_creation: Proof = self
                .marketplace_listing_key_vault
                .as_non_fungible()
                .create_proof_of_non_fungibles(&indexset![nflid])
                .into();

            // Process each group of orders
            for (address, order_group) in grouped_orders {
                // Create proof for the entire group

                if order_group.len() == 1 {
                    // Single purchase case
                    let (nfgid, payment_amount) = order_group.into_iter().next().unwrap();
                    let payment = full_payment.take(payment_amount);
                    // let mut result = address.call_raw::<(Vec<Bucket>, Vec<Bucket>)>(
                    //     "purchase_listing",
                    //     scrypto_args!(nfgid, payment, proof_creation.clone()),
                    // );

                    let mut result = address.call_raw::<(Vec<Bucket>, Vec<Bucket>)>(
                        "multi_purchase_listing",
                        scrypto_args!([nfgid], payment, proof_creation.clone()),
                    );

                    // Handle fee
                    let fee_returned = result.1.pop().unwrap();
                    let fee_resource = fee_returned.resource_address();

                    // Store fee in vault
                    let fee_vault_exists = self.fee_vaults.get(&fee_resource).is_some();

                    if fee_vault_exists {
                        self.fee_vaults
                            .get_mut(&fee_resource)
                            .unwrap()
                            .put(fee_returned);
                    } else {
                        let fee_vault = Vault::with_bucket(fee_returned);
                        self.fee_vaults.insert(fee_resource, fee_vault);
                    }

                    // Collect NFTs
                    all_nfts.extend(result.0);
                } else {
                    // Multi purchase case
                    let nfgids: Vec<NonFungibleGlobalId> =
                        order_group.iter().map(|(nfgid, _)| nfgid.clone()).collect();

                    // Combine all payments into one bucket
                    let total_payment: Decimal = order_group
                        .into_iter()
                        .map(|(_, payment)| payment)
                        .reduce(|acc, payment| acc + payment)
                        .unwrap();

                    let combined_payment = full_payment.take(total_payment);

                    let mut result = address.call_raw::<(Vec<Bucket>, Vec<Bucket>)>(
                        "multi_purchase_listing",
                        scrypto_args!(nfgids, combined_payment, proof_creation.clone()),
                    );

                    // Handle fee
                    let fee_returned = result.1.pop().unwrap();
                    let fee_resource = fee_returned.resource_address();

                    // Store fee in vault
                    let fee_vault_exists = self.fee_vaults.get(&fee_resource).is_some();

                    if fee_vault_exists {
                        self.fee_vaults
                            .get_mut(&fee_resource)
                            .unwrap()
                            .put(fee_returned);
                    } else {
                        let fee_vault = Vault::with_bucket(fee_returned);
                        self.fee_vaults.insert(fee_resource, fee_vault);
                    }

                    // Collect NFTs
                    all_nfts.extend(result.0);
                }
            }
            all_nfts.push(full_payment.into());
            all_nfts
        }

        pub fn purchase_listing(
            &mut self,
            nfgid: NonFungibleGlobalId,
            payment: FungibleBucket,
            trader_account_address: Global<AnyComponent>,
        ) -> Vec<Bucket> {
            let nflid = NonFungibleLocalId::integer(1u64.into());
            let proof_creation: Proof = self
                .marketplace_listing_key_vault
                .as_non_fungible()
                .create_proof_of_non_fungibles(&indexset![nflid])
                .into();

            let mut fee_and_nft: (Vec<Bucket>, Vec<Bucket>) =
                trader_account_address.call_raw::<(Vec<Bucket>, Vec<Bucket>)>(
                    "purchase_listing",
                    scrypto_args!(nfgid, payment, proof_creation),
                );

            let fee_returned = fee_and_nft.1.pop().unwrap();

            let fee_resource = fee_returned.resource_address();

            let fee_vault_exists = self.fee_vaults.get(&fee_resource).is_some();

            if fee_vault_exists {
                self.fee_vaults
                    .get_mut(&fee_resource)
                    .unwrap()
                    .put(fee_returned);
            } else {
                let fee_vault = Vault::with_bucket(fee_returned);
                self.fee_vaults.insert(fee_resource, fee_vault);
            }

            fee_and_nft.0
        }

        pub fn purchase_preview_mint(
            &mut self,
            payment: Bucket,
            amount: u64,
            fee: Bucket,
            account: Option<Global<Account>>,
            preview_mint_address: Global<AnyComponent>,
        ) -> (Bucket, Vec<NonFungibleBucket>, Option<Bucket>) {
            let fee_expected = payment.amount() * self.mint_fee;

            assert!(fee.amount() >= fee_expected);

            let fee_resource = fee.resource_address();
            let fee_vault_exists = self.fee_vaults.get(&fee_resource).is_some();

            if fee_vault_exists {
                self.fee_vaults.get_mut(&fee_resource).unwrap().put(fee);
            } else {
                let fee_vault = Vault::with_bucket(fee);
                self.fee_vaults.insert(fee_resource, fee_vault);
            }

            let nflid = NonFungibleLocalId::integer(1u64.into());
            let proof_creation: Proof = self
                .marketplace_listing_key_vault
                .as_non_fungible()
                .create_proof_of_non_fungibles(&indexset![nflid])
                .into();

            let receipt_and_change: (Bucket, Vec<NonFungibleBucket>, Option<Bucket>) =
                preview_mint_address.call_raw::<(Bucket, Vec<NonFungibleBucket>, Option<Bucket>)>(
                    "mint_preview_nft",
                    scrypto_args!(payment, amount, account, proof_creation),
                );

            receipt_and_change
        }

        pub fn get_marketplace_key_address(&self) -> ResourceAddress {
            self.marketplace_listing_key_vault.resource_address()
        }
    }
}
