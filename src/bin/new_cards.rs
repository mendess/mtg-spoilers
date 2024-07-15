use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().pretty())
        .with(EnvFilter::from_default_env())
        .init();
    // let cache = mtg_spoilers::cache::file::File::new("/tmp/new-cards-cache")
    //     .await
    //     .unwrap();
    let cache = mtg_spoilers::cache::empty::Empty;
    let new_cards = match std::env::args().nth(1).as_deref() {
        Some("magic-spoiler") => mtg_spoilers::magic_spoiler::new_cards(cache).await?,
        Some("mythic") | None => mtg_spoilers::mythic::new_cards(cache).await?,
        Some(s) => return Err(format!("invalid source: {s:?}").into()),
    };

    new_cards
        .iter()
        .skip(new_cards.len().saturating_sub(10))
        .for_each(|p| println!("{p:?}"));
    Ok(())
}
