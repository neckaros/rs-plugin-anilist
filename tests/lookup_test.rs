use extism::*;
use rs_plugin_common_interfaces::lookup::{
    RsLookupQuery, RsLookupSerie, RsLookupWrapper,
};

fn build_plugin() -> Plugin {
    let wasm = Wasm::file("target/wasm32-unknown-unknown/release/rs_plugin_anilist.wasm");
    let manifest = Manifest::new([wasm]).with_allowed_host("graphql.anilist.co");
    Plugin::new(&manifest, [], true).expect("Failed to create plugin")
}

#[test]
fn test_lookup_one_piece() {
    let mut plugin = build_plugin();

    let input = RsLookupWrapper {
        query: RsLookupQuery::Serie(RsLookupSerie {
            name: "One piece".to_string(),
            ids: None,
        }),
        credential: None,
        params: None,
    };

    let input_str = serde_json::to_string(&input).unwrap();

    let output = plugin
        .call::<&str, &[u8]>("lookup_metadata", &input_str)
        .expect("lookup_metadata call failed");

    let results: serde_json::Value =
        serde_json::from_slice(output).expect("Failed to parse output JSON");

    let results_array = results.as_array().expect("Expected an array of results");
    assert!(
        !results_array.is_empty(),
        "Expected at least one result for 'One piece'"
    );

    println!(
        "\n=== One Piece lookup results ({} found) ===",
        results_array.len()
    );
    for (i, result) in results_array.iter().enumerate() {
        println!("\n--- Result {} ---", i + 1);
        println!("{}", serde_json::to_string_pretty(result).unwrap());
    }
}
