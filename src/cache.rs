pub mod empty;
pub mod file;

use super::Spoiler;

pub trait Cache {
    fn is_new(&mut self, spoiler: &Spoiler) -> bool;
}
