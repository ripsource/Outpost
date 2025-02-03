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
fn list_and_purchase_multi_nft_across_multi_accounts() {
    let (mut test_runner, user, package) = setup_for_test();

    // Create multiple users using a vector
    let mut users = vec![user.clone()];
    for i in 2..=20 {
        users.push(make_user(&mut test_runner, Some(&format!("user{}", i))));
    }

    let open_hub_component = instantiate_open_hub(&mut test_runner, &user, package);
    let virtual_badge = fetch_virt_badge(&mut test_runner, &user, open_hub_component.clone());
    let depositer_badger = fetch_depositer_badge(&mut test_runner, &user, open_hub_component);

    // Create outposts for all users
    let mut trader_components = Vec::new();
    let mut trader_auth_data = Vec::new();

    for user in users.iter() {
        let (_trader_key_resource, _trader_key_local, trader_component) =
            create_outpost(&mut test_runner, user, open_hub_component.clone());

        let (trader_auth_resource, trader_auth_local) =
            trader_auth_key(&mut test_runner, user, trader_component.clone());

        trader_components.push(trader_component);
        trader_auth_data.push((trader_auth_resource, trader_auth_local));
    }

    create_event_listener(&mut test_runner, &user, package, virtual_badge.clone());

    let (marketplace_component, marketplace_key) =
        create_marketplace(&mut test_runner, &user, package, dec!(0.02));

    let mint_factory = create_mint_factory(&mut test_runner, &user, package);
    let royalty_config = blank_config();

    let (royalty_nft_component, creator_key) = create_custom_variant_nft(
        &mut test_runner,
        &user,
        mint_factory,
        royalty_config,
        depositer_badger.clone(),
        false,
    );

    // Mint 70 NFTs
    for i in 0..70 {
        direct_mint(
            &mut test_runner,
            &user,
            royalty_nft_component.clone(),
            i,
            creator_key.clone(),
        );
    }

    let nft_address = nft_address(&mut test_runner, &user, royalty_nft_component);

    // List all 70 NFTs from the first user
    for i in 0..70 {
        list(
            &mut test_runner,
            &users[0],
            trader_components[0].clone(),
            trader_auth_data[0].0.clone(),
            trader_auth_data[0].1.clone(),
            nft_address.clone(),
            NonFungibleLocalId::integer(i),
            dec!(100),
            None,
            vec![marketplace_key.clone()],
        );
    }

    // Distribute 1 NFT to each of the 20 users
    for (i, user) in users.iter().enumerate() {
        let orders = vec![(
            trader_components[0].clone(),
            create_global_id(nft_address.clone(), i as u64),
            dec!(100),
        )];

        purchase_multi_listing(
            &mut test_runner,
            user,
            marketplace_component,
            orders,
            dec!(100),
            false,
        );
    }

    let order = vec![(
        trader_components[0].clone(),
        create_global_id(nft_address.clone(), 30),
        dec!(100),
    )];

    purchase_multi_listing(
        &mut test_runner,
        &users[2],
        marketplace_component,
        order,
        dec!(100),
        false,
    );

    // Each user lists their NFT
    for (i, user) in users.iter().enumerate() {
        list(
            &mut test_runner,
            user,
            trader_components[i].clone(),
            trader_auth_data[i].0.clone(),
            trader_auth_data[i].1.clone(),
            nft_address.clone(),
            NonFungibleLocalId::integer(i as u64),
            dec!(100),
            None,
            vec![marketplace_key.clone()],
        );
    }

    list(
        &mut test_runner,
        &users[2],
        trader_components[2].clone(),
        trader_auth_data[2].0.clone(),
        trader_auth_data[2].1.clone(),
        nft_address.clone(),
        NonFungibleLocalId::integer(30 as u64),
        dec!(100),
        None,
        vec![marketplace_key.clone()],
    );

    print!("listings passed");

    // Create orders for final multi-buy
    let mut final_orders = Vec::new();
    for i in 0..6 {
        let number: u64 = i as u64;
        final_orders.push((
            trader_components[i].clone(),
            create_global_id(nft_address.clone(), number),
            dec!(100),
        ));
    }

    // final_orders.push((
    //     trader_components[2].clone(),
    //     create_global_id(nft_address.clone(), 30),
    //     dec!(100),
    // ));

    // Final multi-buy of all NFTs
    purchase_multi_listing(
        &mut test_runner,
        &users[0], // First user buys all
        marketplace_component,
        final_orders,
        dec!(600), // Adjust amount based on total cost
        true,
    );
}
