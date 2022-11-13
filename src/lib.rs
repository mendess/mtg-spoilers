pub mod cache;
pub mod mythic;

#[derive(Debug)]
pub struct SpoilerSource {
    pub name: String,
    pub url: String,
}

#[derive(Debug)]
pub struct Spoiler {
    pub name: Option<String>,
    pub image: String,
    pub source: Option<SpoilerSource>,
}

