use super::Spoiler;
use crate::{cache::Cache, CardText, Error};
use reqwest::Url;

pub async fn new_cards<C: Cache + Send + 'static>(mut cache: C) -> Result<Vec<Spoiler>, Error> {
    todo!()
}

#[allow(dead_code)]
fn _assert() {
    fn is_send<T: Send>(_: T) {}
    is_send(new_cards(super::cache::empty::Empty));
}

pub async fn get_card_text(_url: Url) -> Result<Vec<CardText>, Error> {
    todo!()
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
