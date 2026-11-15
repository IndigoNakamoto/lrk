use bitview_plugin::{ComputePlugin, UpdateContext, compute_items};
use brk_error::Result;
use rayon::join;

use super::{Vecs, policy};
use crate::Dependencies;

impl ComputePlugin for Vecs {
    compute_items!(Dependencies, db);

    fn compute_state(
        &mut self,
        dependencies: Self::Dependencies<'_>,
        context: UpdateContext<'_>,
    ) -> Result<()> {
        let Dependencies {
            indexer,
            mappings,
            price,
        } = dependencies;
        let Vecs {
            count,
            features,
            vsize,
            fees,
            inscription,
            patterns,
            nonstandard,
            versions,
            transfer_volume,
            #[cfg(feature = "litecoin")]
            transparent_count_cumulative,
            ..
        } = self;
        let exit = context.exit();
        let start = indexer.compute_start().height;
        let price = &price.spot.cents.height;

        let (independent, fee_based) = join(
            // Stages reading only indexer and mappings columns.
            || {
                let ((count, features), (versions, patterns)) = join(
                    || {
                        join(
                            || {
                                #[cfg(feature = "litecoin")]
                                super::transparent::compute(
                                    transparent_count_cumulative,
                                    indexer,
                                    mappings,
                                    exit,
                                )?;
                                count.compute(start, &mappings.lookback.window_starts(), exit)
                            },
                            || features.compute(indexer, exit),
                        )
                    },
                    || {
                        join(
                            || versions.compute(indexer, exit),
                            || patterns.compute(indexer, mappings, exit),
                        )
                    },
                );
                count.and(features).and(versions).and(patterns)
            },
            // The fee pass (with the vsize distribution), then the stages reading its fees.
            || {
                fees.compute(
                    indexer,
                    mappings,
                    &mut vsize.distribution,
                    &mut transfer_volume.cumulative.sats.height,
                    exit,
                )?;
                let (inscription, (policy, volume)) = join(
                    || {
                        inscription.compute(indexer, mappings, fees, exit)?;
                        inscription.fees.compute_cents(start, price, exit)
                    },
                    || {
                        join(
                            || policy::compute(nonstandard, indexer, mappings, fees, exit),
                            || transfer_volume.compute_cents(start, price, exit),
                        )
                    },
                );
                inscription.and(policy).and(volume)
            },
        );
        independent.and(fee_based)
    }
}
