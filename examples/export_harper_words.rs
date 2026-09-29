//! Writes Harper's curated dictionary to `dictionaries/harper/words.tsv.zlib` (see
//! `explicit::rules::words`). Run through `scripts/update-harper-words.sh`.

fn main() -> std::io::Result<()> {
    let mut args = std::env::args().skip(1);
    let version = args
        .next()
        .expect("usage: export_harper_words <harper-core version> [out]");
    let out = args
        .next()
        .unwrap_or_else(|| "dictionaries/harper/words.tsv.zlib".to_string());
    let tsv = explicit::rules::words::export(&version);
    let packed = miniz_oxide::deflate::compress_to_vec_zlib(tsv.as_bytes(), 10);
    std::fs::write(&out, &packed)?;
    eprintln!(
        "wrote {out}: {} bytes ({} uncompressed)",
        packed.len(),
        tsv.len()
    );
    Ok(())
}
