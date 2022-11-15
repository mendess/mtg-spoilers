pub mod cache;
pub mod mythic;

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
