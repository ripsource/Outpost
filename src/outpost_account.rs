use crate::outpost_event::event;
use scrypto::component::AccountLocker;
use scrypto::prelude::*;
/// This blueprint is a trader account - where they can list items and where items are purchased from. Each method calls the event emitter component.
/// A trader account has two sets of methods for listing and purchases - one for royalty enforced NFTs and one for standard NFTs.
/// The trader account stores a emitter badge that is used to authenticate event emitters from each trader account and allows traders to buy and sell Royalty NFTs
/// by providing authentication to the deposit rules on an Royalty NFT.
///
///
/// Account lockers are used to store sales revenue, while first attempting to deposit directly to an account.

#[derive(ScryptoSbor, Clone)]
pub struct Listing {
    /// The permissions that a secondary seller must have to sell an NFT. This is used to ensure that only selected
    /// marketplaces or private buyers can buy an NFT.
    pub secondary_seller_permissions: Vec<ResourceAddress>,
    /// The seller is able to decide what currency they want to sell their NFT in (e.g. XRD, FLOOP, EARLY, HUG)
    pub currency: ResourceAddress,
    /// The price of the NFT - this price will be subject to marketplace fees and creator royalties which are taken as a % of this amount.
    pub price: Decimal,
    /// The NFTGID being recorded is potentially redundant as it is the key of the listing in the listings key value store.
    /// The actual NFT is stored in the key value store of vaults separately.
    pub nfgid: NonFungibleGlobalId,
    /// trader's account address - helpful for aggregators to know where to fetch listings from.
    pub outpost_account: ComponentAddress,
}

type Unit = ();

#[blueprint]
#[types(Listing, ResourceAddress, NonFungibleGlobalId, Vault, Hash, Unit)]
#[events(ListingCreated, ListingUpdated, ListingCanceled, ListingPurchased)]
mod opentrader {

    enable_package_royalties! {
        create_trader => Xrd(dec!(0.000000000000000001).into());
        list => Xrd(dec!(0.000000000000000001).into());
        royal_list => Xrd(dec!(0.000000000000000001).into());
        same_owner_royal_transfer => Free;
        transfer_royal_nft_to_component => Free;
        revoke_market_permission => Xrd(dec!(0.000000000000000001).into());
        add_buyer_permission => Xrd(dec!(0.000000000000000001).into());
        change_price => Xrd(dec!(0.000000000000000001).into());
        cancel_listing => Xrd(dec!(0.000000000000000001).into());
        cancel_royal_listing => Xrd(dec!(0.000000000000000001).into());
        multi_list => Xrd(dec!(0.000000000000000001).into());
        purchase_royal_listing => Xrd(dec!(0.000000000000000001).into());
        purchase_listing => Xrd(dec!(0.000000000000000001).into());
        multi_purchase_listing => Xrd(dec!(0.000000000000000001).into());
        purchase_multi_royal_listings => Xrd(dec!(0.000000000000000001).into());
        royal_multi_list => Xrd(dec!(0.000000000000000001).into());
        fetch_auth_key => Free;
        cleared => Free;
        multi_cleared => Free;
        transient_token_address => Free;
    }

    enable_method_auth! {
    roles {
        admin => updatable_by: [];
    },
    methods {
        list => restrict_to: [admin];
        royal_list => restrict_to: [admin];
        royal_multi_list => restrict_to: [admin];
        same_owner_royal_transfer => restrict_to: [admin];
        transfer_royal_nft_to_component => restrict_to: [admin];
        revoke_market_permission => restrict_to: [admin];
        add_buyer_permission => restrict_to: [admin];
        change_price => restrict_to: [admin];
        cancel_listing => restrict_to: [admin];
        cancel_royal_listing => restrict_to: [admin];
        multi_list => restrict_to: [admin];
        purchase_royal_listing => PUBLIC;
        purchase_listing => PUBLIC;
        multi_purchase_listing => PUBLIC;
        fetch_auth_key => PUBLIC;
        cleared => PUBLIC;
        multi_cleared => PUBLIC;
        transient_token_address => PUBLIC;
        purchase_multi_royal_listings => PUBLIC;
    }
    }

    struct OpenTrader {
        /// The key value store of listings information for NFTs the user has listed for sale.
        listings: KeyValueStore<NonFungibleGlobalId, Listing>,
        /// The key value store of vaults that store all the NFTs that the user has listed for sale.
        nft_vaults: KeyValueStore<ResourceAddress, Vault>,
        /// The key value store of vaults that store all the revenue the user has made from sales.
        /// This is used to store the revenue until the user claims it. However a future ambition is to use AccountLockers.
        /// Multiple currencies are supported.
        sales_revenue: KeyValueStore<ResourceAddress, Vault>,
        /// The royal admin badge that is used to authenticate deposits of Royalty NFTs.
        /// A user should never be able to withdraw this badge or access it in a unintended manner.
        royal_admin: Vault,
        /// The emitter badge that is used to authenticate event emitters from each trader account.
        /// A user should never be able to withdraw this badge or access it in a unintended manner.
        emitter_badge: Vault,
        /// The local id of the emitter badge that is used to authenticate event emitters from each trader account.
        emitter_badge_local: NonFungibleLocalId,
        /// the central event emitter component that is used to emit events for all trades.
        event_manager: Global<event::Event>,
        /// The trading account badge resource address. This badge is held by the user and is used to authenticate methods on their trading account.
        auth_key_resource: ResourceAddress,
        /// The trading account badge local id. This badge is held by the user and is used to authenticate methods on their trading account.
        auth_key_local: NonFungibleLocalId,
        /// AccountLockers to be added
        account_locker: Global<AccountLocker>,
        /// Trader Linked Account
        my_account: Global<Account>,
        /// This users trading account component address
        trader_account_component_address: ComponentAddress,
        /// This kvs tracks the royal listing transactions made on the account, preventing double method calls for royalty NFTs.
        transactions: KeyValueStore<Hash, ()>,
        /// Latest transaction tracker for confirming the transaction has been cleared
        latest_transaction: Option<(ResourceAddress, NonFungibleLocalId, Global<Account>)>,
        /// Latest bulk transaction tracker for confirming the transaction has been cleared
        latest_bulk_transaction: Option<(Global<Account>, Vec<NonFungibleGlobalId>)>,
        /// Transient tokens for clearing transactions
        transient_tokens: Vault,
        /// Transient token address
        transient_token_address: ResourceAddress,
    }

    impl OpenTrader {
        /// creates a new trader account. This function should be called via the OpenTradeFactory component in order to be
        /// populated with the correct badges and permissions.
        pub fn create_trader(
            auth_key: NonFungibleGlobalId,
            my_account: Global<Account>,
            emitter_badge: Bucket,
            depositer_admin: Bucket,
            event_manager: Global<event::Event>,
            dapp_global: GlobalAddress,
            locker: Global<AccountLocker>,
            transient_token: FungibleBucket,
        ) -> Global<OpenTrader> {
            let (trader_address_reservation, trader_component_address) =
                Runtime::allocate_component_address(OpenTrader::blueprint_id());
            // let global_caller_badge_rule = rule!(require(global_caller(trader_component_address)));

            let (auth_key_resource, auth_key_local) = auth_key.clone().into_parts();

            let emitter_badge_local = emitter_badge.as_non_fungible().non_fungible_local_id();

            let transient_token_address = transient_token.resource_address();

            let mut transient_token_vault = Vault::new(transient_token.resource_address());

            depositer_admin.authorize_with_all(|| {
                transient_token_vault.put(transient_token.into());
            });

            Self {
                auth_key_local,
                auth_key_resource,
                listings: KeyValueStore::<NonFungibleGlobalId, Listing>::new_with_registered_type(),
                account_locker: locker,
                my_account,
                emitter_badge: Vault::with_bucket(emitter_badge),
                emitter_badge_local,
                event_manager,
                trader_account_component_address: trader_component_address,
                nft_vaults: KeyValueStore::<ResourceAddress, Vault>::new_with_registered_type(),
                sales_revenue: KeyValueStore::<ResourceAddress, Vault>::new_with_registered_type(),
                royal_admin: Vault::with_bucket(depositer_admin),
                transactions: KeyValueStore::<Hash, Unit>::new_with_registered_type(),
                latest_transaction: None,
                latest_bulk_transaction: None,
                transient_tokens: transient_token_vault,
                transient_token_address,
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::None)
            .metadata(metadata! (
                roles {
                    metadata_setter => rule!(deny_all);
                    metadata_setter_updater => rule!(deny_all);
                    metadata_locker => rule!(deny_all);
                    metadata_locker_updater => rule!(deny_all);
                },
                init {
                    "name" => "Outpost Account".to_owned(), locked;
                    "description" => "An Outpost Account".to_owned(), locked;
                    "dapp_definition" => dapp_global, locked;
                    "icon_url" => Url::of("https://www.outpost.trade/img/outpost_symbol.png"), locked;
                }
            ))
            .roles(roles!(
                admin => rule!(require(auth_key));
            ))
            .with_address(trader_address_reservation)
            .globalize()
        }

        //👑👑👑  Royalty Enforced Methods 👑👑👑 //

        pub fn royal_multi_list(
            &mut self,
            listings: Vec<(NonFungibleGlobalId, Decimal)>,
            currency: ResourceAddress,
            permissions: Vec<ResourceAddress>,
            items: NonFungibleBucket,
        ) {
            // We take the hash of the listing as to prevent a user from listing and selling an NFT in the same tx - i.e.
            // calling the list method and purchase method within the same transaction which could be used to send an NFT to another user for free
            // without any risk of someone sniping it.
            let transaction_hash = Runtime::transaction_hash();

            self.transactions.insert(transaction_hash, ());

            let full_listings: Vec<Listing> = listings
                .iter()
                .map(|(nfgid, price)| {
                    let outpost_account = self.trader_account_component_address;

                    let new_listing = Listing {
                        secondary_seller_permissions: permissions.clone(),
                        currency,
                        price: *price,
                        nfgid: nfgid.clone(),
                        outpost_account,
                    };

                    self.listings.insert(nfgid.clone(), new_listing.clone());

                    new_listing
                })
                .collect();

            // Validate the bucket is not empty
            assert!(!items.is_empty(), "[multi_list] No NFTs provided");

            // Get the resource address and local IDs from the bucket
            let bucket_resource_address = items.resource_address();
            let bucket_local_ids = items.non_fungible_local_ids();

            // Validate all NFGIDs match the bucket's resource address and collect local IDs
            let mut listing_local_ids = IndexSet::new();
            for (nfgid, price) in listings.iter() {
                let (resource_address, local_id) = nfgid.clone().into_parts();

                assert!(
                    resource_address == bucket_resource_address,
                    "[multi_list] All NFTs must be from the same collection"
                );

                assert!(
                    *price > Decimal::zero(),
                    "[multi_list] All listing prices must be greater than zero"
                );

                listing_local_ids.insert(local_id);
            }

            // Verify that all listed NFTs are present in the bucket
            assert!(
                bucket_local_ids.is_superset(&listing_local_ids),
                "[multi_list] Not all listed NFTs are present in the provided bucket"
            );

            self.multi_listing_event(full_listings);

            let nft_address = items.resource_address();

            self.royal_admin.as_fungible().authorize_with_amount(1, || {
                // Store the NFT in the vault
                let vault_exists = self.nft_vaults.get(&nft_address.clone()).is_some();
                if vault_exists {
                    let mut vault = self
                        .nft_vaults
                        .get_mut(&nft_address.clone())
                        .expect("[multi_list] NFT not found");
                    vault.put(items.into());
                } else {
                    self.nft_vaults
                        .insert(nft_address.clone(), Vault::with_bucket(items.into()));
                }
            });
        }

        /// Lists an NFT for sale by the user. The user provides the NFT, the price, the currency,
        /// and the ResourceAddress of a badge that a secondary seller must have to sell the NFT.
        /// We don't issue badges to Marketplaces, we just assume they have a badge that a user can easily select to mean
        /// they want to list on their marketplace. In reality, a user will likley just check a box for Trove, Foton and XRDegen, etc.
        /// and doesn't need to know the badge address.
        pub fn royal_list(
            &mut self,
            // The NFT to list for sale
            nft_to_list: NonFungibleBucket,
            // The price of the NFT - this price will be subject to marketplace fees and creator royalties which are taken as a % of this amount.
            price: Decimal,
            // The currency the NFT is listed in
            currency: ResourceAddress,
            // The permissions that a secondary seller must have to sell an NFT. This is used to ensure that only selected
            // marketplaces or private buyers can buy an NFT.
            permissions: Vec<ResourceAddress>,
            // The badge that is used to authenticate the user listing the NFT
            // trader_badge: Proof,
        ) {
            // authenticate user happens at a system level

            assert!(
                price > Decimal::zero(),
                "[list_nft] Listing price must be greater than zero"
            );

            assert!(
                nft_to_list.amount() == dec!(1),
                "[list_nft] Only one NFT can be listed at a time"
            );

            // Gather data from the NFT to complete all the information needed to list the NFT

            let nft_address = nft_to_list.resource_address();

            let id = nft_to_list.non_fungible_local_id();

            let nfgid = NonFungibleGlobalId::new(nft_address.clone(), id.clone());

            // We take the hash of the listing as to prevent a user from listing and selling an NFT in the same tx - i.e.
            // calling the list method and purchase method within the same transaction which could be used to send an NFT to another user for free
            // without any risk of someone sniping it.

            let transaction_hash = Runtime::transaction_hash();

            self.transactions.insert(transaction_hash, ());

            let outpost_account = self.trader_account_component_address;

            let new_listing = Listing {
                secondary_seller_permissions: permissions,
                currency,
                price,
                nfgid: nfgid.clone(),
                outpost_account,
            };

            // add the listing information. We don't need to worry about
            // duplicating as a listing key entry is always removed when and NFT is sold
            // or if the listing is cancelled.
            self.listings.insert(nfgid.clone(), new_listing.clone());

            // As this is a royalty enforced listing, we need to use the royalty admin badge
            // to authenticate the deposit of the NFT.
            // As its not possible to delete vaults that are empty, we need to check if one has been
            // created for this NFT previously. If so, we just use the existing vault - otherwise, we create a new one.
            self.royal_admin.as_fungible().authorize_with_amount(1, || {
                let vault_exists = self.nft_vaults.get(&nft_address.clone()).is_some();

                if vault_exists {
                    let mut vault = self
                        .nft_vaults
                        .get_mut(&nft_address.clone())
                        .expect("[royal_list] NFT not found");
                    vault.put(nft_to_list.into());
                } else {
                    self.nft_vaults
                        .insert(nft_address, Vault::with_bucket(nft_to_list.into()));
                }
            });

            self.listing_event(new_listing, nfgid.clone());
        }

        pub fn purchase_multi_royal_listings(
            &mut self,
            nfgids: Vec<NonFungibleGlobalId>,
            payment: FungibleBucket,
            account_recipient: Global<Account>,
            permission: Proof,
        ) -> (Bucket, Bucket, Option<Bucket>) {
            // set the latest bulk transaction for later verification and clearing
            self.latest_bulk_transaction = Some((account_recipient, nfgids.clone()));

            let marketplace = permission.resource_address();

            let listings: Vec<Listing> = nfgids
                .iter()
                .map(|nfgid| {
                    let listing = self
                        .listings
                        .get(nfgid)
                        .expect("[purchase] Listing not found");

                    assert!(
                        listing.secondary_seller_permissions.contains(&marketplace),
                        "[purchase] Marketplace does not have permission to purchase this listing"
                    );

                    listing.clone()
                })
                .collect();

            // Calculate total price and verify all currencies match
            let first_currency = listings[0].currency;
            let total_price: Decimal = listings.iter().fold(dec!(0), |acc, listing| {
                assert!(
                    listing.currency == first_currency,
                    "[purchase] All listings must use the same currency"
                );
                acc.checked_add(listing.price).unwrap()
            });

            // Verify payment amount and currency
            assert!(
                payment.resource_address() == first_currency,
                "[purchase] Payment currency does not match listing currency"
            );
            assert!(
                payment.amount() == total_price,
                "[purchase] Payment amount does not match total listing price"
            );

            // Calculate marketplace fee
            let marketplace_fee_option: Option<Decimal> = permission
                .skip_checking()
                .resource_manager()
                .get_metadata("marketplace_fee")
                .unwrap();

            let marketplace_fee = if let Some(marketplace_fee_rate) = marketplace_fee_option {
                payment.amount().checked_mul(marketplace_fee_rate).unwrap()
            } else {
                dec!(0)
            };
            // Prepare return buckets
            let nft_bucket: Bucket;
            let mut marketplace_fee_bucket: Option<Bucket> = None;

            {
                let transaction_hash = Runtime::transaction_hash();

                assert!(
    self.transactions.get(&transaction_hash).is_none(),
    "[purchase] Purchasing a listing within the same transaction it is listed is blocked."
);
                let nft_address = nfgids[0].resource_address();
                // Process all NFTs

                let local_id_index_set: indexmap::IndexSet<NonFungibleLocalId> = nfgids
                    .iter()
                    .map(|nfgid| nfgid.local_id().clone())
                    .collect();

                // Take NFT from vault
                let nft = self
                    .nft_vaults
                    .get_mut(&nft_address)
                    .expect("[purchase] NFT not found");

                nft_bucket = nft
                    .as_non_fungible()
                    .take_non_fungibles(&local_id_index_set)
                    .into();

                for nfgid in nfgids.iter() {
                    // Remove listing
                    self.listings.remove(nfgid);
                }

                let nft_manager = ResourceManager::from_address(nft_address);

                self.royal_admin.as_fungible().authorize_with_amount(1, || {
                    nft_manager.set_depositable(rule!(allow_all));
                });

                let royalty_component_global_address: GlobalAddress = nft_manager
                    .get_metadata("royalty_component")
                    .unwrap()
                    .unwrap();

                let royalty_component =
                    ComponentAddress::new_or_panic(royalty_component_global_address.into());

                let call_address: Global<AnyComponent> = Global(ObjectStub::new(
                    ObjectStubHandle::Global(GlobalAddress::from(royalty_component)),
                ));

                // We send the full payment to the royalty component so that it can take its %fee.
                // We also provide the trading permission to check against any other permissions the creator has set.
                let mut remainder_after_royalty: Bucket =
                    Global::<AnyComponent>::from(call_address).call_raw(
                        "pay_royalty",
                        scrypto_args!(
                            nft_address,
                            local_id_index_set,
                            payment,
                            marketplace,
                            account_recipient
                        ),
                    );

                // we then take the marketplaces fee (we've already calculated this earlier based on the full payment amount).

                if marketplace_fee_option.is_some() {
                    marketplace_fee_bucket = Some(remainder_after_royalty.take_advanced(
                        marketplace_fee,
                        WithdrawStrategy::Rounded(RoundingMode::ToZero),
                    ));
                } else {
                    marketplace_fee_bucket = None;
                }

                // Create emitter proof and emit bulk event
                let emitter_proof = self
                    .emitter_badge
                    .as_non_fungible()
                    .create_proof_of_non_fungibles(&indexset![self.emitter_badge_local.clone()]);
                // Store the remaining payment
                emitter_proof.authorize(|| {
                    self.account_locker.store(
                        self.my_account,
                        remainder_after_royalty.into(),
                        true,
                    );
                });
            }

            // self.event_manager
            //     .multi_purchase_event(listings, emitter_proof.clone().into());
            {
                self.multi_purchase_event(listings);
            }

            // We turn off deposit restrictions. However a transient token will be emitted by this method that will be used to clear the transaction
            // an set the deposit rules again.

            (
                nft_bucket,
                self.transient_tokens.take(1),
                marketplace_fee_bucket,
            )
        }

        pub fn multi_cleared(&mut self, transient_token: FungibleBucket) {
            // assert!(
            //     transient_token.amount() == dec!(1),
            //     "Transient token amount must be 1"
            // );

            assert!(
                transient_token.resource_address() == self.transient_token_address,
                "Transient token address must match"
            );

            assert!(
                self.latest_bulk_transaction.is_some(),
                "No transaction to clear"
            );

            let (account_recipient, nfgids) = self.latest_bulk_transaction.clone().unwrap();

            let nft_address = nfgids[0].resource_address();

            let local_id_index_set: IndexSet<NonFungibleLocalId> = nfgids
                .iter()
                .map(|nfgid| nfgid.local_id().clone())
                .collect();

            for local_id in local_id_index_set {
                assert!(
                    account_recipient.has_non_fungible(nft_address, local_id),
                    "NFT not received by expected account"
                );
            }

            let nft_manager = ResourceManager::from_address(nft_address);

            let royalty_component_global_address: GlobalAddress = nft_manager
                .get_metadata("royalty_component")
                .unwrap()
                .unwrap();

            let royalty_component =
                ComponentAddress::new_or_panic(royalty_component_global_address.into());

            self.royal_admin
                .as_fungible()
                .authorize_with_amount(1, || self.transient_tokens.put(transient_token.into()));

            self.royal_admin.as_fungible().authorize_with_amount(1, || {
                nft_manager.set_depositable(rule!(
                    require(self.royal_admin.resource_address())
                        || require(global_caller(royalty_component))
                ));
            });
        }

        /// The intention is that in the majority of cases, a marketplace would call this method using their
        /// marketplace badge to authenticate the purchase, get the NFT and return it to the user on their platform.
        /// However, for a private deal, a user could call this method directly with a badge issued by the listing creator for this deal.
        pub fn purchase_royal_listing(
            &mut self,
            // The NFGID of the NFT to purchase
            nfgid: NonFungibleGlobalId,
            // The payment for the NFT
            payment: FungibleBucket,
            // The badge of the marketplace or private buyer that is purchasing the NFT
            permission: Proof,
            // The account that the NFT should be sent to
            account_recipient: Global<Account>,
        ) -> (Bucket, Bucket, Option<Bucket>) {
            let purchased_nft: Bucket;

            // log current transaction for later verification and clearing
            self.latest_transaction = Some((
                nfgid.resource_address(),
                nfgid.local_id().clone(),
                account_recipient.clone(),
            ));

            let mut marketplace_fee_bucket: Option<Bucket> = None;
            let listing_event: Listing;

            let (nft_address, nft_local) = nfgid.clone().into_parts();

            // First authenticate the proof to check that the marketplace or private buyer has the correct permissions to purchase the NFT
            // We are just using a resource address as validation here - however this could be a more complex check in the future for local ids
            // so that for private deals a brand new resource doesn't need to be created.

            let trading_permission = permission.resource_address();

            {
                let listing_permission = self
                    .listings
                    .get(&nfgid)
                    .expect("[purchase] Listing not found");

                assert!(
                    listing_permission
                        .secondary_seller_permissions
                        .contains(&trading_permission),
                    "[purchase] Marketplace does not have permission to purchase this listing"
                );
            }

            // We get the marketplace fee rate from the metadata of the proof
            // We calculate the marketplace fee from the payment amount.
            // This could be an unsafe decimal at this point - however when taking from the payment we use a safe rounding mode.
            // If not marketplace fee is set, we set the rate to 0.

            let marketplace_fee_option: Option<Decimal> = permission
                .skip_checking()
                .resource_manager()
                .get_metadata("marketplace_fee")
                .unwrap();

            let marketplace_fee_rate: Decimal;
            let marketplace_fee: Decimal;
            if marketplace_fee_option.is_some() {
                marketplace_fee_rate = marketplace_fee_option.unwrap();
                marketplace_fee = payment.amount().checked_mul(marketplace_fee_rate).unwrap();
            } else {
                marketplace_fee = dec!(0);
            };

            // We retrieve basic information about the listing, such as price, currency and time of the listing.
            {
                let listing = self
                    .listings
                    .get_mut(&nfgid)
                    .expect("[purchase] Listing not found");

                listing_event = listing.clone();

                let price = listing.price;

                assert!(
                    payment.amount() == price,
                    "[purchase] Payment amount does not match listing price"
                );

                let currency = listing.currency;

                assert!(
                    payment.resource_address() == currency,
                    "[purchase] Payment currency does not match listing currency",
                );

                // As mentioned elsewhere - we want to ensure no one can do an atomic transaction of listing and purchasing a Royalty NFT
                // as this would provide a loophole for trading NFTs without paying royalties. We do this by checking the hash of the listing
                // and the hash of the purchase. If they are the same, we abort the transaction.

                let transaction_hash = Runtime::transaction_hash();

                assert!(
                    self.transactions.get(&transaction_hash).is_none(),
                    "[purchase] Purchasing a listing within the same transaction it is listed is blocked."
                );

                // We get the NFT from the vault

                let vault = self
                    .nft_vaults
                    .get_mut(&nft_address)
                    .expect("[purchase] NFT not found");

                purchased_nft = vault.as_non_fungible().take_non_fungible(&nft_local).into();

                // We get the royalty component address from the NFT metadata

                let nft_manager = ResourceManager::from_address(nft_address);

                let royalty_component_global_address: GlobalAddress = nft_manager
                    .get_metadata("royalty_component")
                    .unwrap()
                    .unwrap();

                let royalty_component =
                    ComponentAddress::new_or_panic(royalty_component_global_address.into());

                let call_address: Global<AnyComponent> = Global(ObjectStub::new(
                    ObjectStubHandle::Global(GlobalAddress::from(royalty_component)),
                ));

                // We send the full payment to the royalty component so that it can take its %fee.
                // We also provide the trading permission to check against any other permissions the creator has set.
                let mut remainder_after_royalty: Bucket =
                    Global::<AnyComponent>::from(call_address).call_raw(
                        "pay_royalty_basic",
                        scrypto_args!(nft_address, payment, trading_permission),
                    );

                // we then take the marketplaces fee (we've already calculated this earlier based on the full payment amount).

                if marketplace_fee_option.is_some() {
                    marketplace_fee_bucket = Some(remainder_after_royalty.take_advanced(
                        marketplace_fee,
                        WithdrawStrategy::Rounded(RoundingMode::ToZero),
                    ));
                }

                let locker_proof = self
                    .emitter_badge
                    .as_non_fungible()
                    .create_proof_of_non_fungibles(&indexset![self.emitter_badge_local.clone()]);
                // Take the payment for the NFT
                locker_proof.authorize(|| {
                    self.account_locker.store(
                        self.my_account,
                        remainder_after_royalty.into(),
                        true,
                    );
                });

                // We turn off deposit restrictions. However a transient token will be emitted by this method that will be used to clear the transaction
                // an set the deposit rules again.

                self.royal_admin.as_fungible().authorize_with_amount(1, || {
                    nft_manager.set_depositable(rule!(allow_all));
                });
            }
            self.listings.remove(&nfgid);

            // finally we emit a listing event via the event emitter component

            self.purchase_listing_event(listing_event, nfgid);

            (
                purchased_nft,
                self.transient_tokens.take(1),
                marketplace_fee_bucket,
            )
        }

        pub fn transient_token_address(&self) -> ResourceAddress {
            self.transient_token_address
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

            assert!(
                account_recipient.has_non_fungible(nft_resource, nft_local),
                "NFT not received by expected account"
            );

            let nft_manager = ResourceManager::from_address(nft_resource);

            let royalty_component_global_address: GlobalAddress = nft_manager
                .get_metadata("royalty_component")
                .unwrap()
                .unwrap();

            let royalty_component =
                ComponentAddress::new_or_panic(royalty_component_global_address.into());

            self.royal_admin
                .as_fungible()
                .authorize_with_amount(1, || self.transient_tokens.put(transient_token.into()));

            self.royal_admin.as_fungible().authorize_with_amount(1, || {
                nft_manager.set_depositable(rule!(
                    require(self.royal_admin.resource_address())
                        || require(global_caller(royalty_component))
                ));
            });
        }

        pub fn cancel_royal_listing(&mut self, nfgid: NonFungibleGlobalId) {
            let mut nft_bucket: Vec<Bucket> = vec![];

            let (nft_address, nft_local) = nfgid.clone().into_parts();

            {
                let nft = self
                    .nft_vaults
                    .get_mut(&nft_address)
                    .expect("[cancel] NFT not found");

                nft_bucket.push(nft.as_non_fungible().take_non_fungible(&nft_local).into());
            }
            {
                let listing = self
                    .listings
                    .get(&nfgid.clone())
                    .expect("[change_price] Listing not found");

                self.cancel_listing_event(listing.clone(), nfgid.clone());
            }

            self.listings.remove(&nfgid);

            self.royal_admin.as_fungible().authorize_with_amount(1, || {
                self.my_account
                    .try_deposit_or_abort(nft_bucket.pop().unwrap().into(), None);
            });
        }

        /// Using the bottlenose update's ned owner_role assertion, we can ensure that a user can transfer an NFT to another account that they own
        /// without need to pay a royalty or fee.
        pub fn same_owner_royal_transfer(
            &mut self,
            royalty_nft: Bucket,
            mut recipient: Global<Account>,
        ) {
            {
                // Getting the owner role of the account.
                let owner_role = recipient.get_owner_role();

                // Assert against it.
                Runtime::assert_access_rule(owner_role.rule);

                // Assertion passed - the caller is the owner of the account.
            }

            self.royal_admin.as_fungible().authorize_with_amount(1, || {
                recipient.try_deposit_or_abort(royalty_nft.into(), None);
            });
        }

        /// Transfers an NFT to a component. This method is used to transfer an NFT to a component that is not an account.
        /// This can only work if the Royalty NFT's configuration allows the dapp to receive the NFT. The NFT creator
        /// must have permissioned the dapp in their royalty component if they've chosen to turn on dapp limits.
        /// Allowing transfers to components opens a lot of possibilities for the user to create new and interesting use cases
        /// however it also allows loopholes for avoiding royalties. The creator of a collection should be aware of this.
        /// We effectively turn off the restrictions for deposits, do some foreign method, then turn them back on so a
        /// dapp can do what they need to with the asset.
        /// We provide an optional return of a vector of buckets, which should cover most use cases.
        pub fn transfer_royal_nft_to_component(
            &mut self,
            royalty_nft: Bucket,
            // the component of the dapp you want to transfer the NFT to
            component: Global<AnyComponent>,
            // the name of the method you want to use on this component (i.e. pub fn deposit, etc.)
            custom_method: String,
            // optional return vec of buckets for things like badges reciepts, etc. from the dapp
            // should we add the option to be able to send some other asset with the NFT to the dapp?
        ) -> Option<Vec<Bucket>> {
            // we get the package address of the component
            let package_address = component.blueprint_id().package_address;

            // we get the well-known package address of the account components
            let my_bech32_address =
                "package_rdx1pkgxxxxxxxxxaccntxxxxxxxxxx000929625493xxxxxxxxxaccntx";
            let global_account_address = PackageAddress::try_from_bech32(
                &AddressBech32Decoder::new(&NetworkDefinition::mainnet()),
                &my_bech32_address,
            )
            .unwrap();

            // check that we're not passing the asset to a global account address. This is important
            // to ensure someone isn't bypassing royalties by using this channel to send an NFT to another account.
            assert!(
                package_address != global_account_address,
                "Component can not be an account component"
            );

            // Each Royalty NFT has its royalty component addres in its top-level resource metadata
            let royalty_nft_manager = ResourceManager::from_address(royalty_nft.resource_address());

            let royalty_component_global_address: GlobalAddress = royalty_nft_manager
                .get_metadata("royalty_component")
                .unwrap()
                .unwrap();

            let royalty_component =
                ComponentAddress::new_or_panic(royalty_component_global_address.into());

            let call_address: Global<AnyComponent> = Global(ObjectStub::new(
                ObjectStubHandle::Global(GlobalAddress::from(royalty_component)),
            ));

            // We don't need to authorise anything here as deposits will be authorised from the royalty component.

            let returned_buckets_full: Option<Vec<Bucket>> =
                Global::<AnyComponent>::from(call_address).call_raw::<Option<Vec<Bucket>>>(
                    "transfer_royalty_nft_to_dapp",
                    scrypto_args!(royalty_nft, component, custom_method.clone()),
                );

            returned_buckets_full
        }

        //
        // General royalty/non-royalty related Methods //
        //

        pub fn multi_list(
            &mut self,
            listings: Vec<(NonFungibleGlobalId, Decimal)>,
            currency: ResourceAddress,
            permissions: Vec<ResourceAddress>,
            items: NonFungibleBucket,
        ) {
            let full_listings: Vec<Listing> = listings
                .iter()
                .map(|(nfgid, price)| {
                    let outpost_account = self.trader_account_component_address;

                    let new_listing = Listing {
                        secondary_seller_permissions: permissions.clone(),
                        currency,
                        price: *price,
                        nfgid: nfgid.clone(),
                        outpost_account,
                    };

                    self.listings.insert(nfgid.clone(), new_listing.clone());

                    new_listing
                })
                .collect();

            // Validate the bucket is not empty
            assert!(!items.is_empty(), "[multi_list] No NFTs provided");

            // Get the resource address and local IDs from the bucket
            let bucket_resource_address = items.resource_address();
            let bucket_local_ids = items.non_fungible_local_ids();

            // Validate all NFGIDs match the bucket's resource address and collect local IDs
            let mut listing_local_ids = IndexSet::new();
            for (nfgid, price) in listings.iter() {
                let (resource_address, local_id) = nfgid.clone().into_parts();

                assert!(
                    resource_address == bucket_resource_address,
                    "[multi_list] All NFTs must be from the same collection"
                );

                assert!(
                    *price > Decimal::zero(),
                    "[multi_list] All listing prices must be greater than zero"
                );

                listing_local_ids.insert(local_id);
            }

            // Verify that all listed NFTs are present in the bucket
            assert!(
                bucket_local_ids.is_superset(&listing_local_ids),
                "[multi_list] Not all listed NFTs are present in the provided bucket"
            );

            self.multi_listing_event(full_listings);

            let nft_address = items.resource_address();
            // Store the NFT in the vault
            let vault_exists = self.nft_vaults.get(&nft_address.clone()).is_some();
            if vault_exists {
                let mut vault = self
                    .nft_vaults
                    .get_mut(&nft_address.clone())
                    .expect("[multi_list] NFT not found");
                vault.put(items.into());
            } else {
                self.nft_vaults
                    .insert(nft_address.clone(), Vault::with_bucket(items.into()));
            }
        }

        pub fn list(
            &mut self,
            nft_bucket: Bucket,
            currency: ResourceAddress,
            price: Decimal,
            permissions: Vec<ResourceAddress>,
        ) {
            assert!(!nft_bucket.is_empty(), "[list_nft] No NFT provided");

            assert!(
                price > Decimal::zero(),
                "[list_nft] Listing price must be greater than zero"
            );

            assert!(
                nft_bucket.amount() == dec!(1),
                "[list_nft] Only one NFT can be listed at a time"
            );

            let nfgid = NonFungibleGlobalId::new(
                nft_bucket.resource_address(),
                nft_bucket.as_non_fungible().non_fungible_local_id(),
            );

            let outpost_account = self.trader_account_component_address;

            let new_listing = Listing {
                secondary_seller_permissions: permissions,
                currency,
                price,
                nfgid: nfgid.clone(),
                outpost_account,
            };

            let nft_address = nft_bucket.resource_address();

            let vault_exists = self.nft_vaults.get(&nft_address.clone()).is_some();

            if vault_exists {
                let mut vault = self
                    .nft_vaults
                    .get_mut(&nft_address.clone())
                    .expect("[royal_list] NFT not found");
                vault.put(nft_bucket.into());
            } else {
                self.nft_vaults
                    .insert(nft_address.clone(), Vault::with_bucket(nft_bucket.into()));
            }

            self.listings.insert(nfgid.clone(), new_listing.clone());

            self.listing_event(new_listing, nfgid);
        }

        pub fn revoke_market_permission(
            &mut self,
            nft_id: NonFungibleGlobalId,
            permission_id: ResourceAddress,
        ) {
            {
                let mut listing = self
                    .listings
                    .get_mut(&nft_id)
                    .expect("[revoke_permission] Listing not found");

                listing
                    .secondary_seller_permissions
                    .retain(|permissions| permissions != &permission_id);
            }

            let listing = self
                .listings
                .get(&nft_id)
                .expect("[revoke_permission] Listing not found");

            self.update_listing_event(listing.clone(), nft_id);
        }

        pub fn add_buyer_permission(
            &mut self,
            nft_id: NonFungibleGlobalId,
            permission_id: ResourceAddress,
        ) {
            {
                let mut listing = self
                    .listings
                    .get_mut(&nft_id)
                    .expect("[add_permission] Listing not found");

                listing.secondary_seller_permissions.push(permission_id);
            }

            let listing = self
                .listings
                .get(&nft_id)
                .expect("[add_permission] Listing not found");

            self.update_listing_event(listing.clone(), nft_id);
        }

        pub fn change_price(&mut self, nft_id: NonFungibleGlobalId, new_price: Decimal) {
            {
                let mut listing = self
                    .listings
                    .get_mut(&nft_id)
                    .expect("[change_price] Listing not found");
                listing.price = new_price;
            }

            let listing = self
                .listings
                .get(&nft_id)
                .expect("[change_price] Listing not found");

            self.update_listing_event(listing.clone(), nft_id);
        }

        pub fn cancel_listing(&mut self, nft_id: NonFungibleGlobalId) -> Vec<Bucket> {
            let mut nft_bucket: Vec<Bucket> = vec![];

            let (nft_address, local_id) = nft_id.clone().into_parts();

            {
                let nft = self
                    .nft_vaults
                    .get_mut(&nft_address)
                    .expect("[cancel] NFT not found");

                nft_bucket.push(nft.as_non_fungible().take_non_fungible(&local_id).into());
            }
            {
                let listing = self
                    .listings
                    .get(&nft_id)
                    .expect("[change_price] Listing not found");

                self.cancel_listing_event(listing.clone(), nft_id.clone());
            }

            self.listings.remove(&nft_id);

            nft_bucket
        }

        pub fn multi_purchase_listing(
            &mut self,
            nfgids: Vec<NonFungibleGlobalId>,
            mut payment: FungibleBucket,
            permission: Proof,
        ) -> (Vec<Bucket>, Vec<Bucket>) {
            // Validate all listings exist and marketplace has permission
            let marketplace = permission.resource_address();
            let listings: Vec<Listing> = nfgids
                .iter()
                .map(|nfgid| {
                    let listing = self
                        .listings
                        .get(nfgid)
                        .expect("[purchase] Listing not found");

                    assert!(
                        listing.secondary_seller_permissions.contains(&marketplace),
                        "[purchase] Marketplace does not have permission to purchase this listing"
                    );

                    listing.clone()
                })
                .collect();

            // Calculate total price and verify all currencies match
            let first_currency = listings[0].currency;
            let total_price: Decimal = listings.iter().fold(dec!(0), |acc, listing| {
                assert!(
                    listing.currency == first_currency,
                    "[purchase] All listings must use the same currency"
                );
                acc.checked_add(listing.price).unwrap()
            });

            // Verify payment amount and currency
            assert!(
                payment.resource_address() == first_currency,
                "[purchase] Payment currency does not match listing currency"
            );
            assert!(
                payment.amount() == total_price,
                "[purchase] Payment amount does not match total listing price"
            );

            // Calculate marketplace fee
            let marketplace_fee_option: Option<Decimal> = permission
                .skip_checking()
                .resource_manager()
                .get_metadata("marketplace_fee")
                .unwrap();

            let marketplace_fee = if let Some(marketplace_fee_rate) = marketplace_fee_option {
                payment.amount().checked_mul(marketplace_fee_rate).unwrap()
            } else {
                dec!(0)
            };

            // Prepare return buckets
            let mut nft_buckets: Vec<Bucket> = Vec::with_capacity(nfgids.len());
            let mut fee_buckets: Vec<Bucket> = Vec::new();

            // Take marketplace fee if applicable
            if marketplace_fee > dec!(0) {
                let marketplace_payment = payment.take_advanced(
                    marketplace_fee,
                    WithdrawStrategy::Rounded(RoundingMode::ToZero),
                );
                fee_buckets.push(marketplace_payment.into());
            }

            // Process all NFTs

            for nfgid in nfgids.iter() {
                let (nft_resource, nft_local) = nfgid.clone().into_parts();

                // Take NFT from vault
                let nft = self
                    .nft_vaults
                    .get_mut(&nft_resource)
                    .expect("[purchase] NFT not found");
                nft_buckets.push(nft.as_non_fungible().take_non_fungible(&nft_local).into());

                // Remove listing
                self.listings.remove(nfgid);
            }

            // Create emitter proof and emit bulk event
            let emitter_proof = self
                .emitter_badge
                .as_non_fungible()
                .create_proof_of_non_fungibles(&indexset![self.emitter_badge_local.clone()]);

            self.multi_purchase_event(listings);

            // Store the remaining payment
            emitter_proof.authorize(|| {
                self.account_locker
                    .store(self.my_account, payment.into(), true);
            });

            (nft_buckets, fee_buckets)
        }

        pub fn purchase_listing(
            &mut self,
            nfgid: NonFungibleGlobalId,
            mut payment: FungibleBucket,
            permission: Proof,
        ) -> (Vec<Bucket>, Vec<Bucket>) {
            let (nft_address, nft_local) = nfgid.clone().into_parts();

            let mut return_buckets: (Vec<Bucket>, Vec<Bucket>) = (vec![], vec![]);
            let listing_event: Listing;

            {
                let marketplace = permission.resource_address();

                let listing_permission = self
                    .listings
                    .get(&nfgid)
                    .expect("[purchase] Listing not found");

                assert!(
                    listing_permission
                        .secondary_seller_permissions
                        .contains(&marketplace),
                    "[purchase] Marketplace does not have permission to purchase this listing"
                );
            }
            // We get the marketplace fee rate from the metadata of the proof
            // We calculate the marketplace fee from the payment amount.
            // This could be an unsafe decimal at this point - however when taking from the payment we use a safe rounding mode.
            // If not marketplace fee is set, we set the rate to 0.

            let marketplace_fee_option: Option<Decimal> = permission
                .skip_checking()
                .resource_manager()
                .get_metadata("marketplace_fee")
                .unwrap();

            let marketplace_fee_rate: Decimal;
            let marketplace_fee: Decimal;
            if marketplace_fee_option.is_some() {
                marketplace_fee_rate = marketplace_fee_option.unwrap();
                marketplace_fee = payment.amount().checked_mul(marketplace_fee_rate).unwrap();
            } else {
                marketplace_fee = dec!(0);
            };

            {
                let listing = self
                    .listings
                    .get_mut(&nfgid)
                    .expect("[purchase] Listing not found");

                listing_event = listing.clone();

                let price = listing.price;

                assert!(
                    payment.amount() == price,
                    "[purchase] Payment amount does not match listing price"
                );

                let currency = listing.currency;

                assert!(
                    payment.resource_address() == currency,
                    "[purchase] Payment currency does not match listing currency",
                );

                {
                    let nft = self
                        .nft_vaults
                        .get_mut(&nft_address)
                        .expect("[cancel] NFT not found");

                    return_buckets
                        .0
                        .push(nft.as_non_fungible().take_non_fungible(&nft_local).into());
                }
            }

            // Take marketplace fee
            if marketplace_fee > dec!(0) {
                let marketplace_payment = payment.take_advanced(
                    marketplace_fee,
                    WithdrawStrategy::Rounded(RoundingMode::ToZero),
                );

                return_buckets.1.push(marketplace_payment.into());
            }

            // finally we emit a listing event via the event emitter component
            let emitter_proof = self
                .emitter_badge
                .as_non_fungible()
                .create_proof_of_non_fungibles(&indexset![self.emitter_badge_local.clone()]);

            // // Take the payment for the NFT
            emitter_proof.clone().authorize(|| {
                self.account_locker
                    .store(self.my_account, payment.into(), true);
            });

            self.purchase_listing_event(listing_event, nfgid.clone());

            self.listings.remove(&nfgid);

            return_buckets
        }

        // utility methods

        pub fn fetch_auth_key(&self) -> (ResourceAddress, NonFungibleLocalId) {
            (self.auth_key_resource, self.auth_key_local.clone())
        }

        // event emittors
        fn listing_event(&self, listing: Listing, nft_id: NonFungibleGlobalId) {
            Runtime::emit_event(ListingCreated {
                listing: listing.clone(),
                outpost_account: listing.outpost_account,
                nft_id,
            });
        }

        fn update_listing_event(&self, listing: Listing, nft_id: NonFungibleGlobalId) {
            Runtime::emit_event(ListingUpdated {
                listing: listing.clone(),
                outpost_account: listing.outpost_account,
                nft_id,
            });
        }

        fn cancel_listing_event(&self, listing: Listing, nft_id: NonFungibleGlobalId) {
            Runtime::emit_event(ListingCanceled {
                listing: listing.clone(),
                outpost_account: listing.outpost_account,
                nft_id,
            });
        }

        fn purchase_listing_event(&self, listing: Listing, nft_id: NonFungibleGlobalId) {
            Runtime::emit_event(ListingPurchased {
                listing: listing.clone(),
                outpost_account: listing.outpost_account,
                nft_id,
            });
        }

        fn multi_listing_event(&self, listings: Vec<Listing>) {
            // Use zip to iterate over both vectors simultaneously
            for listing in listings {
                Runtime::emit_event(ListingCreated {
                    listing: listing.clone(),
                    outpost_account: listing.outpost_account,
                    nft_id: listing.nfgid, // Wrap single NFT ID in a vector to match event structure
                });
            }
        }

        fn multi_purchase_event(&self, listings: Vec<Listing>) {
            // Use zip to iterate over both vectors simultaneously
            for listing in listings {
                Runtime::emit_event(ListingPurchased {
                    listing: listing.clone(),
                    outpost_account: listing.outpost_account,
                    nft_id: listing.nfgid, // Wrap single NFT ID in a vector to match event structure
                });
            }
        }
    }
}

#[derive(ScryptoSbor, ScryptoEvent)]
struct ListingCreated {
    listing: Listing,
    outpost_account: ComponentAddress,
    nft_id: NonFungibleGlobalId,
}

#[derive(ScryptoSbor, ScryptoEvent)]
struct ListingUpdated {
    listing: Listing,
    outpost_account: ComponentAddress,
    nft_id: NonFungibleGlobalId,
}

#[derive(ScryptoSbor, ScryptoEvent)]
struct ListingCanceled {
    listing: Listing,
    outpost_account: ComponentAddress,
    nft_id: NonFungibleGlobalId,
}

#[derive(ScryptoSbor, ScryptoEvent)]
struct ListingPurchased {
    listing: Listing,
    outpost_account: ComponentAddress,
    nft_id: NonFungibleGlobalId,
}
