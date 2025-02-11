use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, MutexGuard, Weak},
};

use crate::{retain_returned_hashmap, retain_returned_vec};

#[derive(Debug)]
pub struct WeakCollectionElement<'a, T: 'a> {
    _arc: Arc<Mutex<T>>,
    guard: MutexGuard<'a, T>,
}

impl<T> WeakCollectionElement<'_, T> {
    fn new(weak: &Weak<Mutex<T>>) -> Option<Self> {
        let arc = weak.upgrade()?;
        let binding = arc.clone();
        let guard = binding.lock().unwrap();
        Some(Self {
            _arc: arc,
            #[allow(clippy::missing_transmute_annotations)]
            guard: unsafe { std::mem::transmute(guard) },
        })
    }
}

impl<T> Deref for WeakCollectionElement<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<T> DerefMut for WeakCollectionElement<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

#[derive(Debug)]
pub struct WeakList<T> {
    entries: Vec<Weak<Mutex<T>>>,
}

impl<T> Default for WeakList<T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<T> WeakList<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cleanup(&mut self) -> usize {
        retain_returned_vec(&mut self.entries, |e| e.strong_count() > 0).len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(|e| e.strong_count() == 0)
    }

    pub fn length(&self) -> usize {
        self.entries.iter().filter(|e| e.strong_count() > 0).count()
    }

    pub fn push(&mut self, value: T) -> Arc<Mutex<T>> {
        let entry = Arc::new(Mutex::new(value));
        self.entries.push(Arc::downgrade(&entry));
        entry
    }

    pub fn iter<'a>(&self) -> impl Iterator<Item = WeakCollectionElement<'a, T>>
    where
        T: 'a,
    {
        self.entries
            .iter()
            .flat_map(|e| WeakCollectionElement::new(e))
            .collect::<Vec<_>>()
            .into_iter()
    }
}

#[derive(Debug)]
pub struct WeakMap<K: Hash + Eq, V> {
    entries: HashMap<K, Weak<Mutex<V>>>,
}

impl<K: Hash + Eq, V> Default for WeakMap<K, V> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl<K: Hash + Eq, V> WeakMap<K, V> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cleanup(&mut self) -> HashSet<K> {
        retain_returned_hashmap(&mut self.entries, |_, v| v.strong_count() > 0)
            .into_iter()
            .map(|(k, _)| k)
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(|(_, v)| v.strong_count() == 0)
    }

    pub fn length(&self) -> usize {
        self.entries
            .iter()
            .filter(|(_, v)| v.strong_count() > 0)
            .count()
    }

    pub fn insert(&mut self, key: K, value: V) -> (Arc<Mutex<V>>, Option<Arc<Mutex<V>>>) {
        let entry = Arc::new(Mutex::new(value));
        let last = self.entries.insert(key, Arc::downgrade(&entry));
        (entry, last.and_then(|e| e.upgrade()))
    }

    pub fn insert_ignored(&mut self, key: K, value: V) -> Arc<Mutex<V>> {
        let entry = Arc::new(Mutex::new(value));
        self.entries.insert(key, Arc::downgrade(&entry));
        entry
    }

    pub fn get<'a>(&self, key: &K) -> Option<WeakCollectionElement<'a, V>>
    where
        V: 'a,
    {
        self.entries
            .get(key)
            .and_then(|v| WeakCollectionElement::new(v))
    }

    pub fn iter<'a>(&self) -> impl Iterator<Item = (&K, WeakCollectionElement<'a, V>)>
    where
        V: 'a,
    {
        self.entries
            .iter()
            .flat_map(|(k, v)| WeakCollectionElement::new(v).map(|v| (k, v)))
    }
}
