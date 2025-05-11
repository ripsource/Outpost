use outpost::minter::NFTData;
use scrypto::prelude::*;
use scrypto_test::prelude::*;

use crate::common::*;

pub fn enable_mint_reveal(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
    marketplace_resource: ResourceAddress,
    mint_price: Decimal,
    initial_sale_cap: u64,
    mint_start: Instant,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .call_method(
            component,
            "enable_mint_reveal",
            manifest_args!(
                initial_sale_cap,
                mint_start,
                mint_price,
                vec![marketplace_resource]
            ),
        )
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}

pub fn purchase_preview_mint_via_marketplace(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    nft_address: ResourceAddress,
    cost_per_item: Decimal,
    amount: u64,
    transient_address: ResourceAddress,
    mint_component: ComponentAddress,
) {
    let price = cost_per_item * amount;
    let fee = dec!(0.02) * price;

    let total_withdraw = price + fee;

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "withdraw",
            manifest_args!(XRD, total_withdraw),
        )
        .take_all_from_worktop(XRD, "payment")
        .with_name_lookup(|builder, lookup| {
            builder.call_method(
                component,
                "purchase_preview_mint",
                manifest_args!(
                    lookup.bucket("payment"),
                    cost_per_item,
                    amount,
                    Some(user.account),
                    mint_component
                ),
            )

            // pub fn purchase_preview_mint(
            //     &mut self,
            //     mut total_payment: Bucket, // User's total payment for minting and marketplace fees
            //     cost_per_item_from_minter: Decimal, // The price of one NFT item as set by the minter component
            //     quantity_to_mint: u64,    // Number of NFTs the user wants to mint
            //     user_account_recipient: Option<Global<Account>>, // Account to receive NFTs and transient token
            //     preview_mint_address: Global<AnyComponent>, // The minter component
        })
        .take_all_from_worktop(nft_address, "bucket1")
        .call_method_with_name_lookup(user.account, "deposit", |lookup| {
            manifest_args!(lookup.bucket("bucket1"))
        })
        .take_all_from_worktop(transient_address, "bucket2")
        .call_method_with_name_lookup(mint_component, "cleared", |lookup| {
            manifest_args!(lookup.bucket("bucket2"))
        })
        .call_method(
            user.account,
            "deposit_batch",
            manifest_args!(ManifestExpression::EntireWorktop),
        )
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );
    receipt.expect_commit(true);
}

pub fn mint_royalty_nft(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    nft_address: ResourceAddress,
    amount: u64,
    transient_address: ResourceAddress,
) {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(user.account, "withdraw", manifest_args!(XRD, dec!(100)))
        .take_all_from_worktop(XRD, "payment")
        .with_name_lookup(|builder, lookup| {
            builder.call_method(
                component,
                "mint_preview_nft",
                manifest_args!(lookup.bucket("payment"), amount, user.account,),
            )
        })
        .take_all_from_worktop(nft_address, "bucket1")
        .call_method_with_name_lookup(user.account, "deposit", |lookup| {
            manifest_args!(lookup.bucket("bucket1"))
        })
        .take_all_from_worktop(transient_address, "bucket2")
        .call_method_with_name_lookup(component, "cleared", |lookup| {
            manifest_args!(lookup.bucket("bucket2"))
        })
        .call_method(
            user.account,
            "deposit_batch",
            manifest_args!(ManifestExpression::EntireWorktop),
        )
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    if !receipt.is_commit_success() {
        println!("{:?}", receipt);
        panic!("TRANSACTION FAIL");
    }
}

//admin protect direct mint, returns to creator without any payment required.
//    pub fn direct_mint(
//     &mut self,
//     data: Vec<(
//         NonFungibleLocalId,
//         (String, String, String, Vec<HashMap<String, String>>),
//     )>,
// ) -> Vec<Bucket> {

#[derive(ScryptoSbor, ManifestDecode, ManifestEncode, ManifestCategorize)]
pub struct Other {
    pub name: String,
    pub description: String,
    pub key_image_url: Url,
    pub attributes: Vec<HashMap<String, String>>,
    pub ipfs_uri: Option<String>,
}

pub fn direct_mint(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    id: u64,
    creator_key: ResourceAddress,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    // #[derive(ScryptoSbor)]
    // pub struct NFTData {
    //     pub name: String,
    //     pub description: String,
    //     pub key_image_url: Url,
    //     pub attributes: Vec<HashMap<String, String>>,
    //     pub ipfs_uri: Option<String>,
    // }

    let data: Vec<Other> = vec![Other {
        name: "name".to_string(),
        description: "description".to_string(),
        key_image_url: Url::of("https://i.scdn.co/image/ab67616d0000b2735d02af8588949bf7ee2f0a08"),
        attributes: vec![HashMap::new()],
        ipfs_uri: None,
    }];

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id.clone()]),
        )
        .pop_from_auth_zone("creator_proof")
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .call_method_with_name_lookup(component, "direct_mint", |lookup| {
            manifest_args!(lookup.proof("creator_proof"), Some(user.account), data)
        })
        .call_method(
            user.account,
            "deposit_batch",
            manifest_args!(ManifestExpression::EntireWorktop),
        )
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    if !receipt.is_commit_success() {
        println!("{:?}", receipt);
        panic!("TRANSACTION FAIL");
    }
}

pub fn get_nft_address(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
) -> ResourceAddress {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(component, "get_nft_address", manifest_args!())
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true).output(1)
}

pub fn get_transient_address(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
) -> ResourceAddress {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(component, "get_transient_token_address", manifest_args!())
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true).output(1)
}

pub fn nft_address(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
) -> ResourceAddress {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(component, "resource_address", manifest_args!())
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true).output(1)
}

pub fn create_global_id(nft_address: ResourceAddress, number: u64) -> NonFungibleGlobalId {
    let local_id: NonFungibleLocalId = NonFungibleLocalId::integer(number.into());
    NonFungibleGlobalId::new(nft_address.clone(), local_id.clone())
}

pub fn update_burn_role(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    nft_address: ResourceAddress,
    creator_key: ResourceAddress,
    new_rule: AccessRule,
) {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .create_proof_from_account_of_amount(user.account, creator_key, dec!(1))
        .set_role(nft_address, ModuleId::Main, "burner", new_rule)
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}

pub fn lock_burn_rule(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .pop_from_auth_zone("new_proof")
        .with_name_lookup(|builder, lookup| {
            builder.call_method(
                component,
                "lock_burn_rule",
                manifest_args!(lookup.proof("new_proof")),
            )
        })
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}

pub fn update_metadata_updatable_rule(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    nft_address: ResourceAddress,
    creator_key: ResourceAddress,
    new_rule: AccessRule,
) {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .create_proof_from_account_of_amount(user.account, creator_key, dec!(1))
        .set_role(
            nft_address,
            ModuleId::Main,
            "non_fungible_data_updater",
            new_rule,
        )
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    if !receipt.is_commit_success() {
        println!("{:?}", receipt);
        panic!("TRANSACTION FAIL");
    }
}

pub fn lock_metadata_updatable_rule(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .pop_from_auth_zone("creator_proof")
        .with_name_lookup(|builder, lookup| {
            builder.call_method(
                component,
                "lock_metadata_updatable_rule",
                manifest_args!(lookup.proof("creator_proof")),
            )
        })
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}

pub fn change_mint_rule(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
    new_rule: AccessRule,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .pop_from_auth_zone("creator_proof")
        .with_name_lookup(|builder, lookup| {
            builder.call_method(
                component,
                "change_mint_rule",
                manifest_args!(new_rule, lookup.proof("creator_proof")),
            )
        })
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}

pub fn lock_mint_rule(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .pop_from_auth_zone("creator_proof")
        .with_name_lookup(|builder, lookup| {
            builder.call_method(
                component,
                "lock_mint_rule",
                manifest_args!(lookup.proof("creator_proof")),
            )
        })
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}

pub fn change_royalty_percentage_fee(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
    new_fee: Decimal,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .call_method(
            component,
            "change_royalty_percentage_fee",
            manifest_args!(new_fee),
        )
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}

pub fn lower_maximum_royalty_percentage_fee(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
    new_fee: Decimal,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .call_method(
            component,
            "lower_maximum_royalty_percentage",
            manifest_args!(new_fee),
        )
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}

pub fn restrict_currencies_false(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .call_method(component, "restrict_currencies_false", manifest_args!())
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}

pub fn restrict_currencies_true(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .call_method(component, "restrict_currencies_true", manifest_args!())
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}

pub fn add_permitted_currency(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
    add_currency: ResourceAddress,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .call_method(
            component,
            "add_permitted_currency",
            manifest_args!(add_currency),
        )
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}

pub fn remove_permitted_currency(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
    remove_currency: ResourceAddress,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .call_method(
            component,
            "remove_permitted_currency",
            manifest_args!(remove_currency),
        )
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}

pub fn enable_minimum_royalties(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .call_method(component, "enable_minimum_royalties", manifest_args!())
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    if !receipt.is_commit_success() {
        println!("{:?}", receipt);
        panic!("TRANSACTION FAIL");
    }
}

pub fn disable_minimum_royalties(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .call_method(component, "disable_minimum_royalties", manifest_args!())
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    if !receipt.is_commit_success() {
        println!("{:?}", receipt);
        panic!("TRANSACTION FAIL");
    }
}

pub fn set_minimum_royalty_amount(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
    currency: ResourceAddress,
    new_minimum: Decimal,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .call_method(
            component,
            "set_minimum_royalty_amount",
            manifest_args!(currency, new_minimum),
        )
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}

pub fn remove_minimum_royalty_amount(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
    currency: ResourceAddress,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .call_method(
            component,
            "remove_minimum_royalty_amount",
            manifest_args!(currency),
        )
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}

pub fn add_permissioned_dapp(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
    dapp: ComponentAddress,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .call_method(component, "add_permissioned_dapp", manifest_args!(dapp))
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}

pub fn remove_permissioned_dapp(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
    dapp: ComponentAddress,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .call_method(component, "remove_permissioned_dapp", manifest_args!(dapp))
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}

pub fn add_permissioned_buyer(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
    buyer: ResourceAddress,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .call_method(component, "add_permissioned_buyer", manifest_args!(buyer))
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}

pub fn remove_permissioned_buyer(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
    buyer: ResourceAddress,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .call_method(
            component,
            "remove_permissioned_buyer",
            manifest_args!(buyer),
        )
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}

pub fn deny_all_buyers(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .call_method(component, "deny_all_buyers", manifest_args!())
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}

pub fn allow_all_buyers(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .call_method(component, "allow_all_buyers", manifest_args!())
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}

pub fn lock_royalty_configuration(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    component: ComponentAddress,
    creator_key: ResourceAddress,
) {
    let creator_local_id: NonFungibleLocalId =
        NonFungibleLocalId::string("creator_key".to_string()).unwrap();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(creator_key, vec![creator_local_id]),
        )
        .call_method(component, "lock_royalty_configuration", manifest_args!())
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}

pub fn transfer_royal_nft_to_component(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    trader_account: ComponentAddress,
    custom_method: String,
    dapp: ComponentAddress,
    nft_address: ResourceAddress,
    trader_key_resource: ResourceAddress,
    trader_key_local: NonFungibleLocalId,
) {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "withdraw",
            manifest_args!(nft_address, dec!(1)),
        )
        .take_all_from_worktop(nft_address, "nft")
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(trader_key_resource, indexset![trader_key_local.clone()]),
        )
        .with_name_lookup(|builder, lookup| {
            builder.call_method(
                trader_account,
                "transfer_royal_nft_to_component",
                manifest_args!(lookup.bucket("nft"), dapp, custom_method),
            )
        })
        .call_method(
            user.account,
            "deposit_batch",
            manifest_args!(ManifestExpression::EntireWorktop),
        )
        .build();

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    receipt.expect_commit(true);
}
