pub mod cache;
pub mod mythic;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpoilerSource {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Spoiler {
    pub name: Option<String>,
    pub image: String,
    pub source: Option<SpoilerSource>,
}
