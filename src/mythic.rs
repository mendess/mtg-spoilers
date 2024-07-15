use std::sync::OnceLock;

use super::{Spoiler, SpoilerSource};
use crate::{cache::Cache, http, CardText, Error};
use futures::StreamExt;
use reqwest::Url;
use scraper::{
    node::{Comment, Text},
    ElementRef, Html, Node, Selector,
};

fn based(s: &str) -> String {
    static BASE: &str = "http://mythicspoiler.com/";
    format!("{BASE}/{s}")
}

#[tracing::instrument(skip_all)]
pub async fn new_cards<Db: Cache + Send + 'static>(mut db: Db) -> Result<Vec<Spoiler>, Error> {
    let mut spoilers = {
        tracing::trace!("requesting page");
        let doc = request_page().await?;
        tracing::trace!("parsing document");
        let doc = Html::parse_document(&doc);
        parse_document(&doc)
            .filter(|c| db.is_new(c))
            .collect::<Vec<_>>()
    };
    tracing::trace!("persisting cache");
    db.persist().await?;
    tracing::trace!("reversing spoilers list");
    spoilers.reverse();
    tracing::trace!(count = spoilers.len(), "getting card names");
    let now = std::time::Instant::now();
    futures::stream::iter(spoilers.iter_mut())
        .for_each_concurrent(None, get_card_name)
        .await;

    tracing::trace!(elapsed = ?now.elapsed(), "done getting card names");
    Ok(spoilers)
}

#[allow(dead_code)]
fn _assert() {
    fn is_send<T: Send>(_: T) {}
    is_send(new_cards(super::cache::empty::Empty));
}

async fn request_page() -> reqwest::Result<String> {
    http()
        .get(based("newspoilers.html"))
        .send()
        .await?
        .text()
        .await
}

fn parse_document(doc: &'_ Html) -> impl Iterator<Item = Spoiler> + '_ {
    static CARD: OnceLock<Selector> = OnceLock::new();
    let card = CARD.get_or_init(|| Selector::parse("div.grid-card").unwrap());
    doc.select(card).filter_map(parse_card)
}

fn parse_card(card: ElementRef<'_>) -> Option<Spoiler> {
    static LINK: OnceLock<Selector> = OnceLock::new();
    static IMG: OnceLock<Selector> = OnceLock::new();
    static SOURCE: OnceLock<Selector> = OnceLock::new();
    static FONT: OnceLock<Selector> = OnceLock::new();
    let link = LINK.get_or_init(|| Selector::parse("a").unwrap());
    let img = IMG.get_or_init(|| Selector::parse("img").unwrap());
    let source = SOURCE.get_or_init(|| Selector::parse("center").unwrap());
    let font = FONT.get_or_init(|| Selector::parse("font").unwrap());
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
    let card_link = card.select(link).next()?;
    let card_url_in_mythic_site = card_link.value().attr("href")?;
    let img = card_link.select(img).next()?.value().attr("src")?.trim();
    let source = 'source: {
        let Some(source) = card.select(source).next() else {
            break 'source None;
        };
        let Some(source_link_element) = source.select(link).next() else {
            break 'source None;
        };
        let Some(source_link) = source_link_element.value().attr("href") else {
            break 'source None;
        };
        let Some(source_name) = card.select(font).next().and_then(|s| s.text().next()) else {
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
    let Ok(response) = http().get(&url).send().await else {
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

pub async fn get_card_text(url: Url) -> Result<Vec<CardText>, Error> {
    let text = reqwest::get(url).await?.text().await?;

    static CARD: OnceLock<Selector> = OnceLock::new();
    let doc = Html::parse_document(&text);
    let table = CARD.get_or_init(|| Selector::parse("td").unwrap());

    let mut texts = vec![CardText::default()];
    for table in doc.select(table) {
        if let Some(comment) = table.children().find_map(|e| match e.value() {
            Node::Comment(c) => Some(c),
            _ => None,
        }) {
            match comment.comment.trim() {
                "TYPE" => {
                    let Some(parsed_type_line) =
                        table.text().map(|t| t.trim()).find(|t| !t.is_empty())
                    else {
                        continue;
                    };
                    let type_line = &mut texts.last_mut().unwrap().type_line;
                    let parsed_type_line = Some(parsed_type_line.to_owned());
                    if type_line.is_none() {
                        *type_line = parsed_type_line;
                    } else {
                        texts.push(CardText {
                            type_line: parsed_type_line,
                            ..Default::default()
                        });
                    }
                }
                "CARD TEXT" => {
                    let parsed_text = {
                        let mut parsed_text = table.text().collect::<String>();
                        let trimmed_start = parsed_text
                            .char_indices()
                            .find(|(_, c)| !c.is_whitespace())
                            .map(|(i, _)| i);

                        let trimmed_end = parsed_text
                            .char_indices()
                            .filter(|(_, c)| !c.is_whitespace())
                            .last()
                            .map(|(i, _)| i);

                        match (trimmed_start, trimmed_end) {
                            (Some(start), Some(end)) => {
                                let new_length = end - start + 1;
                                parsed_text.drain(..start);
                                parsed_text.drain(new_length..);
                                Some(parsed_text.replace("\n\n\n", "\n\n").replace(" \n", "\n"))
                            }
                            _ => None,
                        }
                    };
                    let text = &mut texts.last_mut().unwrap().text;
                    if text.is_none() {
                        *text = parsed_text;
                    } else {
                        texts.push(CardText {
                            text: parsed_text,
                            ..Default::default()
                        });
                    }
                }
                _ => {}
            }
        }
    }
    if texts == [CardText::default()] {
        Ok(vec![])
    } else {
        Ok(texts)
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

    macro_rules! test_card_parser {
        ($($exp:literal / $name:expr => [$({name: $e_name:expr, type_line: $e_type:expr, text: $e_text:expr}),*$(,)?])*) => {
            $(paste::paste! {
                #[tokio::test]
                async fn [<get_ $name>]() {
                    let text = get_card_text(
                        Url::parse(::std::concat!("https://mythicspoiler.com/", $exp, "/cards/", $name, ".html")).unwrap(),
                    )
                    .await
                    .unwrap();

                    let texts: [CardText; test_card_parser!(@count $($e_name),*)] = text.try_into().unwrap();
                    texts
                        .into_iter()
                        .zip([$((Option::from($e_name), Option::from($e_type), Option::from($e_text))),*])
                        .for_each(|(got, (name, type_line, text))| {
                            assert_eq!(got.name.as_deref(), name);
                            assert_eq!(got.type_line.as_deref(), type_line);
                            assert_eq!(
                                got.text.as_deref(),
                                text
                            );
                        });
                }
            })*
        };

        (@count) => { 0 };
        (@count $_:expr$(, $tail:tt)*) => {
            1 + test_card_parser!(@count $($tail)*)
        };
    }

    test_card_parser! {
        "woe" / "gingerbreadhunter" => [
            {name: None, type_line: "Creature - Giant", text: "When Gingerbread Hunter enters the battlefield, create a Food Token."},
            {name: None, type_line: "Adventure - Instant", text: "Target creature gets -2/-2 until end of turn."},
        ]
        "woe" / "ragingfirebolt" => [
            {name: None, type_line: "Instant", text: "Raging Firebolt deals X damage to target creature, where X is 2 plus the number of instants, sorceries, and cards with adventure in your graveyard."}
        ]
        "woe" / "picklockprankster" => []
        "one" / "vraskabetrayalssting" => [
            {name: None, type_line: "Legendary Planeswalker - Vraska", text: "
Compleated ([B/P] can be paid with B, or 2 life. If life was paid, this planeswalker enters with two fewer loyalty counters.)

[0]: You draw a card and you lose 1 life.
Proliferate.

[-2]: Target creature becomes a Treasure artifact with \"T: Sacrifice this artifact: Add one mana of any color\" and oses all other card types and abilities.

[-9]: If target player has fewer than nine poison counters, they get a number of poison counters equal to the difference.
".trim()}
        ]
    }

    // #[tokio::test]
    // async fn get_gingerbread_hunter() {
    //     let text = get_card_text(
    //         Url::parse("https://mythicspoiler.com/woe/cards/gingerbreadhunter.html").unwrap(),
    //     )
    //     .await
    //     .unwrap();

    //     let [main_card, adventure]: [CardText; 2] = text.try_into().unwrap();
    //     assert_eq!(main_card.name, None);
    //     assert_eq!(main_card.type_line.as_deref(), Some(""));
    //     assert_eq!(
    //         main_card.text.as_deref(),
    //         Some("When Gingerbread Hunter enters the battlefield, create a Food Token.")
    //     );

    //     assert_eq!(adventure.name, None);
    //     assert_eq!(adventure.type_line.as_deref(), Some("Adventure - Instant"));
    //     assert_eq!(
    //         adventure.text.as_deref(),
    //         Some("Target creature gets -2/-2 until end of turn.")
    //     );
    // }
}
