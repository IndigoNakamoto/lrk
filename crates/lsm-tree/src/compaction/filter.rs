//! Definitions for compaction filters
//!
//! Compaction filters allow users to run custom logic during compactions, e.g. custom cleanup rules such as TTL.
//! Because compactions run in background workers, using compactions filters instead of scans can massively increase the efficiency of the storage engine.
use crate::{
    InternalValue, UserKey, UserValue, ValueType,
    compaction::stream::{StreamFilter, StreamFilterVerdict},
};
use std::panic::RefUnwindSafe;

/// Verdict returned by a [`CompactionFilter`].
#[non_exhaustive]
#[derive(Debug, Default)]
pub enum Verdict {
    /// Keeps the item.
    #[default]
    Keep,

    /// Removes the item.
    Remove,

    /// Removes the item and replace it with a weak tombstone.
    ///
    /// This may cause old versions of this item to be resurrected.
    /// The semantics of this operation are identical to [`remove_weak`](crate::AbstractTree::remove_weak).
    RemoveWeak,

    /// Replaces the value of the item.
    ReplaceValue(UserValue),

    /// Destroys a value - does not leave behind a tombstone.
    ///
    /// Only use in situations where you absolutely 100% know your
    /// item key is never written or updated multiple times.
    Destroy,
}

/// Trait to implement custom compaction filters
pub trait CompactionFilter: Send {
    /// Returns whether an item should be kept during compaction.
    ///
    /// # Panicking
    ///
    /// This function should NOT panic.
    ///
    /// # Errors
    ///
    /// Returning an error will abort the running compaction.
    /// This should only be done when **strictly** necessary, such as when fetching a value fails.
    fn filter_item(&mut self, item: ItemAccessor<'_>, ctx: &Context) -> crate::Result<Verdict>;

    // TODO: how would we go about adding custom properties to tables?
    // a function that gets called every time a table is cut...? e.g.:
    // fn on_table_cut(&mut self, ctx: ...) {}

    /// Called when compaction is finished.
    ///
    /// # Panicking
    ///
    /// This function should NOT panic.
    fn finish(self: Box<Self>) {}
}

/// Context passed to compaction filters and factories for each compaction run
#[non_exhaustive]
#[derive(Debug)]
pub struct Context {
    /// Whether we are compacting into the last level.
    pub is_last_level: bool,
}

/// Trait that creates compaction filter objects for each compaction
pub trait Factory: Send + Sync + RefUnwindSafe {
    /// Returns the compaction filter name.
    ///
    /// This is currently only used for logging purposes.
    fn name(&self) -> &str;

    /// Returns a new compaction filter.
    ///
    /// # Panicking
    ///
    /// This function should NOT panic.
    fn make_filter(&self, ctx: &Context) -> Box<dyn CompactionFilter>;
}

/// Accessor for the key/value from a compaction filter
pub struct ItemAccessor<'a> {
    item: &'a InternalValue,
}

impl<'a> ItemAccessor<'a> {
    /// Get the key of this item.
    #[must_use]
    pub fn key(&self) -> &'a UserKey {
        &self.item.key.user_key
    }

    /// Get the value of this item.
    ///
    /// # Errors
    ///
    /// This method will return an error if reading the value fails.
    pub fn value(&self) -> crate::Result<UserValue> {
        if self.item.is_tombstone() {
            unreachable!("tombstones are filtered out before calling filter");
        }
        Ok(self.item.value.clone())
    }
}

/// Adapts a [`CompactionFilter`] to a [`StreamFilter`]
//
// NOTE: this slightly helps insulate CompactionStream from lifetime spam
pub(crate) struct StreamFilterAdapter<'a, 'b: 'a> {
    filter: Option<&'a mut (dyn CompactionFilter + 'b)>,
    ctx: &'a Context,
}

impl<'a, 'b: 'a> StreamFilterAdapter<'a, 'b> {
    pub fn new(filter: Option<&'a mut (dyn CompactionFilter + 'b)>, ctx: &'a Context) -> Self {
        Self { filter, ctx }
    }
}

impl<'a, 'b: 'a> StreamFilter for StreamFilterAdapter<'a, 'b> {
    fn filter_item(&mut self, item: &InternalValue) -> crate::Result<StreamFilterVerdict> {
        let Some(filter) = self.filter.as_mut() else {
            return Ok(StreamFilterVerdict::Keep);
        };

        match filter.filter_item(ItemAccessor { item }, self.ctx)? {
            Verdict::Destroy => Ok(StreamFilterVerdict::Drop),
            Verdict::Keep => Ok(StreamFilterVerdict::Keep),
            Verdict::Remove => Ok(StreamFilterVerdict::Replace((
                ValueType::Tombstone,
                UserValue::empty(),
            ))),
            Verdict::RemoveWeak => Ok(StreamFilterVerdict::Replace((
                ValueType::WeakTombstone,
                UserValue::empty(),
            ))),
            Verdict::ReplaceValue(new_value) => {
                Ok(StreamFilterVerdict::Replace((ValueType::Value, new_value)))
            }
        }
    }
}
