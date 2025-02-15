use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, MutexGuard, Weak},
};

use crate::{retain_returned_hashmap, retain_returned_vec};

/// Like [`MutexGuard`], but owns the Mutex, instead of a lifetime to that mutex.
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
/// let mut list: WeakList<String> = WeakList::new();
/// let item1: Arc<Mutex<String>> = list.push("Hello".to_owned());
/// let item2: Arc<Mutex<String>> = list.push("World".to_owned());
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
    pub fn push(&mut self, value: T) -> Arc<Mutex<T>> {
        let entry = Arc::new(Mutex::new(value));
        self.entries.push(Arc::downgrade(&entry));
        entry
    }

    /// Iterate through each element in the list.
    /// Each element is wrapped inside a [`WeakCollectionElement<T>`].
    pub fn iter<'a>(&self) -> impl Iterator<Item = WeakCollectionElement<'a, T>> + use<'a, '_, T>
    where
        T: 'a,
    {
        self.entries
            .iter()
            .flat_map(|e| WeakCollectionElement::new(e))
    }

    /// Lock each element in the list, returning a [`WeakCollectionElement<T>`] of each element.
    pub fn lock<'a>(&self) -> Vec<WeakCollectionElement<'a, T>>
    where
        T: 'a,
    {
        self.iter().collect()
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
/// let mut map: WeakMap<u32, String> = WeakMap::new();
/// // Insert elements into the WeakMap.
/// // WeakMap::insert(..) -> (inserted element, Option<old element>)
/// let (item1, _previous): (Arc<Mutex<String>>, _) = map.insert(42, "Hello".to_owned());
/// // WeakMap::insert_ignored(..) -> inserted element
/// // Not really recommended, unless you know it's going to never clash with any existing entry.
/// let item2: Arc<Mutex<String>> = map.insert_ignored(69, "World".to_owned());
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
    /// Returning a tuple where the first value is a <code>[`Arc`]<[`Mutex`]\<V>></code> to that
    /// element, and the second value is a <code>[`Option`]<[`Arc`]<[`Mutex`]\<V>>></code> to the
    /// previous element that was stored there.
    pub fn insert(&mut self, key: K, value: V) -> (Arc<Mutex<V>>, Option<Arc<Mutex<V>>>) {
        let entry = Arc::new(Mutex::new(value));
        let last = self.entries.insert(key, Arc::downgrade(&entry));
        (entry, last.and_then(|e| e.upgrade()))
    }

    /// Insert a new entry into the map.
    /// Returning a <code>[`Arc`]<[`Mutex`]\<V>></code> to that element.
    /// Overwrites the previous element unknowingly to you, so it is recommended to use
    /// WeakMap::insert, unless you know for sure there will not be anything there.
    pub fn insert_ignored(&mut self, key: K, value: V) -> Arc<Mutex<V>> {
        let entry = Arc::new(Mutex::new(value));
        self.entries.insert(key, Arc::downgrade(&entry));
        entry
    }

    /// Tries to insert a new entry into the map.
    /// Returning a <code>[`Option`]<[`Arc`]<[`Mutex`]\<V>>></code> to that element.
    /// If there's already an entry inside the map at the position, it is not inserted.
    pub fn try_insert(&mut self, key: K, value: V) -> Option<Arc<Mutex<V>>> {
        let entry = Arc::new(Mutex::new(value));
        if let Some(weak_entry) = self.entries.get(&key) {
            if weak_entry.strong_count() > 0 {
                return None;
            }
        }
        self.entries.insert(key, Arc::downgrade(&entry));
        Some(entry)
    }

    /// Gets an element inside the map.
    /// Element is wrapped inside a [`WeakCollectionElement<V>`].
    pub fn get<'a>(&self, key: &K) -> Option<WeakCollectionElement<'a, V>>
    where
        V: 'a,
    {
        self.entries
            .get(key)
            .and_then(|v| WeakCollectionElement::new(v))
    }

    /// Iterate through each entry in the map.
    /// Element is wrapped inside a [`WeakCollectionElement<V>`].
    pub fn iter<'a>(&self) -> impl Iterator<Item = (&K, WeakCollectionElement<'a, V>)>
    where
        V: 'a,
    {
        self.entries
            .iter()
            .flat_map(|(k, v)| WeakCollectionElement::new(v).map(|v| (k, v)))
    }

    /// Lock each element in the map, returning a <code>(&K, [`WeakCollectionElement<T>`])</code> of each entry.
    pub fn lock<'a>(&self) -> HashMap<&K, WeakCollectionElement<'a, V>>
    where
        V: 'a,
    {
        self.iter().collect()
    }
}
