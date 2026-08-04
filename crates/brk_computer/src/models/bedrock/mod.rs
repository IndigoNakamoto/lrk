mod compute;
mod import;
mod price;
mod vecs;

use std::path::PathBuf;

use brk_cohort::{AGE_RANGE_NAMES, UTXO_AGGREGATE_NAMES, UTXO_ALL_NAME};
use brk_error::Result;
use brk_types::{Cohort, Date, Day1, UrpdRaw, UrpdWeight};
use vecdb::{ReadableVec, StorageMode};

use self::compute::resolve_age_value;
use crate::Computer;

pub use vecs::Vecs;

pub(crate) const STORED_URPD_COHORTS: [brk_cohort::CohortName; 3] = UTXO_AGGREGATE_NAMES;

pub(crate) fn weighted_urpd_name(weight: UrpdWeight, cohort: &str) -> String {
    debug_assert!(weight.is_weighted());
    if cohort == UTXO_ALL_NAME.id {
        format!("bedrock_{}", weight.as_str())
    } else {
        format!("bedrock_{}_{cohort}", weight.as_str())
    }
}

impl<M: StorageMode> Computer<M> {
    /// Directory containing a persisted aggregate Bedrock-weighted URPD.
    pub fn bedrock_urpd_dir(&self, weight: UrpdWeight, cohort: &Cohort) -> PathBuf {
        UrpdRaw::dir(
            &self.models.bedrock.states_path,
            &weighted_urpd_name(weight, cohort),
        )
    }

    /// Read a persisted aggregate Bedrock-weighted URPD.
    pub fn bedrock_urpd_raw(
        &self,
        weight: UrpdWeight,
        cohort: &Cohort,
        date: Date,
    ) -> Result<UrpdRaw> {
        UrpdRaw::read(
            &self.models.bedrock.states_path,
            &weighted_urpd_name(weight, cohort),
            date,
        )
    }

    /// Resolve one age-range cohort's scalar Bedrock weight for a day.
    pub fn bedrock_urpd_weight(
        &self,
        cohort: &Cohort,
        day: Day1,
        weight: UrpdWeight,
    ) -> Option<f64> {
        let cohort_id = cohort.strip_prefix("utxos_")?;
        let age = AGE_RANGE_NAMES
            .iter()
            .position(|name| name.id == cohort_id)?;

        if weight == UrpdWeight::Raw {
            return Some(1.0);
        }

        let supply = self
            .distribution
            .utxo_cohorts
            .age_range
            .iter()
            .nth(age)?
            .metrics
            .supply
            .total
            .sats
            .day1
            .collect_one(day)
            .flatten()?;

        match weight {
            UrpdWeight::Raw => Some(1.0),
            UrpdWeight::Cointime => {
                let cohort = self.frameworks.cointime.age_range.iter().nth(age)?;
                resolve_age_value(cohort.wakefulness.day1.collect_one(day).flatten(), supply)
                    .map(|value| value.clamp(0.0, 1.0))
            }
            UrpdWeight::Coinflow => {
                let cohort = self.frameworks.coinflow.age_range.iter().nth(age)?;
                resolve_age_value(cohort.mobility.day1.0.collect_one(day).flatten(), supply)
                    .map(|value| value.clamp(0.0, 1.0))
            }
        }
    }
}
