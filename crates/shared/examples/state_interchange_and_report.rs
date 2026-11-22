//! Demonstrates JSON, CSV, compressed BSON, shareable URL hash, SVG, and printable HTML5 export.

use shared::{
    AppState, Item, ParametricPreviewConfig, Priority, decode_shareable_url_hash,
    encode_shareable_url_hash, export_to_compressed_bson, export_to_csv, export_to_json,
    export_to_printable_html, export_to_svg, import_from_compressed_bson, import_from_csv,
    import_from_json,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = AppState::new();
    state.collection.add(Item::new(
        "example-5",
        "Publish Offline Report",
        "Verify SVG chart and printable HTML5 generation",
        Priority::High,
    ));

    let json = export_to_json(&state)?;
    let from_json = import_from_json(&json)?;
    assert_eq!(state, from_json);

    let csv = export_to_csv(&state.collection)?;
    let from_csv = import_from_csv(&csv)?;
    assert_eq!(state.collection.total_count(), from_csv.total_count());

    let bson = export_to_compressed_bson(&state)?;
    let from_bson = import_from_compressed_bson(&bson)?;
    assert_eq!(state, from_bson);

    let url_hash = encode_shareable_url_hash(&state)?;
    let from_hash = decode_shareable_url_hash(&url_hash)?;
    assert_eq!(state, from_hash);

    let svg = export_to_svg(&state);
    let html = export_to_printable_html(&state);
    let samples = ParametricPreviewConfig::default().sample_points(32);

    println!(
        "Verified interchange: {} items | share hash len={} | svg={}B | html={}B | curve samples={}",
        state.collection.total_count(),
        url_hash.len(),
        svg.len(),
        html.len(),
        samples.len()
    );
    Ok(())
}
