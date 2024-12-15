use std::{
    collections::HashSet,
    hash::{BuildHasher, Hash},
};

pub trait IterRetain<T> {
    fn retain_returned<F>(&mut self, predicate: F) -> impl Iterator<Item = T>
    where
        F: Fn(&T) -> bool;
}

impl<T> IterRetain<T> for Vec<T> {
    fn retain_returned<F>(&mut self, predicate: F) -> impl Iterator<Item = T>
    where
        F: Fn(&T) -> bool,
    {
        let mut removed = Vec::new();
        for i in (0..self.len()).rev() {
            if !predicate(&self[i]) {
                removed.push(self.remove(i));
            }
        }
        removed.into_iter().rev()
    }
}

// TODO: Possible without clone? I'm not enough of a rust pro to know.
impl<T: Eq + Hash + Clone, S: BuildHasher> IterRetain<T> for HashSet<T, S> {
    fn retain_returned<F>(&mut self, predicate: F) -> impl Iterator<Item = T>
    where
        F: Fn(&T) -> bool,
    {
        self.iter()
            .filter(|item| !predicate(item))
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .map(|item| self.take(&item).unwrap())
            .collect::<Vec<_>>()
            .into_iter()
    }
}
