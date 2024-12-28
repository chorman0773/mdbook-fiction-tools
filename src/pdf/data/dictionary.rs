use std::{
    hash::Hash,
    iter::FusedIterator,
    ops::{Deref, DerefMut},
};

use indexmap::IndexMap;

use super::{Name, Object};

#[derive(Clone, Debug)]
pub struct Iter<'a>(indexmap::map::Iter<'a, Name, Object>);

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a Name, &'a Object);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a> FusedIterator for Iter<'a> {}

#[derive(Clone, Debug)]
pub struct IntoIter(indexmap::map::IntoIter<Name, Object>);

impl Iterator for IntoIter {
    type Item = (Name, Object);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl ExactSizeIterator for IntoIter {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl FusedIterator for IntoIter {}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Dictionary(IndexMap<Name, Object>);

impl Hash for Dictionary {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.iter().0.as_slice().hash(state);
    }
}

impl IntoIterator for Dictionary {
    type IntoIter = IntoIter;
    type Item = (Name, Object);
    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter())
    }
}

impl<'a> IntoIterator for &'a Dictionary {
    type IntoIter = Iter<'a>;
    type Item = (&'a Name, &'a Object);

    fn into_iter(self) -> Self::IntoIter {
        Iter(self.0.iter())
    }
}

impl Deref for Dictionary {
    type Target = IndexMap<Name, Object>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Dictionary {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromIterator<(Name, Object)> for Dictionary {
    fn from_iter<T: IntoIterator<Item = (Name, Object)>>(iter: T) -> Self {
        Self::from_map(IndexMap::from_iter(iter))
    }
}

impl Dictionary {
    pub fn new() -> Dictionary {
        Dictionary(IndexMap::new())
    }

    pub const fn from_map(m: IndexMap<Name, Object>) -> Dictionary {
        Self(m)
    }

    pub fn iter(&self) -> Iter {
        Iter(self.0.iter())
    }
}
