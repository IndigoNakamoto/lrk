use crate::{KvPair, UserKey, UserValue, tree::Guard as StandardGuard};

/// Guard to access key-value pairs
pub trait IterGuard {
    /// Accesses the key-value pair if the predicate returns `true`.
    ///
    /// The predicate receives the key - if returning false, the value
    /// may not be loaded if the tree is key-value separated.
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    fn into_inner_if(
        self,
        pred: impl Fn(&UserKey) -> bool,
    ) -> crate::Result<(UserKey, Option<UserValue>)>;

    /// Accesses the key-value pair.
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    fn into_inner(self) -> crate::Result<KvPair>;

    /// Accesses the key.
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    fn key(self) -> crate::Result<UserKey>;

    /// Returns the value size.
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    fn size(self) -> crate::Result<u32>;

    /// Accesses the value.
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    fn value(self) -> crate::Result<UserValue>
    where
        Self: Sized,
    {
        self.into_inner().map(|(_, v)| v)
    }
}

/// Iterator value.
pub type IterGuardImpl = StandardGuard;
