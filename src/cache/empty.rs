use crate::Spoiler;

#[derive(Debug)]
pub struct Empty;

impl super::Cache for Empty {
    fn is_new(&mut self, _: &Spoiler) -> bool {
        true
    }
}
