use scrypto::{data::manifest, prelude::*};
use scrypto_test::{prelude::*, utils::dump_manifest_to_file_system};

use crate::common::*;

pub fn trader_auth_key(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    trader_component: ComponentAddress,
) -> (ResourceAddress, NonFungibleLocalId) {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(trader_component, "fetch_auth_key", manifest_args!())
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

    let (trader_auth_resource, trader_auth_local): (ResourceAddress, NonFungibleLocalId) =
        receipt.expect_commit(true).output(1);

    (trader_auth_resource, trader_auth_local)
}

pub fn list_royalty_nft(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    trader_component: ComponentAddress,
    trader_key_resource: ResourceAddress,
    trader_key_local: NonFungibleLocalId,
    nft_address: ResourceAddress,
    nft_local_id: NonFungibleLocalId,
    price: Decimal,
    currency: Option<ResourceAddress>,
    auth_buyers: Vec<ResourceAddress>,
) {
    let sell_currency: ResourceAddress;

    if currency.is_some() {
        sell_currency = currency.unwrap();
    } else {
        sell_currency = XRD;
    }

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(trader_key_resource, indexset![trader_key_local.clone()]),
        )
        .call_method(
            user.account,
            "withdraw_non_fungibles",
            manifest_args!(nft_address, indexset![nft_local_id.clone()]),
        )
        .take_all_from_worktop(nft_address, "listing")
        .with_name_lookup(|builder, lookup| {
            builder.call_method(
                trader_component,
                "royal_list",
                manifest_args!(lookup.bucket("listing"), price, sell_currency, auth_buyers,),
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

    if !receipt.is_commit_success() {
        println!("{:?}", receipt);
        panic!("TRANSACTION FAIL");
    }
}

pub fn get_transient_token_address(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    trader_component: ComponentAddress,
) -> ResourceAddress {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            trader_component,
            "transient_token_address",
            manifest_args!(),
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

    let resource: ResourceAddress = receipt.expect_commit(true).output(1);

    resource
}

pub fn purchase_multi_listing(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    marketplace_component: ComponentAddress,
    orders: Vec<(ComponentAddress, NonFungibleGlobalId, Decimal)>,
    amount: Decimal,
    cost_receipt: bool,
) {
    // let nft_ids: IndexSet<NonFungibleLocalId> = orders
    //     .iter()
    //     .map(|(_, nfgid, _)| nfgid.clone().into_parts().1.into())
    //     .collect();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(user.account, "withdraw", manifest_args!(XRD, amount))
        .take_all_from_worktop(XRD, "payment")
        .call_method_with_name_lookup(marketplace_component, "multi_listing_purchase", |lookup| {
            manifest_args!(orders, lookup.bucket("payment"))
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
    };

    if cost_receipt {
        println!(
            "{:?}",
            format_cost_breakdown(&receipt.fee_summary, receipt.fee_details.as_ref().unwrap())
        );
    }

    // println!("hit")
    // println!("{:?}", receipt);
    // println!("{:?}", receipt.costing_parameters)
}

pub fn purchase_multi_royalty_nft(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    orders: Vec<(ComponentAddress, NonFungibleGlobalId, Decimal)>,
    amount: Decimal,
    marketplace_component: ComponentAddress,
    trader_component: ComponentAddress,
    // nfgids: Vec<NonFungibleGlobalId>,
    currency: Option<ResourceAddress>,
    transient_token_address: ResourceAddress,
) {
    let buy_currency: ResourceAddress;

    if currency.is_some() {
        buy_currency = currency.unwrap();
    } else {
        buy_currency = XRD;
    }

    let (resource, _local) = orders[0].1.clone().into_parts();

    let index_set_local_ids: IndexSet<NonFungibleLocalId> = orders
        .iter()
        .map(|nfgid| nfgid.1.clone().into_parts().1)
        .collect();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "withdraw",
            manifest_args!(buy_currency, amount),
        )
        .take_all_from_worktop(buy_currency, "payment")
        .with_name_lookup(|builder, lookup| {
            builder.call_method(
                marketplace_component,
                "purchase_multi_royal_listing",
                manifest_args!(orders, lookup.bucket("payment"), user.account),
            )
        })
        .take_non_fungibles_from_worktop(resource, index_set_local_ids, "bucket1")
        .take_from_worktop(transient_token_address, dec!(1), "bucket2")
        .call_method_with_name_lookup(user.account, "deposit", |lookup| {
            manifest_args!(lookup.bucket("bucket1"))
        })
        .call_method_with_name_lookup(trader_component, "multi_cleared", |lookup| {
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

pub fn purchase_royalty_nft(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    marketplace_component: ComponentAddress,
    trader_component: ComponentAddress,
    nfgid: NonFungibleGlobalId,
    payment: Decimal,
    currency: Option<ResourceAddress>,
    transient_token_address: ResourceAddress,
) {
    let buy_currency: ResourceAddress;

    if currency.is_some() {
        buy_currency = currency.unwrap();
    } else {
        buy_currency = XRD;
    }

    let (resource, local) = nfgid.clone().into_parts();

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "withdraw",
            manifest_args!(buy_currency, payment),
        )
        .take_all_from_worktop(buy_currency, "payment")
        .with_name_lookup(|builder, lookup| {
            builder.call_method(
                marketplace_component,
                "purchase_royal_listing",
                manifest_args!(
                    nfgid,
                    lookup.bucket("payment"),
                    trader_component,
                    user.account,
                ),
            )
        })
        .take_non_fungibles_from_worktop(resource, indexset!(local), "bucket1")
        .take_from_worktop(transient_token_address, dec!(1), "bucket2")
        .call_method_with_name_lookup(user.account, "deposit", |lookup| {
            manifest_args!(lookup.bucket("bucket1"))
        })
        .call_method_with_name_lookup(trader_component, "cleared", |lookup| {
            manifest_args!(lookup.bucket("bucket2"))
        })
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

pub fn cancel_royal_listing(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    trader_component: ComponentAddress,
    trader_key_resource: ResourceAddress,
    trader_key_local: NonFungibleLocalId,
    nfgid: NonFungibleGlobalId,
) {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(trader_key_resource, indexset![trader_key_local.clone()]),
        )
        .call_method(
            trader_component,
            "cancel_royal_listing",
            manifest_args!(nfgid),
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

pub fn same_owner_royal_transfer(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    trader_component: ComponentAddress,
    nft_address: ResourceAddress,
    nft_local_id: NonFungibleLocalId,
    other_account: ComponentAddress,
) {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "withdraw_non_fungibles",
            manifest_args!(nft_address, indexset![nft_local_id.clone()]),
        )
        .take_all_from_worktop(nft_address, "transfer")
        .with_name_lookup(|builder, lookup| {
            builder.call_method(
                trader_component,
                "royal_transfer",
                manifest_args!(lookup.bucket("transfer"), other_account),
            )
        })
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

pub fn transfer_royal_nft_to_component(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    trader_component: ComponentAddress,
    nft_address: ResourceAddress,
    nft_local_id: NonFungibleLocalId,
    other_component: ComponentAddress,
    custom_method: String,
) {
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "withdraw_non_fungibles",
            manifest_args!(nft_address, indexset![nft_local_id.clone()]),
        )
        .take_all_from_worktop(nft_address, "transfer")
        .with_name_lookup(|builder, lookup| {
            builder.call_method(
                trader_component,
                "transfer_royal_nft_to_component",
                manifest_args!(lookup.bucket("transfer"), other_component, custom_method),
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

    if !receipt.is_commit_success() {
        println!("{:?}", receipt);
        panic!("TRANSACTION FAIL");
    }
}

pub fn list(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    trader_component: ComponentAddress,
    trader_key_resource: ResourceAddress,
    trader_key_local: NonFungibleLocalId,
    nft_address: ResourceAddress,
    nft_local_id: NonFungibleLocalId,
    price: Decimal,
    currency: Option<ResourceAddress>,
    auth_buyers: Vec<ResourceAddress>,
) {
    let sell_currency: ResourceAddress;

    if currency.is_some() {
        sell_currency = currency.unwrap();
    } else {
        sell_currency = XRD;
    }

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(trader_key_resource, indexset![trader_key_local.clone()]),
        )
        .call_method(
            user.account,
            "withdraw_non_fungibles",
            manifest_args!(nft_address, indexset![nft_local_id.clone()]),
        )
        .take_all_from_worktop(nft_address, "listing")
        .with_name_lookup(|builder, lookup| {
            builder.call_method(
                trader_component,
                "list",
                manifest_args!(lookup.bucket("listing"), sell_currency, price, auth_buyers,),
            )
        })
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

// pub fn multi_list(
//     &mut self,
//     listings: Vec<(NonFungibleGlobalId, Decimal)>,
//     currency: ResourceAddress,
//     permissions: Vec<ResourceAddress>,
//     mut items: NonFungibleBucket,
// ) {

pub fn multi_list(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    listings: Vec<(NonFungibleGlobalId, Decimal)>,
    trader_component: ComponentAddress,
    trader_key_resource: ResourceAddress,
    trader_key_local: NonFungibleLocalId,
    nft_address: ResourceAddress,
    nft_local_id: NonFungibleLocalId,
    currency: Option<ResourceAddress>,
    auth_buyers: Vec<ResourceAddress>,
) {
    let mut indexSetNfts: IndexSet<NonFungibleLocalId> = index_set_new();

    for (nonfungibleid, price) in listings.clone() {
        let (_, local_id) = nonfungibleid.into_parts();

        indexSetNfts.insert(local_id);
    }

    let sell_currency: ResourceAddress;

    if currency.is_some() {
        sell_currency = currency.unwrap();
    } else {
        sell_currency = XRD;
    }

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(trader_key_resource, indexset![trader_key_local.clone()]),
        )
        .call_method(
            user.account,
            "withdraw_non_fungibles",
            manifest_args!(nft_address, indexSetNfts),
        )
        .take_all_from_worktop(nft_address, "listing")
        .with_name_lookup(|builder, lookup| {
            builder.call_method(
                trader_component,
                "multi_list",
                manifest_args!(
                    listings,
                    sell_currency,
                    auth_buyers,
                    lookup.bucket("listing")
                ),
            )
        })
        .build();

    dump_manifest_to_file_system(
        &manifest,
        "./tests/manifests",
        Some("multi_list"),
        &NetworkDefinition::simulator(),
    );

    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&user.pubkey)],
    );

    if !receipt.is_commit_success() {
        println!("{:?}", receipt);
        panic!("TRANSACTION FAIL");
    }
}

pub fn royal_multi_list(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    listings: Vec<(NonFungibleGlobalId, Decimal)>,
    trader_component: ComponentAddress,
    trader_key_resource: ResourceAddress,
    trader_key_local: NonFungibleLocalId,
    nft_address: ResourceAddress,
    nft_local_id: NonFungibleLocalId,
    currency: Option<ResourceAddress>,
    auth_buyers: Vec<ResourceAddress>,
) {
    let mut indexSetNfts: IndexSet<NonFungibleLocalId> = index_set_new();

    for (nonfungibleid, price) in listings.clone() {
        let (_, local_id) = nonfungibleid.into_parts();

        indexSetNfts.insert(local_id);
    }

    let sell_currency: ResourceAddress;

    if currency.is_some() {
        sell_currency = currency.unwrap();
    } else {
        sell_currency = XRD;
    }

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "create_proof_of_non_fungibles",
            manifest_args!(trader_key_resource, indexset![trader_key_local.clone()]),
        )
        .call_method(
            user.account,
            "withdraw_non_fungibles",
            manifest_args!(nft_address, indexSetNfts),
        )
        .take_all_from_worktop(nft_address, "listing")
        .with_name_lookup(|builder, lookup| {
            builder.call_method(
                trader_component,
                "royal_multi_list",
                manifest_args!(
                    listings,
                    sell_currency,
                    auth_buyers,
                    lookup.bucket("listing")
                ),
            )
        })
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

pub fn purchase_listing(
    test_runner: &mut DefaultLedgerSimulator,
    user: &User,
    marketplace_component: ComponentAddress,
    trader_component: ComponentAddress,
    nfgid: NonFungibleGlobalId,
    payment: Decimal,
    currency: Option<ResourceAddress>,
) {
    let buy_currency: ResourceAddress;

    if currency.is_some() {
        buy_currency = currency.unwrap();
    } else {
        buy_currency = XRD;
    }

    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            user.account,
            "withdraw",
            manifest_args!(buy_currency, payment),
        )
        .take_all_from_worktop(buy_currency, "payment")
        .with_name_lookup(|builder, lookup| {
            builder.call_method(
                marketplace_component,
                "purchase_listing",
                manifest_args!(
                    nfgid,
                    lookup.bucket("payment"),
                    trader_component,
                    user.account,
                ),
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

    if !receipt.is_commit_success() {
        println!("{:?}", receipt);
        panic!("TRANSACTION FAIL");
    }
}
