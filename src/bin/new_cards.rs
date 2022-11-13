#[tokio::main]
async fn main() {
    let cache = mtg_spoilers::cache::file::File::new("/tmp/cenas")
        .await
        .unwrap();
    mtg_spoilers::mythic::new_cards(cache)
        .await
        .unwrap()
        .into_iter()
        .take(10)
        .for_each(|p| println!("{p:?}"));
}
