pub trait VecExt<T> {
    fn retain_returned<F>(&mut self, predicate: F) -> Vec<T>
    where
        F: Fn(&T) -> bool;
}

impl<T> VecExt<T> for Vec<T> {
    fn retain_returned<F>(&mut self, predicate: F) -> Vec<T>
    where
        F: Fn(&T) -> bool,
    {
        let mut removed = Vec::new();

        // TODO: This is most definitely not the correct or fastest way to do this.
        for i in (0..self.len()).rev() {
            if !predicate(&self[i]) {
                removed.push(self.remove(i));
            }
        }

        removed
    }
}
