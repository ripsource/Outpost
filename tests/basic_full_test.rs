use scrypto::runtime::Clock;
use scrypto_test::prelude::*;
mod common;
mod creator_manifests;
mod marketplace_manifests;
mod misc_manifests;
mod scenario_manifests;
mod trader_manifests;
use common::*;
use creator_manifests::*;
use marketplace_manifests::*;
use misc_manifests::*;
use scenario_manifests::*;
use trader_manifests::*;

#[test]
fn list_and_purchase_royalty_nft() {
    let (mut test_runner, user, package) = setup_for_test();

    let open_hub_component = instantiate_open_hub(&mut test_runner, &user, package);

    let virtual_badge = fetch_virt_badge(&mut test_runner, &user, open_hub_component.clone());

    let depositer_badger = fetch_depositer_badge(&mut test_runner, &user, open_hub_component);

    let (trader_key_resource, trader_key_local, trader_component) =
        create_outpost(&mut test_runner, &user, open_hub_component);

    create_event_listener(&mut test_runner, &user, package, virtual_badge.clone());

    let (marketplace_component, marketplace_key) =
        create_marketplace(&mut test_runner, &user, package, dec!(0.02));

    println!("marketplace passed");
    let mint_factory = create_mint_factory(&mut test_runner, &user, package);

    println!("mint factory passed");

    let royalty_config = defaults_royalty_config(depositer_badger);

    let (royalty_nft_component, creator_key) =
        create_royalty_nft(&mut test_runner, &user, mint_factory, royalty_config);

    println!("royalty nft passed");

    let time: Instant = Instant {
        seconds_since_unix_epoch: 0,
    };

    enable_mint_reveal(
        &mut test_runner,
        &user,
        royalty_nft_component,
        creator_key,
        marketplace_key,
        dec!(100.2389272937),
        1000u64,
        time,
    );
    println!("enable mint reveal passed");

    let minting_transient = get_transient_address(&mut test_runner, &user, royalty_nft_component);

    let nft_address = nft_address(&mut test_runner, &user, royalty_nft_component);

    println!("nft address passed");

    purchase_preview_mint_via_marketplace(
        &mut test_runner,
        &user,
        marketplace_component,
        nft_address.clone(),
        dec!(100.2389272937),
        3u64,
        minting_transient,
        royalty_nft_component,
    );

    println!("mint royalty passed");

    let global_id = create_global_id(nft_address.clone(), 0);

    list_royalty_nft(
        &mut test_runner,
        &user,
        trader_component.clone(),
        trader_key_resource.clone(),
        trader_key_local.clone(),
        nft_address.clone(),
        NonFungibleLocalId::integer(0),
        dec!(100),
        None,
        vec![marketplace_key.clone()],
    );

    println!("list royalty passed");

    let transient_token_address =
        get_transient_token_address(&mut test_runner, &user, trader_component.clone());

    println!("transient token passed");

    purchase_royalty_nft(
        &mut test_runner,
        &user,
        marketplace_component,
        trader_component,
        global_id,
        dec!(100),
        None,
        transient_token_address,
    )
}
