use std::{io, sync::OnceLock};

pub mod cache;
pub mod magic_spoiler;
pub mod mythic;

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
fn http() -> &'static reqwest::Client {
    CLIENT.get_or_init(Default::default)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpoilerSource {
    pub name: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Spoiler {
    pub name: Option<String>,
    pub source_site_url: String,
    pub image: String,
    pub source: Option<SpoilerSource>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct CardText {
    pub name: Option<String>,
    pub type_line: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Reqwest({0})")]
    Reqwest(#[from] reqwest::Error),
    #[error("Io({0})")]
    Io(#[from] io::Error),
}
