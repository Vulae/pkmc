// NOTE: This will all be unneeded soon: https://github.com/rust-lang/rust/issues/43244

use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

// NOTE: I gave up this for now, I'm too dumb for rust magic like this (If it is even is possible)
//pub trait IterRetain<'a, T> {
//    type I<'b: 'a>;
//    fn retain_returned<F>(&'a mut self, predicate: F) -> Vec<T>
//    where
//        for<'b> F: Fn(Self::I<'b>) -> bool + 'b;
//}
//
//impl<'a, T> IterRetain<'a, T> for Vec<T> {
//    type I<'b: 'a> = &'b T;
//    fn retain_returned<F>(&'a mut self, predicate: F) -> Vec<T>
//    where
//        for<'b> F: Fn(Self::I<'b>) -> bool + 'b,
//    {
//        let mut removed = Vec::new();
//        for i in (0..self.len()).rev() {
//            if !predicate(&self[i]) {
//                removed.push(self.remove(i));
//            }
//        }
//        removed.into_iter().rev().collect()
//    }
//}

pub fn retain_returned_vec<T, F>(vec: &mut Vec<T>, predicate: F) -> Vec<T>
where
    F: Fn(&T) -> bool,
{
    let mut removed = Vec::new();
    for i in (0..vec.len()).rev() {
        if !predicate(&vec[i]) {
            removed.push(vec.remove(i));
        }
    }
    removed.into_iter().rev().collect()
}

pub fn retain_returned_hashset<T: Eq + Hash, F>(hashset: &mut HashSet<T>, predicate: F) -> Vec<T>
where
    F: Fn(&T) -> bool,
{
    hashset
        .drain()
        .collect::<Vec<_>>()
        .into_iter()
        .fold(Vec::new(), |mut removed, value| {
            if !predicate(&value) {
                removed.push(value);
            } else {
                hashset.insert(value);
            }
            removed
        })
}

pub fn retain_returned_hashmap<K: Eq + Hash, V, F>(
    hashmap: &mut HashMap<K, V>,
    predicate: F,
) -> Vec<(K, V)>
where
    F: Fn(&K, &V) -> bool,
{
    hashmap
        .drain()
        .collect::<Vec<_>>()
        .into_iter()
        .fold(Vec::new(), |mut removed, (key, value)| {
            if !predicate(&key, &value) {
                removed.push((key, value));
            } else {
                hashmap.insert(key, value);
            }
            removed
        })
}
