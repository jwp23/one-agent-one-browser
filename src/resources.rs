use crate::net;
use crate::url::Url;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub trait ResourceLoader {
    fn load_bytes(&self, reference: &str) -> Result<Option<Arc<Vec<u8>>>, String>;
}

pub struct NoResources;

impl ResourceLoader for NoResources {
    fn load_bytes(&self, _reference: &str) -> Result<Option<Arc<Vec<u8>>>, String> {
        Ok(None)
    }
}

#[derive(Clone, Debug)]
pub enum ResourceBase {
    Url(Url),
    FileDir(PathBuf),
}

pub struct ResourceManager {
    base: ResourceBase,
    state: RefCell<ResourceState>,
}

impl ResourceManager {
    pub fn from_url(base: Url) -> Self {
        Self::new(ResourceBase::Url(base))
    }

    pub fn from_file_dir(base_dir: PathBuf) -> Self {
        Self::new(ResourceBase::FileDir(base_dir))
    }

    fn new(base: ResourceBase) -> Self {
        Self {
            base,
            state: RefCell::new(ResourceState::new()),
        }
    }

    pub fn tick(&self) -> ResourceTickResult {
        self.state.borrow_mut().drain_events()
    }

    fn resolve_reference(&self, reference: &str) -> Option<ResolvedReference> {
        resolve_reference(&self.base, reference)
    }

    fn cache_file(&self, path: PathBuf) -> Option<Arc<Vec<u8>>> {
        let mut state = self.state.borrow_mut();
        let key = ResolvedReference::File(path.clone());

        if let Some(bytes) = state.cache_ok.get(&key) {
            return Some(Arc::clone(bytes));
        }
        if state.cache_fail.contains(&key) {
            return None;
        }

        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => {
                state.cache_fail.insert(key);
                return None;
            }
        };

        if !crate::image::looks_like_supported_image(&bytes) {
            state.cache_fail.insert(key);
            return None;
        }

        let bytes = Arc::new(bytes);
        state.cache_ok.insert(key, Arc::clone(&bytes));
        Some(bytes)
    }

    fn cache_url(&self, url: String) -> Result<Option<Arc<Vec<u8>>>, String> {
        let mut state = self.state.borrow_mut();
        let key = ResolvedReference::Url(url.clone());

        if let Some(bytes) = state.cache_ok.get(&key) {
            return Ok(Some(Arc::clone(bytes)));
        }
        if state.cache_fail.contains(&key) {
            return Ok(None);
        }

        if state.pending.contains_key(&key) {
            return Ok(None);
        }

        match state.pool.fetch_bytes(url) {
            Ok(request_id) => {
                state.pending.insert(key, request_id);
                Ok(None)
            }
            Err(err) => Err(err),
        }
    }
}

impl ResourceLoader for ResourceManager {
    fn load_bytes(&self, reference: &str) -> Result<Option<Arc<Vec<u8>>>, String> {
        let Some(resolved) = self.resolve_reference(reference) else {
            return Ok(None);
        };

        match resolved {
            ResolvedReference::File(path) => Ok(self.cache_file(path)),
            ResolvedReference::Url(url) => self.cache_url(url),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ResourceTickResult {
    pub new_successes: usize,
}

struct ResourceState {
    pool: net::FetchPool,
    pending: HashMap<ResolvedReference, net::RequestId>,
    cache_ok: HashMap<ResolvedReference, Arc<Vec<u8>>>,
    cache_fail: HashSet<ResolvedReference>,
}

impl ResourceState {
    fn new() -> Self {
        Self {
            pool: net::FetchPool::new(8),
            pending: HashMap::new(),
            cache_ok: HashMap::new(),
            cache_fail: HashSet::new(),
        }
    }

    fn drain_events(&mut self) -> ResourceTickResult {
        let mut new_successes = 0usize;

        while let Some(event) = self.pool.try_recv() {
            let key = ResolvedReference::Url(event.url);
            let Some(_) = self.pending.remove(&key) else {
                continue;
            };

            match event.result {
                Ok(bytes) => {
                    if crate::image::looks_like_supported_image(&bytes) {
                        self.cache_ok.insert(key, Arc::new(bytes));
                        new_successes = new_successes.saturating_add(1);
                    } else {
                        self.cache_fail.insert(key);
                    }
                }
                Err(_) => {
                    self.cache_fail.insert(key);
                }
            }
        }

        ResourceTickResult { new_successes }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum ResolvedReference {
    Url(String),
    File(PathBuf),
}

fn resolve_reference(base: &ResourceBase, reference: &str) -> Option<ResolvedReference> {
    let reference = reference.trim();
    if reference.is_empty() {
        return None;
    }

    if reference.starts_with("http://") || reference.starts_with("https://") {
        return Some(ResolvedReference::Url(reference.to_owned()));
    }

    match base {
        ResourceBase::Url(base) => {
            let url = base.resolve(reference)?.as_str().to_owned();
            Some(ResolvedReference::Url(url))
        }
        ResourceBase::FileDir(dir) => Some(ResolvedReference::File(resolve_file_reference(
            dir, reference,
        ))),
    }
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
