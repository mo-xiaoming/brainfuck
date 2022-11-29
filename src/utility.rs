#![allow(dead_code)]

pub(crate) struct Timing<'a> {
    a: std::time::Instant,
    name: &'a str,
}

impl<'a> Timing<'a> {
    pub(crate) fn new(name: &'a str) -> Self {
        Self {
            a: std::time::Instant::now(),
            name,
        }
    }
}

impl<'a> Drop for Timing<'a> {
    fn drop(&mut self) {
        //println!("{}: {}", self.name, self.a.elapsed().as_nanos());
    }
}

#[cfg(test)]
pub(crate) mod traits {
    /// have everything a value type should have
    pub(crate) fn is_small_value_struct<T>(_: &T)
    where
        T: Sync
            + Send
            + Copy
            + Clone
            + Default
            + std::fmt::Debug
            + std::hash::Hash
            + PartialEq
            + Eq
            + PartialOrd
            + Ord,
    {
    }
    /// have everything a value type should have, but no meaningful default
    pub(crate) fn is_small_value_struct_but_no_default<T>(_: &T)
    where
        T: Sync
            + Send
            + Copy
            + Clone
            + std::fmt::Debug
            + std::hash::Hash
            + PartialEq
            + Eq
            + PartialOrd
            + Ord,
    {
    }
    // like `is_small_value_struct`, but too big to be copied around
    pub(crate) fn is_big_value_struct<T>(_: &T)
    where
        T: Sync
            + Send
            + Clone
            + Default
            + std::fmt::Debug
            + std::hash::Hash
            + PartialEq
            + Eq
            + PartialOrd
            + Ord,
    {
    }
    // like `is_big_value_struct`, but too big to be copied around, and no meaningful default
    pub(crate) fn is_big_value_struct_but_no_default<T>(_: &T)
    where
        T: Sync
            + Send
            + Clone
            + std::fmt::Debug
            + std::hash::Hash
            + PartialEq
            + Eq
            + PartialOrd
            + Ord,
    {
    }
    /// have everything a value type should have
    /// short of `Default` compares to `is_small_value_struct`
    pub(crate) fn is_small_value_enum<T>(_: &T)
    where
        T: Sync
            + Send
            + Copy
            + Clone
            + std::fmt::Debug
            + std::hash::Hash
            + PartialEq
            + Eq
            + PartialOrd
            + Ord,
    {
    }
    // like `is_small_value_enum`, but too big to be copied around
    pub(crate) fn is_big_value_enum<T>(_: &T)
    where
        T: Sync
            + Send
            + Clone
            + std::fmt::Debug
            + std::hash::Hash
            + PartialEq
            + Eq
            + PartialOrd
            + Ord,
    {
    }
    pub(crate) fn is_big_but_incomparable_and_no_default<T>(_: &T)
    where
        T: std::fmt::Debug + Sync + Send + Clone,
    {
    }
    pub(crate) fn is_big_but_incomparable<T>(_: &T)
    where
        T: Default + std::fmt::Debug + Sync + Send + Clone,
    {
    }
    pub(crate) fn is_default_debug<T>(_: &T)
    where
        T: Default + std::fmt::Debug + Sync + Send,
    {
    }
    pub(crate) fn is_debug<T>(v: &T) -> usize
    where
        T: std::fmt::Debug + Sync + Send,
    {
        format!("{:?}", v).len()
    }
}
