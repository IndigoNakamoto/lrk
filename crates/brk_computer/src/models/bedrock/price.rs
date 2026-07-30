//! Bedrock-local price wrapper for metrics whose stored source is indexed by day.
//!
//! This mirrors the per-block `Price` conversion chain while preserving Bedrock's
//! custom daily repeat/last-day views: cents are stored, USD is derived from cents,
//! and sats are derived from USD.

use brk_traversable::Traversable;
use brk_types::{Cents, Day1, Dollars, SatsFract, Version};
use schemars::JsonSchema;
use vecdb::{
    LazyVecFrom1, ReadableBoxedVec, ReadableCloneableVec, Rw, StorageMode, UnaryTransform,
};

use super::urpd_metric::{UrpdMappings, UrpdMetric, UrpdViews};
use crate::internal::{CentsUnsignedToDollars, DollarsToSatsFract, NumericValue};

type LazyDay<T, S> = LazyVecFrom1<Day1, T, Day1, S>;

#[derive(Clone, Traversable)]
#[traversable(merge)]
pub struct LazyUrpdMetric<T, S>
where
    T: NumericValue + JsonSchema,
    S: NumericValue,
{
    pub day1: LazyDay<T, S>,
    #[traversable(flatten)]
    pub views: Box<UrpdViews<T>>,
}

impl<T, S> LazyUrpdMetric<T, S>
where
    T: NumericValue + JsonSchema + 'static,
    S: NumericValue + JsonSchema,
{
    fn from_source<F>(
        name: &str,
        version: Version,
        source: ReadableBoxedVec<Day1, S>,
        mappings: &UrpdMappings,
    ) -> Self
    where
        F: UnaryTransform<S, T>,
    {
        let day1 = LazyVecFrom1::transformed::<F>(name, version, source);
        let views = Box::new(UrpdViews::new(
            name,
            day1.read_only_boxed_clone(),
            version,
            mappings,
        ));

        Self { day1, views }
    }
}

#[derive(Traversable)]
pub struct Price<M: StorageMode = Rw> {
    pub usd: LazyUrpdMetric<Dollars, Cents>,
    pub cents: UrpdMetric<Cents, M>,
    pub sats: LazyUrpdMetric<SatsFract, Dollars>,
}

impl Price {
    pub(crate) fn forced_import(
        db: &vecdb::Database,
        name: &str,
        version: Version,
        mappings: &UrpdMappings,
    ) -> brk_error::Result<Self> {
        let cents = UrpdMetric::forced_import(db, &format!("{name}_cents"), version, mappings)?;
        let usd = LazyUrpdMetric::from_source::<CentsUnsignedToDollars>(
            name,
            version,
            cents.day1.read_only_boxed_clone(),
            mappings,
        );
        let sats = LazyUrpdMetric::from_source::<DollarsToSatsFract>(
            &format!("{name}_sats"),
            version,
            usd.day1.read_only_boxed_clone(),
            mappings,
        );

        Ok(Self { usd, cents, sats })
    }
}
