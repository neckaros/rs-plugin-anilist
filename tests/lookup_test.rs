use extism::*;
use rs_plugin_common_interfaces::{
    domain::rs_ids::RsIds,
    lookup::{RsLookupQuery, RsLookupSerie, RsLookupWrapper},
};

fn build_plugin() -> Plugin {
    let wasm = Wasm::file("target/wasm32-unknown-unknown/release/rs_plugin_anilist.wasm");
    let manifest = Manifest::new([wasm]).with_allowed_host("graphql.anilist.co");
    Plugin::new(&manifest, [], true).expect("Failed to create plugin")
}

fn call_lookup(plugin: &mut Plugin, input: &RsLookupWrapper) -> serde_json::Value {
    let input_str = serde_json::to_string(input).unwrap();
    let output = plugin
        .call::<&str, &[u8]>("lookup_metadata", &input_str)
        .expect("lookup_metadata call failed");
    serde_json::from_slice(output).expect("Failed to parse output JSON")
}

fn call_lookup_images(plugin: &mut Plugin, input: &RsLookupWrapper) -> serde_json::Value {
    let input_str = serde_json::to_string(input).unwrap();
    let output = plugin
        .call::<&str, &[u8]>("lookup_metadata_images", &input_str)
        .expect("lookup_metadata_images call failed");
    serde_json::from_slice(output).expect("Failed to parse output JSON")
}

#[test]
fn test_lookup_one_piece_by_name() {
    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Serie(RsLookupSerie {
            name: Some("One piece".to_string()),
            ids: None,
        }),
        credential: None,
        params: None,
    };

    let results = call_lookup(&mut plugin, &input);
    let results_array = results.as_array().expect("Expected an array");
    assert!(
        !results_array.is_empty(),
        "Expected at least one result for 'One piece'"
    );

    println!(
        "\n=== One Piece search results ({} found) ===",
        results_array.len()
    );
    for (i, result) in results_array.iter().take(1).enumerate() {
        println!("\n--- Result {} ---", i + 1);
        println!("{}", serde_json::to_string_pretty(result).unwrap());
    }
}

#[test]
fn test_lookup_one_piece_by_anilist_id() {
    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Serie(RsLookupSerie {
            name: None,
            ids: Some({
                let mut ids = RsIds::default();
                ids.set("anilist", 74347); // One Piece
                ids
            }),
        }),
        credential: None,
        params: None,
    };

    let results = call_lookup(&mut plugin, &input);
    let results_array = results.as_array().expect("Expected an array");
    assert_eq!(
        results_array.len(),
        1,
        "Expected exactly one result when fetching by ID"
    );

    let serie = &results_array[0]["metadata"]["serie"];
    assert_eq!(serie["id"], "anilist:74347");

    println!(
        "\n=== One Piece by ID result ===\n{}",
        serde_json::to_string_pretty(&results_array[0]).unwrap()
    );
}

#[test]
fn test_lookup_empty_name_returns_404() {
    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Serie(RsLookupSerie {
            name: Some("".to_string()),
            ids: None,
        }),
        credential: None,
        params: None,
    };

    let input_str = serde_json::to_string(&input).unwrap();
    let error = plugin
        .call::<&str, &[u8]>("lookup_metadata", &input_str)
        .expect_err("Expected 404 error for empty search");
    let message = error.to_string();
    assert!(
        message.contains("Not supported") || message.contains("404"),
        "Expected error message to mention 404/Not supported, got: {message}"
    );
}

#[test]
fn test_lookup_images_by_anilist_id() {
    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Serie(RsLookupSerie {
            name: None,
            ids: Some({
                let mut ids = RsIds::default();
                ids.set("anilist", 74347); // One Piece
                ids
            }),
        }),
        credential: None,
        params: None,
    };

    let images = call_lookup_images(&mut plugin, &input);
    let images_array = images.as_array().expect("Expected an array");
    assert!(
        !images_array.is_empty(),
        "Expected at least one image when fetching by ID"
    );
    println!(
        "\n=== One Piece images by ID result ({} images) ===\n{}",
        images_array.len(),
        serde_json::to_string_pretty(&images_array).unwrap()
    );
}
