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
fn list_and_purchase_multi_nft() {
    let (mut test_runner, user, package) = setup_for_test();

    let open_hub_component = instantiate_open_hub(&mut test_runner, &user, package);

    let virtual_badge = fetch_virt_badge(&mut test_runner, &user, open_hub_component.clone());

    let depositer_badger = fetch_depositer_badge(&mut test_runner, &user, open_hub_component);

    let (_trader_key_resource, _trader_key_local, trader_component) =
        create_outpost(&mut test_runner, &user, open_hub_component);

    create_event_listener(&mut test_runner, &user, package, virtual_badge.clone());

    let (marketplace_component, marketplace_key) =
        create_marketplace(&mut test_runner, &user, package, dec!(0.02));

    println!("marketplace passed");
    let mint_factory = create_mint_factory(&mut test_runner, &user, package);

    println!("mint factory passed");

    let royalty_config = blank_config();

    let (royalty_nft_component, creator_key) = create_custom_variant_nft(
        &mut test_runner,
        &user,
        mint_factory,
        royalty_config,
        depositer_badger.clone(),
        false,
    );

    for i in 0..124 {
        direct_mint(
            &mut test_runner,
            &user,
            royalty_nft_component.clone(),
            i,
            creator_key.clone(),
        );
    }

    let minting_transient = get_transient_address(&mut test_runner, &user, royalty_nft_component);

    let nft_address = nft_address(&mut test_runner, &user, royalty_nft_component);

    println!("nft address passed");

    println!("mint royalty passed");

    println!("global id passed");

    let (trader_auth_resource, trader_auth_local) =
        trader_auth_key(&mut test_runner, &user, trader_component.clone());

    println!("trader auth passed");

    let mut listings = Vec::new();
    for i in 0..=123 {
        listings.push((
            NonFungibleGlobalId::new(nft_address.clone(), NonFungibleLocalId::integer(i)),
            dec!(10),
        ));
    }

    // for i in 0..10 {
    multi_list(
        &mut test_runner,
        &user,
        listings,
        trader_component.clone(),
        trader_auth_resource.clone(),
        trader_auth_local.clone(),
        nft_address.clone(),
        NonFungibleLocalId::integer(1),
        None,
        vec![marketplace_key.clone()],
    );

    // pub fn multi_list(
    //     test_runner: &mut DefaultLedgerSimulator,
    //     user: &User,
    //     listings: Vec<(NonFungibleGlobalId, Decimal)>,
    //     trader_component: ComponentAddress,
    //     trader_key_resource: ResourceAddress,
    //     trader_key_local: NonFungibleLocalId,
    //     nft_address: ResourceAddress,
    //     nft_local_id: NonFungibleLocalId,
    //     currency: Option<ResourceAddress>,
    //     auth_buyers: Vec<ResourceAddress>,
    // ) {
    // // }

    println!("list royalty passed");

    let transient_token_address =
        get_transient_token_address(&mut test_runner, &user, trader_component.clone());

    println!("transient token passed");

    let mut orders: Vec<(ComponentAddress, NonFungibleGlobalId, Decimal)> = vec![];

    for i in 0..123 {
        orders.push((
            trader_component.clone(),
            create_global_id(nft_address.clone(), i),
            dec!(10),
        ));
    }

    // orders.push((
    //     trader_component.clone(),
    //     create_global_id(nft_address.clone(), 1),
    //     dec!(10),
    // ));

    let amount = dec!(1230);

    purchase_multi_listing(
        &mut test_runner,
        &user,
        marketplace_component,
        orders,
        amount,
        false,
    );
}
