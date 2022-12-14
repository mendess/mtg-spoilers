use std::io;

use super::{Spoiler, SpoilerSource};
use crate::cache::Cache;
use futures::{stream::FuturesUnordered, StreamExt};
use lazy_static::lazy_static;
use scraper::{
    node::{Comment, Text},
    ElementRef, Html, Node, Selector,
};

lazy_static! {
    static ref CLIENT: reqwest::Client = reqwest::Client::new();
}

fn based(s: &str) -> String {
    static BASE: &str = "http://mythicspoiler.com/";
    format!("{BASE}/{s}")
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Reqwest({0})")]
    Reqwest(#[from] reqwest::Error),
    #[error("Io({0})")]
    Io(#[from] io::Error),
}

pub async fn new_cards<Db: Cache + Send + 'static>(mut db: Db) -> Result<Vec<Spoiler>, Error> {
    let mut spoilers = {
        let doc = request_page().await?;
        let doc = Html::parse_document(&doc);
        parse_document(&doc)
            .filter(|c| db.is_new(c))
            .collect::<Vec<_>>()
    };
    db.persist().await?;
    spoilers.reverse();
    FuturesUnordered::from_iter(spoilers.iter_mut().map(get_card_name))
        .for_each(|_| async {})
        .await;
    Ok(spoilers)
}

#[allow(dead_code)]
fn _assert() {
    fn is_send<T: Send>(_: T) {}
    is_send(new_cards(super::cache::empty::Empty));
}

async fn request_page() -> reqwest::Result<String> {
    CLIENT
        .get(based("newspoilers.html"))
        .send()
        .await?
        .text()
        .await
}

fn parse_document(doc: &'_ Html) -> impl Iterator<Item = Spoiler> + '_ {
    lazy_static! {
        static ref CARD: Selector = Selector::parse("div.grid-card").unwrap();
    };
    doc.select(&CARD).filter_map(parse_card)
}

fn parse_card(card: ElementRef<'_>) -> Option<Spoiler> {
    lazy_static! {
        static ref LINK: Selector = Selector::parse("a").unwrap();
        static ref IMG: Selector = Selector::parse("img").unwrap();
        static ref SOURCE: Selector = Selector::parse("center").unwrap();
        static ref FONT: Selector = Selector::parse("font").unwrap();
    };
    /*
     * <div class="grid-card">
     *  <a href="brw/cards/jalumtome.html">
     *      <img class="" src="brw/cards/jalumtome.jpg">
     *  </a>
     *  <!--URL BELOW-->
     *  <a href="twitch.tv/magic"></a>
     *  <center>
     *      <a href="twitch.tv/magic">
     *          <font face="'Arial Black', Gadget, sans-serif" color="#555555" size="-4">
     *              WeeklyMTG
     *          </font>
     *      </a>
     *  </center>
     * </div>
     */
    let card_link = card.select(&LINK).next()?;
    let card_url_in_mythic_site = card_link.value().attr("href")?;
    let img = card_link.select(&IMG).next()?.value().attr("src")?.trim();
    let source = 'source: {
        let Some(source) = card.select(&SOURCE).next() else {
            break 'source None;
        };
        let Some(source_link_element) = source.select(&LINK).next() else {
            break 'source None;
        };
        let Some(source_link) = source_link_element.value().attr("href") else {
            break 'source None;
        };
        let Some(source_name) = card.select(&FONT).next().and_then(|s| s.text().next()) else {
            break 'source None;
        };

        Some(SpoilerSource {
            name: source_name.trim().to_string(),
            url: {
                let source_link = source_link.trim();
                if source_link.is_empty() {
                    None
                } else if !source_link.starts_with("http") {
                    Some(format!("http://{source_link}"))
                } else {
                    Some(source_link.to_string())
                }
            },
        })
    };

    Some(Spoiler {
        image: based(img.trim()),
        source_site_url: based(card_url_in_mythic_site.trim()),
        name: None,
        source,
    })
}

async fn get_card_name(spoiler: &mut Spoiler) {
    let mut url = String::with_capacity(spoiler.image.len() + 1);
    url.push_str(
        spoiler
            .image
            .trim_end_matches("jpg")
            .trim_end_matches("png"),
    );
    url.push_str("html");
    let Ok(response) = CLIENT.get(&url).send().await else {
        return;
    };
    let Ok(doc) = response.text().await else {
        return;
    };
    let doc = Html::parse_document(&doc);
    for f in doc.select(&Selector::parse("font").unwrap()) {
        if f.children()
            .find_map(|n| as_comment(n.value()))
            .filter(|c| c.contains("CARD NAME"))
            .is_some()
        {
            if let Some(name) = f
                .children()
                .filter_map(|nr| as_text(nr.value()))
                .map(|s| s.trim())
                .find(|s| !s.is_empty())
            {
                spoiler.name = Some(name.to_string());
                return;
            }
        }
    }

    fn as_text(n: &Node) -> Option<&Text> {
        match n {
            Node::Text(t) => Some(t),
            _ => None,
        }
    }
    fn as_comment(n: &Node) -> Option<&Comment> {
        match n {
            Node::Comment(c) => Some(c),
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cache::empty::Empty;

    #[tokio::test]
    async fn foo() {
        let cards = new_cards(Empty).await.unwrap();
        assert_ne!(cards.len(), 0);
    }
}
