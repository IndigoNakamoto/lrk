use brk_error::Result;
use brk_indexer::Indexer;
use brk_traversable::Traversable;
use brk_types::{Cents, Height, StoredI8, Version};
use vecdb::{AnyVec, Database, EagerVec, Exit, PcoVec, ReadableVec, Rw, StorageMode, WritableVec};

use crate::{
    indexes,
    internal::{PerBlock, Price},
};

use super::Component;

#[derive(Traversable)]
pub struct RarityMeterInner<M: StorageMode = Rw> {
    pub pct0_01: Price<PerBlock<Cents, M>>,
    pub pct0_5: Price<PerBlock<Cents, M>>,
    pub pct1: Price<PerBlock<Cents, M>>,
    pub pct2: Price<PerBlock<Cents, M>>,
    pub pct5: Price<PerBlock<Cents, M>>,
    pub pct10: Price<PerBlock<Cents, M>>,
    pub pct20: Price<PerBlock<Cents, M>>,
    pub pct30: Price<PerBlock<Cents, M>>,
    pub pct40: Price<PerBlock<Cents, M>>,
    pub pct50: Price<PerBlock<Cents, M>>,
    pub pct60: Price<PerBlock<Cents, M>>,
    pub pct70: Price<PerBlock<Cents, M>>,
    pub pct80: Price<PerBlock<Cents, M>>,
    pub pct90: Price<PerBlock<Cents, M>>,
    pub pct95: Price<PerBlock<Cents, M>>,
    pub pct98: Price<PerBlock<Cents, M>>,
    pub pct99: Price<PerBlock<Cents, M>>,
    pub pct99_5: Price<PerBlock<Cents, M>>,
    pub pct99_9: Price<PerBlock<Cents, M>>,
    pub index: PerBlock<StoredI8, M>,
    pub score: PerBlock<StoredI8, M>,
}

impl RarityMeterInner {
    pub(crate) fn forced_import(
        db: &Database,
        prefix: &str,
        version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        Ok(Self {
            pct0_01: Price::forced_import(db, &format!("{prefix}_pct0_01"), version, indexes)?,
            pct0_5: Price::forced_import(db, &format!("{prefix}_pct0_5"), version, indexes)?,
            pct1: Price::forced_import(db, &format!("{prefix}_pct01"), version, indexes)?,
            pct2: Price::forced_import(db, &format!("{prefix}_pct02"), version, indexes)?,
            pct5: Price::forced_import(db, &format!("{prefix}_pct05"), version, indexes)?,
            pct10: Price::forced_import(db, &format!("{prefix}_pct10"), version, indexes)?,
            pct20: Price::forced_import(db, &format!("{prefix}_pct20"), version, indexes)?,
            pct30: Price::forced_import(db, &format!("{prefix}_pct30"), version, indexes)?,
            pct40: Price::forced_import(db, &format!("{prefix}_pct40"), version, indexes)?,
            pct50: Price::forced_import(db, &format!("{prefix}_pct50"), version, indexes)?,
            pct60: Price::forced_import(db, &format!("{prefix}_pct60"), version, indexes)?,
            pct70: Price::forced_import(db, &format!("{prefix}_pct70"), version, indexes)?,
            pct80: Price::forced_import(db, &format!("{prefix}_pct80"), version, indexes)?,
            pct90: Price::forced_import(db, &format!("{prefix}_pct90"), version, indexes)?,
            pct95: Price::forced_import(db, &format!("{prefix}_pct95"), version, indexes)?,
            pct98: Price::forced_import(db, &format!("{prefix}_pct98"), version, indexes)?,
            pct99: Price::forced_import(db, &format!("{prefix}_pct99"), version, indexes)?,
            pct99_5: Price::forced_import(db, &format!("{prefix}_pct99_5"), version, indexes)?,
            pct99_9: Price::forced_import(db, &format!("{prefix}_pct99_9"), version, indexes)?,
            index: PerBlock::forced_import(db, &format!("{prefix}_index"), version, indexes)?,
            score: PerBlock::forced_import(db, &format!("{prefix}_score"), version, indexes)?,
        })
    }

    pub(super) fn compute(
        &mut self,
        components: &[&Component],
        spot: &impl ReadableVec<Height, Cents>,
        indexer: &Indexer,
        exit: &Exit,
    ) -> Result<()> {
        let starting_height = indexer.safe_lengths().height;
        let gather = |f: fn(&Component) -> &_| -> Vec<_> {
            components.iter().map(|component| f(component)).collect()
        };

        // Lower percentiles: max across all models (tightest lower bound)
        self.pct0_01.cents.height.compute_max_of_others(
            starting_height,
            &gather(|component| &component.pct0_01.price.cents.height),
            exit,
        )?;
        self.pct0_5.cents.height.compute_max_of_others(
            starting_height,
            &gather(|component| &component.pct0_5.price.cents.height),
            exit,
        )?;
        self.pct1.cents.height.compute_max_of_others(
            starting_height,
            &gather(|component| &component.pct1.price.cents.height),
            exit,
        )?;
        self.pct2.cents.height.compute_max_of_others(
            starting_height,
            &gather(|component| &component.pct2.price.cents.height),
            exit,
        )?;
        self.pct5.cents.height.compute_max_of_others(
            starting_height,
            &gather(|component| &component.pct5.price.cents.height),
            exit,
        )?;

        // Upper percentiles: min across all models (tightest upper bound)
        self.pct95.cents.height.compute_min_of_others(
            starting_height,
            &gather(|component| &component.pct95.price.cents.height),
            exit,
        )?;
        self.pct98.cents.height.compute_min_of_others(
            starting_height,
            &gather(|component| &component.pct98.price.cents.height),
            exit,
        )?;
        self.pct99.cents.height.compute_min_of_others(
            starting_height,
            &gather(|component| &component.pct99.price.cents.height),
            exit,
        )?;
        self.pct99_5.cents.height.compute_min_of_others(
            starting_height,
            &gather(|component| &component.pct99_5.price.cents.height),
            exit,
        )?;
        self.pct99_9.cents.height.compute_min_of_others(
            starting_height,
            &gather(|component| &component.pct99_9.price.cents.height),
            exit,
        )?;

        compute_inner_percentile(
            &mut self.pct10.cents.height,
            starting_height,
            &self.pct5.cents.height,
            &self.pct95.cents.height,
            10,
            exit,
        )?;
        compute_inner_percentile(
            &mut self.pct20.cents.height,
            starting_height,
            &self.pct5.cents.height,
            &self.pct95.cents.height,
            20,
            exit,
        )?;
        compute_inner_percentile(
            &mut self.pct30.cents.height,
            starting_height,
            &self.pct5.cents.height,
            &self.pct95.cents.height,
            30,
            exit,
        )?;
        compute_inner_percentile(
            &mut self.pct40.cents.height,
            starting_height,
            &self.pct5.cents.height,
            &self.pct95.cents.height,
            40,
            exit,
        )?;
        compute_inner_percentile(
            &mut self.pct50.cents.height,
            starting_height,
            &self.pct5.cents.height,
            &self.pct95.cents.height,
            50,
            exit,
        )?;
        compute_inner_percentile(
            &mut self.pct60.cents.height,
            starting_height,
            &self.pct5.cents.height,
            &self.pct95.cents.height,
            60,
            exit,
        )?;
        compute_inner_percentile(
            &mut self.pct70.cents.height,
            starting_height,
            &self.pct5.cents.height,
            &self.pct95.cents.height,
            70,
            exit,
        )?;
        compute_inner_percentile(
            &mut self.pct80.cents.height,
            starting_height,
            &self.pct5.cents.height,
            &self.pct95.cents.height,
            80,
            exit,
        )?;
        compute_inner_percentile(
            &mut self.pct90.cents.height,
            starting_height,
            &self.pct5.cents.height,
            &self.pct95.cents.height,
            90,
            exit,
        )?;

        self.compute_index(spot, indexer, exit)?;

        self.compute_score(components, spot, indexer, exit)?;

        Ok(())
    }

    fn compute_index(
        &mut self,
        spot: &impl ReadableVec<Height, Cents>,
        indexer: &Indexer,
        exit: &Exit,
    ) -> Result<()> {
        let starting_height = indexer.safe_lengths().height;
        let bands = [
            &self.pct0_01.cents.height,
            &self.pct0_5.cents.height,
            &self.pct1.cents.height,
            &self.pct2.cents.height,
            &self.pct5.cents.height,
            &self.pct95.cents.height,
            &self.pct98.cents.height,
            &self.pct99.cents.height,
            &self.pct99_5.cents.height,
            &self.pct99_9.cents.height,
        ];

        let dep_version: Version =
            bands.iter().map(|b| b.version()).sum::<Version>() + spot.version();

        self.index
            .height
            .validate_computed_version_or_reset(dep_version)?;
        self.index.height.truncate_if_needed(starting_height)?;

        self.index.height.repeat_until_complete(exit, |vec| {
            let skip = vec.len();
            let source_end = bands.iter().map(|b| b.len()).min().unwrap().min(spot.len());
            let end = vec.batch_end(source_end);

            if skip >= end {
                return Ok(());
            }

            let spot_batch = spot.collect_range_at(skip, end);
            let b: [Vec<Cents>; 10] = bands.each_ref().map(|v| v.collect_range_at(skip, end));

            for j in 0..(end - skip) {
                vec.push(StoredI8::new(score_at(spot_batch[j], &b, j)));
            }

            Ok(())
        })?;

        Ok(())
    }

    fn compute_score(
        &mut self,
        components: &[&Component],
        spot: &impl ReadableVec<Height, Cents>,
        indexer: &Indexer,
        exit: &Exit,
    ) -> Result<()> {
        let starting_height = indexer.safe_lengths().height;
        let dep_version: Version = components
            .iter()
            .map(|component| {
                component.pct0_01.price.cents.height.version()
                    + component.pct0_5.price.cents.height.version()
                    + component.pct1.price.cents.height.version()
                    + component.pct2.price.cents.height.version()
                    + component.pct5.price.cents.height.version()
                    + component.pct95.price.cents.height.version()
                    + component.pct98.price.cents.height.version()
                    + component.pct99.price.cents.height.version()
                    + component.pct99_5.price.cents.height.version()
                    + component.pct99_9.price.cents.height.version()
            })
            .sum::<Version>()
            + spot.version();

        self.score
            .height
            .validate_computed_version_or_reset(dep_version)?;
        self.score.height.truncate_if_needed(starting_height)?;

        self.score.height.repeat_until_complete(exit, |vec| {
            let skip = vec.len();
            let source_end = components
                .iter()
                .flat_map(|component| {
                    [
                        component.pct0_01.price.cents.height.len(),
                        component.pct0_5.price.cents.height.len(),
                        component.pct1.price.cents.height.len(),
                        component.pct2.price.cents.height.len(),
                        component.pct5.price.cents.height.len(),
                        component.pct95.price.cents.height.len(),
                        component.pct98.price.cents.height.len(),
                        component.pct99.price.cents.height.len(),
                        component.pct99_5.price.cents.height.len(),
                        component.pct99_9.price.cents.height.len(),
                    ]
                })
                .min()
                .unwrap()
                .min(spot.len());
            let end = vec.batch_end(source_end);

            if skip >= end {
                return Ok(());
            }

            let spot_batch = spot.collect_range_at(skip, end);

            let bands: Vec<[Vec<Cents>; 10]> = components
                .iter()
                .map(|component| {
                    [
                        component
                            .pct0_01
                            .price
                            .cents
                            .height
                            .collect_range_at(skip, end),
                        component
                            .pct0_5
                            .price
                            .cents
                            .height
                            .collect_range_at(skip, end),
                        component
                            .pct1
                            .price
                            .cents
                            .height
                            .collect_range_at(skip, end),
                        component
                            .pct2
                            .price
                            .cents
                            .height
                            .collect_range_at(skip, end),
                        component
                            .pct5
                            .price
                            .cents
                            .height
                            .collect_range_at(skip, end),
                        component
                            .pct95
                            .price
                            .cents
                            .height
                            .collect_range_at(skip, end),
                        component
                            .pct98
                            .price
                            .cents
                            .height
                            .collect_range_at(skip, end),
                        component
                            .pct99
                            .price
                            .cents
                            .height
                            .collect_range_at(skip, end),
                        component
                            .pct99_5
                            .price
                            .cents
                            .height
                            .collect_range_at(skip, end),
                        component
                            .pct99_9
                            .price
                            .cents
                            .height
                            .collect_range_at(skip, end),
                    ]
                })
                .collect();

            for j in 0..(end - skip) {
                let price = spot_batch[j];
                let mut total: i8 = 0;

                for component in &bands {
                    total += score_at(price, component, j);
                }

                vec.push(StoredI8::new(total));
            }

            Ok(())
        })?;

        Ok(())
    }
}

fn compute_inner_percentile(
    out: &mut EagerVec<PcoVec<Height, Cents>>,
    max_from: Height,
    lower: &EagerVec<PcoVec<Height, Cents>>,
    upper: &EagerVec<PcoVec<Height, Cents>>,
    percentile: u8,
    exit: &Exit,
) -> Result<()> {
    debug_assert!((5..=95).contains(&percentile));
    let position = (f64::from(percentile) - 5.0) / 90.0;

    out.validate_and_truncate(lower.version() + upper.version(), max_from)?;

    out.repeat_until_complete(exit, |vec| {
        let skip = vec.len();
        let source_end = lower.len().min(upper.len());
        let end = vec.batch_end(source_end);
        if skip >= end {
            return Ok(());
        }

        let lower_batch = lower.collect_range_at(skip, end);
        let upper_batch = upper.collect_range_at(skip, end);
        for j in 0..(end - skip) {
            let lower = f64::from(lower_batch[j]);
            let upper = f64::from(upper_batch[j]);
            let value = if lower > 0.0 && upper > 0.0 {
                (lower.ln() + position * (upper.ln() - lower.ln())).exp()
            } else {
                lower + position * (upper - lower)
            };
            vec.push(Cents::from(value.round()));
        }

        Ok(())
    })?;

    Ok(())
}

fn score_at(price: Cents, bands: &[Vec<Cents>; 10], index: usize) -> i8 {
    let lower = bands[..5].iter().filter(|band| price < band[index]).count() as i8;
    let upper = bands[5..].iter().filter(|band| price > band[index]).count() as i8;

    upper - lower
}
