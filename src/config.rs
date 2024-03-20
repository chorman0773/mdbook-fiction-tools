use std::iter::FusedIterator;

use serde_derive::Deserialize;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum SerList<T> {
    SingleItem(T),
    List(Vec<T>),
}

impl<T> Default for SerList<T> {
    fn default() -> Self {
        Self::List(vec![])
    }
}

impl<T> SerList<T> {
    pub fn iter(&self) -> Iter<T> {
        match self {
            Self::SingleItem(it) => Iter {
                inner: ListIterInner::SingleItem(it),
            },
            Self::List(v) => Iter {
                inner: ListIterInner::List(v.iter()),
            },
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        match self {
            Self::SingleItem(it) => IterMut {
                inner: ListIterInner::SingleItem(it),
            },
            Self::List(v) => IterMut {
                inner: ListIterInner::List(v.iter_mut()),
            },
        }
    }
}

impl<T> IntoIterator for SerList<T> {
    type IntoIter = IntoIter<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::SingleItem(it) => IntoIter {
                inner: ListIterInner::SingleItem(it),
            },
            Self::List(v) => IntoIter {
                inner: ListIterInner::List(v.into_iter()),
            },
        }
    }
}

impl<'a, T> IntoIterator for &'a SerList<T> {
    type IntoIter = Iter<'a, T>;
    type Item = &'a T;

    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut SerList<T> {
    type IntoIter = IterMut<'a, T>;
    type Item = &'a mut T;

    fn into_iter(self) -> IterMut<'a, T> {
        self.iter_mut()
    }
}

enum ListIterInner<T, I> {
    Finished,
    SingleItem(T),
    List(I),
}

impl<T, I: Iterator<Item = T>> Iterator for ListIterInner<T, I> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match core::mem::replace(self, Self::Finished) {
            Self::Finished => None,
            Self::SingleItem(it) => Some(it),
            Self::List(mut iter) => {
                let item = iter.next()?;

                *self = Self::List(iter);

                Some(item)
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Finished => (0, Some(0)),
            Self::SingleItem(it) => (1, Some(1)),
            Self::List(iter) => iter.size_hint(),
        }
    }
}

impl<T, I: DoubleEndedIterator<Item = T>> DoubleEndedIterator for ListIterInner<T, I> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match core::mem::replace(self, Self::Finished) {
            Self::Finished => None,
            Self::SingleItem(it) => Some(it),
            Self::List(mut iter) => {
                let item = iter.next_back()?;

                *self = Self::List(iter);

                Some(item)
            }
        }
    }
}

pub struct IntoIter<T> {
    inner: ListIterInner<T, std::vec::IntoIter<T>>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

impl<T> FusedIterator for IntoIter<T> {}
impl<T> ExactSizeIterator for IntoIter<T> {}

pub struct Iter<'a, T> {
    inner: ListIterInner<&'a T, std::slice::Iter<'a, T>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

impl<'a, T> FusedIterator for Iter<'a, T> {}
impl<'a, T> ExactSizeIterator for Iter<'a, T> {}

pub struct IterMut<'a, T> {
    inner: ListIterInner<&'a mut T, std::slice::IterMut<'a, T>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a, T> DoubleEndedIterator for IterMut<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

impl<'a, T> FusedIterator for IterMut<'a, T> {}
impl<'a, T> ExactSizeIterator for IterMut<'a, T> {}
