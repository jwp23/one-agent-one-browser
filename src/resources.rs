use crate::net;
use crate::url::Url;
use std::path::{Path, PathBuf};

pub trait ResourceLoader {
    fn load_bytes(&self, reference: &str) -> Result<Option<Vec<u8>>, String>;
}

pub struct NoResources;

impl ResourceLoader for NoResources {
    fn load_bytes(&self, _reference: &str) -> Result<Option<Vec<u8>>, String> {
        Ok(None)
    }
}

#[derive(Clone, Debug)]
pub enum ResourceBase {
    Url(Url),
    FileDir(PathBuf),
}

#[derive(Clone, Debug)]
pub struct PageResources {
    base: ResourceBase,
}

impl PageResources {
    pub fn from_url(base: Url) -> Self {
        Self {
            base: ResourceBase::Url(base),
        }
    }

    pub fn from_file_dir(base_dir: PathBuf) -> Self {
        Self {
            base: ResourceBase::FileDir(base_dir),
        }
    }

    fn resolve_reference(&self, reference: &str) -> Option<ResolvedReference> {
        let reference = reference.trim();
        if reference.is_empty() {
            return None;
        }

        if reference.starts_with("http://") || reference.starts_with("https://") {
            return Some(ResolvedReference::Url(reference.to_owned()));
        }

        match &self.base {
            ResourceBase::Url(base) => {
                let url = base.resolve(reference)?.as_str().to_owned();
                Some(ResolvedReference::Url(url))
            }
            ResourceBase::FileDir(dir) => Some(ResolvedReference::File(resolve_file_reference(
                dir, reference,
            ))),
        }
    }
}

impl ResourceLoader for PageResources {
    fn load_bytes(&self, reference: &str) -> Result<Option<Vec<u8>>, String> {
        let Some(resolved) = self.resolve_reference(reference) else {
            return Ok(None);
        };

        match resolved {
            ResolvedReference::Url(url) => match net::fetch_url_bytes(&url) {
                Ok(bytes) => Ok(Some(bytes)),
                Err(_) => Ok(None),
            },
            ResolvedReference::File(path) => match std::fs::read(&path) {
                Ok(bytes) => Ok(Some(bytes)),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(_) => Ok(None),
            },
        }
    }
}

#[derive(Clone, Debug)]
enum ResolvedReference {
    Url(String),
    File(PathBuf),
}

fn resolve_file_reference(base_dir: &Path, reference: &str) -> PathBuf {
    let reference = reference
        .split('#')
        .next()
        .unwrap_or(reference)
        .split('?')
        .next()
        .unwrap_or(reference)
        .trim();

    if reference.starts_with('/') {
        return PathBuf::from(reference);
    }

    base_dir.join(reference)
}

