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
        marketplace_admin: ResourceAddress,
        marketplace_fee: Decimal,
        fee_vaults: KeyValueStore<ResourceAddress, Vault>,
        mint_fee: Decimal,
    }

    impl GenericMarketplace {
        pub fn start_marketplace(
            marketplace_fee: Decimal,
            mint_fee: Decimal,
            dapp_definition: ComponentAddress,
            admin_key: ResourceAddress,
        ) -> Global<GenericMarketplace> {
            let (marketplace_address_reservation, marketplace_component_address) =
                Runtime::allocate_component_address(GenericMarketplace::blueprint_id());

            let global_caller_badge_rule =
                rule!(require(global_caller(marketplace_component_address)));

            let marketplace_listing_key =
                ResourceBuilder::new_integer_non_fungible_with_registered_type::<
                    MarketPlacePermission,
                >(OwnerRole::Fixed(rule!(require(admin_key))))
                .mint_roles(mint_roles! {
                    minter => rule!(require(admin_key));
                    minter_updater => rule!(require(admin_key));
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
                        name: "Trove".to_string(),
                    },
                )]);

            let key_manager =
                ResourceManager::from_address(marketplace_listing_key.resource_address());

            let component_address = Self {
                marketplace_listing_key_vault: Vault::with_bucket(marketplace_listing_key.into()),
                marketplace_key_manager: key_manager,
                marketplace_admin: admin_key,
                marketplace_fee,
                fee_vaults: KeyValueStore::<ResourceAddress, Vault>::new_with_registered_type(),
                mint_fee,
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::Fixed(rule!(require(admin_key))))
            .metadata(metadata! (
                roles {
                    metadata_setter => rule!(require(
                        admin_key
                    ));
                    metadata_setter_updater => rule!(require(
                        admin_key
                    ));
                    metadata_locker => rule!(require(
                        admin_key
                    ));
                    metadata_locker_updater => rule!(require(
                        admin_key
                    ));
                },
                init {
                    "name" => "Trove".to_owned(), updatable;
                    "description" => "Trove Aggregator".to_owned(), updatable;
                    "dapp_definition" => dapp_definition, updatable;
                    "icon_url" => Url::of("https://trove.tools/trove%20square.png"), updatable;
                }
            ))
            .with_address(marketplace_address_reservation)
            .globalize();

            component_address
        }

        pub fn retrieve_internal_market_key(&mut self, proof: Proof) -> Bucket {
            proof.check(self.marketplace_admin);

            let key = self.marketplace_listing_key_vault.take_all();
            key
        }

        pub fn withdraw_fees(&mut self, resource_address: ResourceAddress, proof: Proof) -> Bucket {
            proof.check(self.marketplace_admin);
            let mut vault = self.fee_vaults.get_mut(&resource_address).unwrap();
            let fee = vault.take_all();
            fee
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

        pub fn multi_listing_honour_purchase(
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
                        "multi_purchase_honour_listing",
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
                        "multi_purchase_honour_listing",
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

        pub fn get_marketplace_key_address(&self) -> ResourceAddress {
            self.marketplace_listing_key_vault.resource_address()
        }

        pub fn purchase_preview_mint(
            &mut self,
            mut total_payment: Bucket, // User's total payment for minting and marketplace fees
            cost_per_item_from_minter: Decimal, // The price of one NFT item as set by the minter component
            quantity_to_mint: u64,              // Number of NFTs the user wants to mint
            user_account_recipient: Option<Global<Account>>, // Account to receive NFTs and transient token
            preview_mint_address: Global<AnyComponent>,      // The minter component
        ) -> (Bucket, Vec<NonFungibleBucket>, Option<Bucket>) {
            // Returns: (total_change_to_user, nfts_minted, optional_transient_token)
            assert!(
                self.mint_fee >= Decimal::ZERO,
                "[purchase_preview_mint] Marketplace mint_fee rate must be non-negative."
            );
            assert!(
                cost_per_item_from_minter >= Decimal::ZERO,
                "[purchase_preview_mint] cost_per_item_from_minter must be non-negative."
            );
            assert!(
                quantity_to_mint > 0,
                "[purchase_preview_mint] quantity_to_mint must be greater than zero."
            );

            // 1. Calculate the total cost due to the minter for the requested quantity.
            // This is the amount the minter component expects to receive for the items.
            let payment_due_to_minter_decimal = cost_per_item_from_minter
                .checked_mul(Decimal::from(quantity_to_mint))
                .unwrap_or_else(|| panic!("[purchase_preview_mint] Overflow calculating payment due to minter. Cost: {}, Quantity: {}", cost_per_item_from_minter, quantity_to_mint));

            // 2. Calculate the marketplace fee. This fee is a percentage of the payment_due_to_minter_decimal.
            let marketplace_fee_on_minter_payment_decimal = payment_due_to_minter_decimal
                .checked_mul(self.mint_fee) // self.mint_fee is the rate, e.g., Decimal("0.01") for 1%
                .unwrap_or_else(|| panic!("[purchase_preview_mint] Overflow calculating marketplace fee. Minter Payment: {}, Fee Rate: {}", payment_due_to_minter_decimal, self.mint_fee));

            // Round the marketplace fee to a takeable Decimal value (e.g., 18 decimal places).
            // RoundingMode::ToNearestMidpointAwayFromZero is a common financial rounding method.
            let marketplace_fee_to_take_decimal = marketplace_fee_on_minter_payment_decimal
                .checked_round(18, RoundingMode::ToNearestMidpointAwayFromZero)
                .unwrap();

            // 3. Calculate the total amount required by the protocol (minter + marketplace).
            let total_required_by_protocol_decimal = payment_due_to_minter_decimal
                .checked_add(marketplace_fee_to_take_decimal)
                .unwrap_or_else(|| panic!("[purchase_preview_mint] Overflow calculating total required by protocol. Minter Payment: {}, Fee: {}", payment_due_to_minter_decimal, marketplace_fee_to_take_decimal));

            // 4. Assert that the user's total payment is sufficient.
            assert!(
                total_payment.amount() >= total_required_by_protocol_decimal,
                "[purchase_preview_mint] Insufficient total payment. Required: {}, Provided: {}",
                total_required_by_protocol_decimal,
                total_payment.amount()
            );

            // 5. Take the payment intended for the minter from the total_payment.
            // WithdrawStrategy::Rounded(RoundingMode::ToZero) will take at most the specified amount,
            // rounding down if the bucket's divisibility requires it (unlikely for standard fungibles like XRD).
            let payment_for_minter_bucket = total_payment.take_advanced(
                payment_due_to_minter_decimal,
                WithdrawStrategy::Rounded(RoundingMode::ToZero),
            );

            // Sanity check: ensure we actually took what was expected for the minter.
            // This is crucial for the minter to receive the correct amount.
            assert_eq!(payment_for_minter_bucket.amount(), payment_due_to_minter_decimal,
                       "[purchase_preview_mint] Mismatch in amount taken for minter. Expected {}, Got {}. This could indicate an issue with total_payment or take_advanced behavior.", 
                       payment_due_to_minter_decimal, payment_for_minter_bucket.amount());

            // 6. Take the marketplace fee from the *remaining* total_payment.
            if marketplace_fee_to_take_decimal > Decimal::ZERO {
                let fee_bucket_for_storage = total_payment.take_advanced(
                    marketplace_fee_to_take_decimal,
                    WithdrawStrategy::Rounded(RoundingMode::ToZero),
                );
                // Sanity check for fee taken
                assert_eq!(fee_bucket_for_storage.amount(), marketplace_fee_to_take_decimal,
                           "[purchase_preview_mint] Mismatch in marketplace fee taken. Expected {}, Got {}",
                           marketplace_fee_to_take_decimal, fee_bucket_for_storage.amount());

                // Store the collected fee.
                let fee_resource = fee_bucket_for_storage.resource_address();
                let fee_vault_exists = self.fee_vaults.get(&fee_resource).is_some();
                if fee_vault_exists {
                    let mut vault = self.fee_vaults.get_mut(&fee_resource).unwrap();
                    vault.put(fee_bucket_for_storage);
                } else {
                    let new_fee_vault = Vault::with_bucket(fee_bucket_for_storage);
                    self.fee_vaults.insert(fee_resource, new_fee_vault);
                }
            }

            // 8. Create the marketplace permission proof (e.g., using a marketplace-specific key).
            let nflid = NonFungibleLocalId::integer(1u64.into()); // Example ID for marketplace key
            let marketplace_permission_proof: Proof = self
                .marketplace_listing_key_vault // Assuming this vault holds the marketplace's authorization keys
                .as_non_fungible()
                .create_proof_of_non_fungibles(&indexset![nflid])
                .into();

            // 9. Call the minter component's "mint_preview_nft" method.
            // The minter expects: (payment_for_items, quantity_to_mint, user_account_if_any, marketplace_permission_proof)
            // The minter returns: (change_from_minter, Vec<NonFungibleBucket_nfts>, Option<Bucket_transient_token>)
            let (mut change_from_minter_bucket, minted_nfts_vec, transient_token_opt_bucket) =
                preview_mint_address.call_raw::<(Bucket, Vec<NonFungibleBucket>, Option<Bucket>)>(
                    "mint_preview_nft",
                    scrypto_args!(
                        payment_for_minter_bucket,
                        quantity_to_mint,
                        user_account_recipient,
                        marketplace_permission_proof
                    ),
                );

            change_from_minter_bucket.put(total_payment);

            // If user_change_from_marketplace_bucket was zero, then change_from_minter_bucket (whether zero or not) is the correct total change.

            // Return the combined change, the minted NFTs, and the optional transient token.
            (
                change_from_minter_bucket,
                minted_nfts_vec,
                transient_token_opt_bucket,
            )
        }
    }
}
