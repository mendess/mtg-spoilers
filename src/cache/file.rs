use std::{
    collections::HashSet,
    io,
    path::{Path, PathBuf},
};

use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWrite, AsyncWriteExt, BufWriter},
};

use super::Cache;

pub struct File {
    set: HashSet<String>,
    path: PathBuf,
}

impl File {
    pub async fn new<P: Into<PathBuf>>(path: P) -> io::Result<Self> {
        let path = path.into();
        let set = Self::load(&path).await?;
        Ok(Self { set, path })
    }

    async fn load<P: AsRef<Path>>(p: P) -> io::Result<HashSet<String>> {
        let mut file = match fs::File::open(p).await {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Default::default()),
            Err(e) => return Err(e),
        };
        let mut buf = String::new();
        file.read_to_string(&mut buf).await?;
        Ok(buf.split_whitespace().map(ToOwned::to_owned).collect())
    }

    async fn save<W, I>(mut to: W, set: I) -> io::Result<()>
    where
        I: IntoIterator,
        I::Item: AsRef<[u8]>,
        W: AsyncWrite + Unpin,
    {
        for link in set.into_iter() {
            to.write_all(link.as_ref()).await?;
            to.write_all(b"\n").await?;
        }
        to.flush().await?;
        Ok(())
    }
}

impl Cache for File {
    fn is_new(&mut self, spoiler: &crate::Spoiler) -> bool {
        self.set.insert(spoiler.image.clone())
    }
}

impl Cache for &'_ mut File {
    fn is_new(&mut self, spoiler: &crate::Spoiler) -> bool {
        <File as Cache>::is_new(self, spoiler)
    }
}

impl Drop for File {
    fn drop(&mut self) {
        use std::mem::take;
        eprintln!("[mtg-spoilers] saving cache to file");
        let set = take(&mut self.set);
        let path = take(&mut self.path);
        tokio::spawn(async move {
            let base = path.parent().unwrap_or_else(|| Path::new("/"));
            let tmp = match tempfile::NamedTempFile::new_in(base) {
                Ok(tmp) => tmp,
                Err(e) => {
                    eprintln!("[mtg-spoilers] failed to create temporary file, writing to original file: {e:?}");
                    return fallback(set, path).await;
                }
            };
            let (tmp_file, tmp_path) = tmp.into_parts();
            let writer = BufWriter::new(fs::File::from_std(tmp_file));
            if let Err(e) = Self::save(writer, set.iter()).await {
                eprintln!("[mtg-spoilers] couldn't save to tmp file: {e:?}");
                return fallback(set, path).await;
            }
            if let Err(e) = tokio::fs::rename(tmp_path, &path).await {
                eprintln!("[mtg-spoilers] overwrite original file: {e:?}");
                fallback(set, path).await;
            }
        });

        async fn fallback(set: HashSet<String>, path: PathBuf) {
            let file = match fs::File::create(&path).await {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("[mtg-spoilers] can't open original file: {e:?}");
                    return;
                }
            };
            let writer = BufWriter::new(file);
            if let Err(e) = File::save(writer, set).await {
                eprintln!("[mtg-spoilers] can't write to original file: {e:?}");
            }
        }
    }
}
