pub mod empty;
pub mod file;

use std::io;

use super::Spoiler;

#[async_trait::async_trait]
pub trait Cache {
    fn is_new(&mut self, spoiler: &Spoiler) -> bool;
    async fn persist(self) -> io::Result<()>
    where
        Self: Sized,
    {
        Ok(())
    }
}
