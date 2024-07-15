pub mod empty;
pub mod file;

use std::{future::Future, io};

use super::Spoiler;

pub trait Cache {
    fn is_new(&mut self, spoiler: &Spoiler) -> bool;

    fn persist(self) -> impl Future<Output = io::Result<()>> + Send
    where
        Self: Sized,
    {
        async { Ok(()) }
    }
}
