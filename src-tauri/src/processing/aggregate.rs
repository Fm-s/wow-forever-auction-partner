use crate::domain::Listing;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub struct ItemAggregate {
    pub item_id: i64,
    pub variant_key: String,
    pub min_price: Option<i64>,
    pub median: Option<i64>,
    pub p25: Option<i64>,
    pub p75: Option<i64>,
    pub quantity: i64,
    pub listings: i64,
}

/// Nearest-rank percentile over quantity-weighted units; `sorted` is (unit_price, quantity) ascending.
fn percentile(sorted: &[(i64, i64)], total_units: i64, p: f64) -> Option<i64> {
    if total_units == 0 {
        return None;
    }
    let k = ((p * total_units as f64).ceil() as i64 - 1).max(0);
    let mut seen = 0;
    for &(price, qty) in sorted {
        seen += qty;
        if seen > k {
            return Some(price);
        }
    }
    sorted.last().map(|x| x.0)
}

/// Per (item_id, variant_key) stats for one snapshot, sorted by key.
/// Bid-only listings count toward quantity/listings but not prices.
pub fn aggregate(listings: &[Listing]) -> Vec<ItemAggregate> {
    // (priced (unit_price, quantity) pairs, total quantity, listing count)
    type Group = (Vec<(i64, i64)>, i64, i64);
    let mut groups: BTreeMap<(i64, &str), Group> = BTreeMap::new();
    for l in listings {
        let g = groups.entry((l.item_id, l.variant_key.as_str())).or_default();
        if let Some(p) = l.unit_price {
            g.0.push((p, l.quantity));
        }
        g.1 += l.quantity;
        g.2 += 1;
    }
    groups
        .into_iter()
        .map(|((item_id, variant), (mut priced, quantity, listings))| {
            priced.sort_unstable();
            let units: i64 = priced.iter().map(|x| x.1).sum();
            ItemAggregate {
                item_id,
                variant_key: variant.to_string(),
                min_price: priced.first().map(|x| x.0),
                median: percentile(&priced, units, 0.5),
                p25: percentile(&priced, units, 0.25),
                p75: percentile(&priced, units, 0.75),
                quantity,
                listings,
            }
        })
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::domain::{Listing, TimeLeft};

    pub(crate) fn l(auction_id: i64, item_id: i64, variant: &str, unit_price: Option<i64>, qty: i64) -> Listing {
        Listing { auction_id, item_id, variant_key: variant.into(), unit_price, quantity: qty, time_left: TimeLeft::VeryLong }
    }

    #[test]
    fn weighted_percentiles() {
        let a = aggregate(&[l(1, 5, "", Some(100), 1), l(2, 5, "", Some(200), 3)]);
        assert_eq!((a[0].min_price, a[0].p25, a[0].median, a[0].p75), (Some(100), Some(100), Some(200), Some(200)));
        assert_eq!((a[0].quantity, a[0].listings), (4, 2));
    }

    #[test]
    fn bid_only_counts_but_no_price() {
        let a = aggregate(&[l(1, 5, "", None, 2)]);
        assert_eq!((a[0].min_price, a[0].median, a[0].quantity, a[0].listings), (None, None, 2, 1));
    }

    #[test]
    fn variants_aggregate_separately_and_sorted() {
        let a = aggregate(&[l(2, 727, "12721", Some(7300), 1), l(1, 727, "12718", Some(257), 1), l(3, 5, "", Some(9), 1)]);
        let keys: Vec<_> = a.iter().map(|x| (x.item_id, x.variant_key.as_str())).collect();
        assert_eq!(keys, vec![(5, ""), (727, "12718"), (727, "12721")]);
    }

    #[test]
    fn empty_input_gives_empty_output() {
        assert!(aggregate(&[]).is_empty());
    }
}
