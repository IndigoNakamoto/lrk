use brk_error::Result;
use brk_indexer::{Indexer, Lengths};
use brk_traversable::Traversable;
use brk_types::{Cents, Height, PartsPerMillion32, Version};
use vecdb::{
    AnyStoredVec, AnyVec, Database, EagerVec, Exit, PcoVec, ReadableVec, Rw, StorageMode, VecIndex,
    WritableVec,
};

use crate::{
    distribution,
    frameworks::{coinflow, cointime},
    indexes,
    internal::{PerBlock, Price, PriceTimesRatio, PriceWithRatioPerBlock, RatioPerBlock},
};

use super::percentiles::{BlockDecayPercentiles, START_HEIGHT};

#[derive(Traversable)]
pub struct Band<M: StorageMode = Rw> {
    #[traversable(flatten)]
    pub ratio: RatioPerBlock<PartsPerMillion32, M>,
    pub price: Price<PerBlock<Cents, M>>,
}

#[derive(Traversable)]
pub struct Component<M: StorageMode = Rw> {
    pub pct0_01: Band<M>,
    pub pct0_5: Band<M>,
    pub pct1: Band<M>,
    pub pct2: Band<M>,
    pub pct5: Band<M>,
    pub pct10: Band<M>,
    pub pct20: Band<M>,
    pub pct30: Band<M>,
    pub pct40: Band<M>,
    pub pct50: Band<M>,
    pub pct60: Band<M>,
    pub pct70: Band<M>,
    pub pct80: Band<M>,
    pub pct90: Band<M>,
    pub pct95: Band<M>,
    pub pct98: Band<M>,
    pub pct99: Band<M>,
    pub pct99_5: Band<M>,
    pub pct99_9: Band<M>,

    #[traversable(skip)]
    block_decay_pct: BlockDecayPercentiles,
}

const VERSION: Version = Version::new(8);

impl Component {
    fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        let version = version + VERSION;

        macro_rules! import_ratio {
            ($suffix:expr) => {
                RatioPerBlock::forced_import_ppm(
                    db,
                    &format!("{name}_{}", $suffix),
                    version,
                    indexes,
                )?
            };
        }

        macro_rules! import_price {
            ($suffix:expr) => {
                Price::forced_import(db, &format!("{name}_{}", $suffix), version, indexes)?
            };
        }

        macro_rules! import_band {
            ($pct:expr) => {
                Band {
                    ratio: import_ratio!(concat!("ratio_", $pct)),
                    price: import_price!($pct),
                }
            };
        }

        Ok(Self {
            pct0_01: import_band!("pct0_01"),
            pct0_5: import_band!("pct0_5"),
            pct1: import_band!("pct1"),
            pct2: import_band!("pct2"),
            pct5: import_band!("pct5"),
            pct10: import_band!("pct10"),
            pct20: import_band!("pct20"),
            pct30: import_band!("pct30"),
            pct40: import_band!("pct40"),
            pct50: import_band!("pct50"),
            pct60: import_band!("pct60"),
            pct70: import_band!("pct70"),
            pct80: import_band!("pct80"),
            pct90: import_band!("pct90"),
            pct95: import_band!("pct95"),
            pct98: import_band!("pct98"),
            pct99: import_band!("pct99"),
            pct99_5: import_band!("pct99_5"),
            pct99_9: import_band!("pct99_9"),
            block_decay_pct: BlockDecayPercentiles::default(),
        })
    }

    fn compute(
        &mut self,
        starting_lengths: &Lengths,
        source: &PriceWithRatioPerBlock,
        exit: &Exit,
    ) -> Result<()> {
        let ratio_source = &source.ratio.height;
        let series_price = &source.cents.height;
        let ratio_version = ratio_source.version();

        self.mut_pct_vecs().try_for_each(|vec| -> Result<()> {
            vec.validate_computed_version_or_reset(ratio_version)?;
            Ok(())
        })?;

        let starting_height = self
            .mut_pct_vecs()
            .map(|vec| Height::from(vec.len()))
            .min()
            .unwrap()
            .min(starting_lengths.height);

        let start = starting_height.to_usize();
        let ratio_len = ratio_source.len();

        if ratio_len > start {
            let expected_len = start.saturating_sub(START_HEIGHT);
            if self.block_decay_pct.len() != expected_len {
                self.block_decay_pct.reset();
                if start > START_HEIGHT {
                    let historical = ratio_source.collect_range_at(START_HEIGHT, start);
                    self.block_decay_pct.add_bulk(START_HEIGHT, &historical);
                }
            }

            let new_ratios = ratio_source.collect_range_at(start, ratio_len);
            let mut pct_vecs: [&mut EagerVec<PcoVec<Height, PartsPerMillion32>>; 19] = [
                &mut self.pct0_01.ratio.ppm.height,
                &mut self.pct0_5.ratio.ppm.height,
                &mut self.pct1.ratio.ppm.height,
                &mut self.pct2.ratio.ppm.height,
                &mut self.pct5.ratio.ppm.height,
                &mut self.pct10.ratio.ppm.height,
                &mut self.pct20.ratio.ppm.height,
                &mut self.pct30.ratio.ppm.height,
                &mut self.pct40.ratio.ppm.height,
                &mut self.pct50.ratio.ppm.height,
                &mut self.pct60.ratio.ppm.height,
                &mut self.pct70.ratio.ppm.height,
                &mut self.pct80.ratio.ppm.height,
                &mut self.pct90.ratio.ppm.height,
                &mut self.pct95.ratio.ppm.height,
                &mut self.pct98.ratio.ppm.height,
                &mut self.pct99.ratio.ppm.height,
                &mut self.pct99_5.ratio.ppm.height,
                &mut self.pct99_9.ratio.ppm.height,
            ];
            const PCTS: [f64; 19] = [
                0.0001, 0.005, 0.01, 0.02, 0.05, 0.10, 0.20, 0.30, 0.40, 0.50, 0.60, 0.70, 0.80,
                0.90, 0.95, 0.98, 0.99, 0.995, 0.999,
            ];
            let mut out = [0.0; 19];

            for vec in &mut pct_vecs {
                vec.truncate_if_needed_at(start)?;
            }

            for (offset, &ratio) in new_ratios.iter().enumerate() {
                let height = start + offset;
                if height >= START_HEIGHT {
                    self.block_decay_pct.add(height, *ratio);
                }
                self.block_decay_pct.quantiles(&PCTS, &mut out);
                for (vec, &value) in pct_vecs.iter_mut().zip(&out) {
                    vec.push(PartsPerMillion32::from(value));
                }
            }
        }

        {
            let _lock = exit.lock();
            self.mut_pct_vecs()
                .try_for_each(|vec| vec.write().map(|_| ()))?;
        }

        macro_rules! compute_price {
            ($band:ident) => {
                self.$band
                    .price
                    .cents
                    .compute_binary::<Cents, PartsPerMillion32, PriceTimesRatio<PartsPerMillion32>>(
                        starting_lengths.height,
                        series_price,
                        &self.$band.ratio.ppm.height,
                        exit,
                    )?;
            };
        }

        compute_price!(pct0_01);
        compute_price!(pct0_5);
        compute_price!(pct1);
        compute_price!(pct2);
        compute_price!(pct5);
        compute_price!(pct10);
        compute_price!(pct20);
        compute_price!(pct30);
        compute_price!(pct40);
        compute_price!(pct50);
        compute_price!(pct60);
        compute_price!(pct70);
        compute_price!(pct80);
        compute_price!(pct90);
        compute_price!(pct95);
        compute_price!(pct98);
        compute_price!(pct99);
        compute_price!(pct99_5);
        compute_price!(pct99_9);

        Ok(())
    }

    fn mut_pct_vecs(
        &mut self,
    ) -> impl Iterator<Item = &mut EagerVec<PcoVec<Height, PartsPerMillion32>>> {
        [
            &mut self.pct0_01.ratio.ppm.height,
            &mut self.pct0_5.ratio.ppm.height,
            &mut self.pct1.ratio.ppm.height,
            &mut self.pct2.ratio.ppm.height,
            &mut self.pct5.ratio.ppm.height,
            &mut self.pct10.ratio.ppm.height,
            &mut self.pct20.ratio.ppm.height,
            &mut self.pct30.ratio.ppm.height,
            &mut self.pct40.ratio.ppm.height,
            &mut self.pct50.ratio.ppm.height,
            &mut self.pct60.ratio.ppm.height,
            &mut self.pct70.ratio.ppm.height,
            &mut self.pct80.ratio.ppm.height,
            &mut self.pct90.ratio.ppm.height,
            &mut self.pct95.ratio.ppm.height,
            &mut self.pct98.ratio.ppm.height,
            &mut self.pct99.ratio.ppm.height,
            &mut self.pct99_5.ratio.ppm.height,
            &mut self.pct99_9.ratio.ppm.height,
        ]
        .into_iter()
    }
}

#[derive(Traversable)]
pub struct Components<M: StorageMode = Rw> {
    pub realized_price: Component<M>,
    pub capitalized_price: Component<M>,
    pub sth_realized_price: Component<M>,
    pub sth_capitalized_price: Component<M>,
    pub lth_realized_price: Component<M>,
    pub lth_capitalized_price: Component<M>,
    pub over_6m_realized_price: Component<M>,
    pub over_4m_realized_price: Component<M>,
    pub under_4m_realized_price: Component<M>,
    pub under_6m_realized_price: Component<M>,
    pub vaulted_price: Component<M>,
    pub active_price: Component<M>,
    pub true_market_mean_price: Component<M>,
    pub cointime_price: Component<M>,
    pub coinflow_price: Component<M>,
}

impl Components {
    pub(super) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        let import = |name| Component::forced_import(db, name, version, indexes);

        Ok(Self {
            realized_price: import("realized_price")?,
            capitalized_price: import("capitalized_price")?,
            sth_realized_price: import("sth_realized_price")?,
            sth_capitalized_price: import("sth_capitalized_price")?,
            lth_realized_price: import("lth_realized_price")?,
            lth_capitalized_price: import("lth_capitalized_price")?,
            over_6m_realized_price: import("over_6m_realized_price")?,
            over_4m_realized_price: import("over_4m_realized_price")?,
            under_4m_realized_price: import("under_4m_realized_price")?,
            under_6m_realized_price: import("under_6m_realized_price")?,
            vaulted_price: import("vaulted_price")?,
            active_price: import("active_price")?,
            true_market_mean_price: import("true_market_mean_price")?,
            cointime_price: import("cointime_price")?,
            coinflow_price: import("coinflow_price")?,
        })
    }

    pub(super) fn compute(
        &mut self,
        indexer: &Indexer,
        distribution: &distribution::Vecs,
        cointime: &cointime::Vecs,
        coinflow: &coinflow::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_lengths = indexer.safe_lengths();
        let utxos = &distribution.utxo_cohorts;
        let all = &utxos.all.metrics.realized;
        let sth = &utxos.sth.metrics.realized;
        let lth = &utxos.lth.metrics.realized;

        macro_rules! compute {
            ($component:ident, $source:expr) => {
                self.$component.compute(&starting_lengths, $source, exit)?;
            };
        }

        compute!(realized_price, &all.price);
        compute!(capitalized_price, &all.capitalized.price);
        compute!(sth_realized_price, &sth.price);
        compute!(sth_capitalized_price, &sth.capitalized.price);
        compute!(lth_realized_price, &lth.price);
        compute!(lth_capitalized_price, &lth.capitalized.price);
        compute!(
            over_6m_realized_price,
            &utxos.over_age._6m.metrics.realized.price
        );
        compute!(
            over_4m_realized_price,
            &utxos.over_age._4m.metrics.realized.price
        );
        compute!(
            under_4m_realized_price,
            &utxos.under_age._4m.metrics.realized.price
        );
        compute!(
            under_6m_realized_price,
            &utxos.under_age._6m.metrics.realized.price
        );
        compute!(vaulted_price, &cointime.prices.vaulted);
        compute!(active_price, &cointime.prices.active);
        compute!(true_market_mean_price, &cointime.prices.true_market_mean);
        compute!(cointime_price, &cointime.prices.cointime);
        compute!(coinflow_price, &coinflow.price);

        Ok(())
    }
}
