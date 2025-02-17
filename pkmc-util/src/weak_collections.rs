use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard, Weak},
};

use crate::{retain_returned_hashmap, retain_returned_vec};

/// Like [`MutexGuard`], but owns the Mutex, instead of a lifetime to that Mutex.
#[derive(Debug)]
pub struct OwnedMutexGuard<'a, T: 'a> {
    _ref: Arc<Mutex<T>>,
    guard: MutexGuard<'a, T>,
}

impl<T> OwnedMutexGuard<'_, T> {
    fn new(weak: &Weak<Mutex<T>>) -> Option<Self> {
        let arc = weak.upgrade()?;
        let binding = arc.clone();
        let guard = binding.lock().unwrap();
        Some(Self {
            _ref: arc,
            #[allow(clippy::missing_transmute_annotations)]
            guard: unsafe { std::mem::transmute(guard) },
        })
    }
}

impl<T> Deref for OwnedMutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<T> DerefMut for OwnedMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

/// Like [`RwLockReadGuard`], but owns the RwLock, instead of a lifetime to that RwLock.
#[derive(Debug)]
pub struct OwnedRwLockReadGuard<'a, T: 'a> {
    _ref: Arc<RwLock<T>>,
    guard: RwLockReadGuard<'a, T>,
}

impl<T> OwnedRwLockReadGuard<'_, T> {
    fn new(weak: &Weak<RwLock<T>>) -> Option<Self> {
        let arc = weak.upgrade()?;
        let binding = arc.clone();
        let guard = binding.read().unwrap();
        Some(Self {
            _ref: arc,
            #[allow(clippy::missing_transmute_annotations)]
            guard: unsafe { std::mem::transmute(guard) },
        })
    }
}

impl<T> Deref for OwnedRwLockReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

/// Like [`RwLockWriteGuard`], but owns the RwLock, instead of a lifetime to that RwLock.
#[derive(Debug)]
pub struct OwnedRwLockWriteGuard<'a, T: 'a> {
    _ref: Arc<RwLock<T>>,
    guard: RwLockWriteGuard<'a, T>,
}

impl<T> OwnedRwLockWriteGuard<'_, T> {
    fn new(weak: &Weak<RwLock<T>>) -> Option<Self> {
        let arc = weak.upgrade()?;
        let binding = arc.clone();
        let guard = binding.write().unwrap();
        Some(Self {
            _ref: arc,
            #[allow(clippy::missing_transmute_annotations)]
            guard: unsafe { std::mem::transmute(guard) },
        })
    }
}

impl<T> Deref for OwnedRwLockWriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<T> DerefMut for OwnedRwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

/// Stores Weak references to <code>\<T></code>
/// As elements of the list may be dropped at any moment, this cannot be indexed, only iterated.
/// And for the same reason, elements can only be removed by dropping the strong reference.
///
/// # Example
///
/// ```
/// # use std::sync::{Arc, Mutex};
/// # use pkmc_util::WeakList;
///
/// let mut list: WeakList<Mutex<String>> = WeakList::new();
/// let item1: Arc<Mutex<String>> = list.push(Mutex::new("Hello".to_owned()));
/// let item2: Arc<Mutex<String>> = list.push(Mutex::new("World".to_owned()));
///
/// // Iterate through each element in the list.
/// list.iter().for_each(|mut v| v.push_str("!"));
///
/// assert_eq!(
///     list.iter().map(|v| v.clone()).collect::<Vec<String>>(),
///     vec!["Hello!".to_owned(), "World!".to_owned()]
/// );
///
/// *item1.lock().unwrap() = "Bye".to_owned();
///
/// assert_eq!(
///     list.iter().map(|v| v.clone()).collect::<Vec<String>>(),
///     vec!["Bye".to_owned(), "World!".to_owned()]
/// );
///
/// // item1 is dropped, no longer accessable inside the WeakList.
/// std::mem::drop(item1);
///
/// assert_eq!(
///     list.iter().map(|v| v.clone()).collect::<Vec<String>>(),
///     vec!["World!".to_owned()]
/// );
///
/// assert_eq!(list.length(), 1);
/// // WeakList::cleanup removes all elements that are dropped & returns number of elements dropped.
/// // It is not needed to cleanup at all, but it does help with memory & speed to remove all
/// // unused elements.
/// assert_eq!(list.cleanup(), 1);
///
/// std::mem::drop(item2);
///
/// assert!(list.is_empty());
/// assert_eq!(list.cleanup(), 1);
/// ```
#[derive(Debug)]
pub struct WeakList<T> {
    entries: Vec<Weak<T>>,
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

    /// Remove dropped elements inside the list, returning number of removed elements.
    pub fn cleanup(&mut self) -> usize {
        retain_returned_vec(&mut self.entries, |e| e.strong_count() > 0).len()
    }

    /// If the list is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(|e| e.strong_count() == 0)
    }

    /// Number of elements in the list.
    pub fn length(&self) -> usize {
        self.entries.iter().filter(|e| e.strong_count() > 0).count()
    }

    /// Push a new element to the end of the list.
    /// Returning a <code>[`Arc`]<[`Mutex`]\<T>></code> to that element.
    pub fn push(&mut self, value: T) -> Arc<T> {
        let entry = Arc::new(value);
        self.entries.push(Arc::downgrade(&entry));
        entry
    }
}

impl<T> WeakList<Mutex<T>> {
    /// Iterate through each element in the list.
    pub fn iter<'a>(&self) -> impl Iterator<Item = OwnedMutexGuard<'a, T>> + use<'a, '_, T>
    where
        T: 'a,
    {
        self.entries.iter().flat_map(|e| OwnedMutexGuard::new(e))
    }

    /// Lock each element in the list.
    pub fn lock<'a>(&self) -> Vec<OwnedMutexGuard<'a, T>>
    where
        T: 'a,
    {
        self.iter().collect()
    }
}

impl<T> WeakList<RwLock<T>> {
    /// Read only iterate through each element in the list.
    pub fn iter_read<'a>(
        &self,
    ) -> impl Iterator<Item = OwnedRwLockReadGuard<'a, T>> + use<'a, '_, T>
    where
        T: 'a,
    {
        self.entries
            .iter()
            .flat_map(|e| OwnedRwLockReadGuard::new(e))
    }

    /// Read only lock each element in the list.
    pub fn lock_read<'a>(&self) -> Vec<OwnedRwLockReadGuard<'a, T>>
    where
        T: 'a,
    {
        self.iter_read().collect()
    }

    /// Iterate through each element in the list.
    pub fn iter_write<'a>(
        &self,
    ) -> impl Iterator<Item = OwnedRwLockWriteGuard<'a, T>> + use<'a, '_, T>
    where
        T: 'a,
    {
        self.entries
            .iter()
            .flat_map(|e| OwnedRwLockWriteGuard::new(e))
    }

    /// Lock each element in the list.
    pub fn lock_write<'a>(&self) -> Vec<OwnedRwLockWriteGuard<'a, T>>
    where
        T: 'a,
    {
        self.iter_write().collect()
    }
}

/// Stores Weak references to <code>\<T></code>
/// Elements can only be removed by dropping the strong reference.
///
/// # Example
/// ```
/// # use std::sync::{Arc, Mutex};
/// # use std::collections::HashSet;
/// # use pkmc_util::WeakMap;
///
/// let mut map: WeakMap<u32, Mutex<String>> = WeakMap::new();
/// // Insert elements into the WeakMap.
/// // WeakMap::insert(..) -> (inserted element, Option<old element>)
/// let (item1, _previous): (Arc<Mutex<String>>, Option<Arc<Mutex<String>>>) = map.insert(42, Mutex::new("Hello".to_owned()));
/// // WeakMap::insert_ignored(..) -> inserted element
/// // Not really recommended, unless you know it's going to never clash with any existing entry.
/// let item2: Arc<Mutex<String>> = map.insert_ignored(69, Mutex::new("World".to_owned()));
///
/// assert_eq!(map.get(&42).map(|v| v.clone()), Some("Hello".to_owned()));
/// assert_eq!(map.get(&69).map(|v| v.clone()), Some("World".to_owned()));
///
/// // Iterate through each entry in the map.
/// map.iter().for_each(|(k, mut v)| v.push_str("!"));
///
/// assert_eq!(map.get(&42).map(|v| v.clone()), Some("Hello!".to_owned()));
/// assert_eq!(map.get(&69).map(|v| v.clone()), Some("World!".to_owned()));
///
/// *item1.lock().unwrap() = "Bye".to_owned();
///
/// // Iterate through each entry.
/// map.iter().for_each(|(k, v)| println!("{} => \"{}\"", k, *v));
///
/// // item1 is dropped, no longer accessable inside the WeakMap.
/// std::mem::drop(item1);
///
/// assert_eq!(map.get(&42).map(|v| v.clone()), None);
/// assert_eq!(map.get(&69).map(|v| v.clone()), Some("World!".to_owned()));
///
/// assert_eq!(map.length(), 1);
/// // WeakMap::cleanup removes all elements that are dropped, & returns [`HashSet<K>`] of elements
/// // that were dropped.
/// // It is not needed to cleanup at all, but it does help with memory & speed to remove all
/// // unused elements.
/// assert_eq!(map.cleanup(), HashSet::from([42]));
///
/// std::mem::drop(item2);
///
/// assert!(map.is_empty());
/// assert_eq!(map.cleanup(), HashSet::from([69]));
/// ```
#[derive(Debug)]
pub struct WeakMap<K: Hash + Eq, V> {
    entries: HashMap<K, Weak<V>>,
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

    /// Remove dropped entries inside the map, returning a [`HashSet<K>`] of removed entries.
    pub fn cleanup(&mut self) -> HashSet<K> {
        retain_returned_hashmap(&mut self.entries, |_, v| v.strong_count() > 0)
            .into_iter()
            .map(|(k, _)| k)
            .collect()
    }

    /// If the map is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(|(_, v)| v.strong_count() == 0)
    }

    /// Number of elements in the list.
    pub fn length(&self) -> usize {
        self.entries
            .iter()
            .filter(|(_, v)| v.strong_count() > 0)
            .count()
    }

    /// Insert a new entry into the map.
    /// Returning a tuple where the first value is a <code>[`Arc`]\<V></code> to that
    /// element, and the second value is a <code>[`Option`]<[`Arc`]\<V>></code> to the
    /// previous element that was stored there.
    pub fn insert(&mut self, key: K, value: V) -> (Arc<V>, Option<Arc<V>>) {
        let entry = Arc::new(value);
        let last = self.entries.insert(key, Arc::downgrade(&entry));
        (entry, last.and_then(|e| e.upgrade()))
    }

    /// Insert a new entry into the map.
    /// Returning a <code>[`Arc`]\<V></code> to that element.
    /// Overwrites the previous element unknowingly to you, so it is recommended to use
    /// WeakMap::insert, unless you know for sure there will not be anything there.
    pub fn insert_ignored(&mut self, key: K, value: V) -> Arc<V> {
        let entry = Arc::new(value);
        self.entries.insert(key, Arc::downgrade(&entry));
        entry
    }

    /// Tries to insert a new entry into the map.
    /// Returning a <code>[`Option`]<[`Arc`]\<V>></code> to that element.
    /// If there's already an entry inside the map at the position, it is not inserted.
    pub fn try_insert(&mut self, key: K, value: V) -> Option<Arc<V>> {
        let entry = Arc::new(value);
        if let Some(weak_entry) = self.entries.get(&key) {
            if weak_entry.strong_count() > 0 {
                return None;
            }
        }
        self.entries.insert(key, Arc::downgrade(&entry));
        Some(entry)
    }
}

impl<K: Hash + Eq, V> WeakMap<K, Mutex<V>> {
    /// Gets an element inside the map.
    pub fn get<'a>(&self, key: &K) -> Option<OwnedMutexGuard<'a, V>>
    where
        V: 'a,
    {
        self.entries.get(key).and_then(|v| OwnedMutexGuard::new(v))
    }

    /// Iterate through each entry in the map.
    pub fn iter<'a>(&self) -> impl Iterator<Item = (&K, OwnedMutexGuard<'a, V>)>
    where
        V: 'a,
    {
        self.entries
            .iter()
            .flat_map(|(k, v)| OwnedMutexGuard::new(v).map(|v| (k, v)))
    }

    /// Lock each element in the map.
    pub fn lock<'a>(&self) -> HashMap<&K, OwnedMutexGuard<'a, V>>
    where
        V: 'a,
    {
        self.iter().collect()
    }
}

impl<K: Hash + Eq, V> WeakMap<K, RwLock<V>> {
    /// Read only gets an element inside the map.
    pub fn read<'a>(&self, key: &K) -> Option<OwnedRwLockReadGuard<'a, V>>
    where
        V: 'a,
    {
        self.entries
            .get(key)
            .and_then(|v| OwnedRwLockReadGuard::new(v))
    }

    /// Read only iterate through each entry in the map.
    pub fn iter_read<'a>(&self) -> impl Iterator<Item = (&K, OwnedRwLockReadGuard<'a, V>)>
    where
        V: 'a,
    {
        self.entries
            .iter()
            .flat_map(|(k, v)| OwnedRwLockReadGuard::new(v).map(|v| (k, v)))
    }

    /// Read only lock each element in the map.
    pub fn lock_read<'a>(&self) -> HashMap<&K, OwnedRwLockReadGuard<'a, V>>
    where
        V: 'a,
    {
        self.iter_read().collect()
    }

    /// Gets an element inside the map.
    pub fn write<'a>(&self, key: &K) -> Option<OwnedRwLockWriteGuard<'a, V>>
    where
        V: 'a,
    {
        self.entries
            .get(key)
            .and_then(|v| OwnedRwLockWriteGuard::new(v))
    }

    /// Iterate through each entry in the map.
    pub fn iter_write<'a>(&self) -> impl Iterator<Item = (&K, OwnedRwLockWriteGuard<'a, V>)>
    where
        V: 'a,
    {
        self.entries
            .iter()
            .flat_map(|(k, v)| OwnedRwLockWriteGuard::new(v).map(|v| (k, v)))
    }

    /// Lock each element in the map.
    pub fn lock_write<'a>(&self) -> HashMap<&K, OwnedRwLockWriteGuard<'a, V>>
    where
        V: 'a,
    {
        self.iter_write().collect()
    }
}
