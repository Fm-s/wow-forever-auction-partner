use crate::domain::{Listing, GAP_FACTOR};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, PartialEq)]
pub struct VolumeRow {
    pub item_id: i64,
    pub variant_key: String,
    pub est_sold: i64,
    pub expired_unknown: i64,
}

/// Δt between consecutive snapshots larger than GAP_FACTOR x cadence.
pub fn is_gap(dt_secs: i64, cadence_secs: i64) -> bool {
    dt_secs as f64 > GAP_FACTOR * cadence_secs as f64
}

/// Infer sales between two snapshots of one feed, matching listings by auction id.
/// Vanished listings count as sold only when they could not have expired within `dt_secs`;
/// a quantity decrease on a surviving listing counts the difference as sold.
pub fn diff(prev: &[Listing], next: &[Listing], dt_secs: i64) -> Vec<VolumeRow> {
    let next_qty: HashMap<i64, i64> = next.iter().map(|l| (l.auction_id, l.quantity)).collect();
    let mut acc: BTreeMap<(i64, &str), (i64, i64)> = BTreeMap::new();
    for l in prev {
        let (sold, expired) = match next_qty.get(&l.auction_id) {
            None if l.time_left.min_remaining_secs() > dt_secs => (l.quantity, 0),
            None => (0, l.quantity),
            Some(&q) if q < l.quantity => (l.quantity - q, 0),
            Some(_) => (0, 0),
        };
        if sold != 0 || expired != 0 {
            let e = acc.entry((l.item_id, l.variant_key.as_str())).or_default();
            e.0 += sold;
            e.1 += expired;
        }
    }
    acc.into_iter()
        .map(|((item_id, v), (est_sold, expired_unknown))| VolumeRow { item_id, variant_key: v.to_string(), est_sold, expired_unknown })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Listing, TimeLeft};

    fn lt(auction_id: i64, item_id: i64, qty: i64, t: TimeLeft) -> Listing {
        Listing { auction_id, item_id, variant_key: String::new(), unit_price: Some(100), quantity: qty, time_left: t }
    }

    #[test]
    fn hourly_diff_classifies_by_time_left() {
        let prev = vec![lt(1, 5, 2, TimeLeft::Long), lt(2, 5, 3, TimeLeft::VeryLong), lt(3, 5, 4, TimeLeft::Medium), lt(4, 5, 5, TimeLeft::Short)];
        let rows = diff(&prev, &[], 3600);
        assert_eq!(rows, vec![VolumeRow { item_id: 5, variant_key: String::new(), est_sold: 5, expired_unknown: 9 }]);
    }

    #[test]
    fn short_dt_counts_medium_as_sold() {
        let rows = diff(&[lt(1, 5, 2, TimeLeft::Medium)], &[], 1200);
        assert_eq!((rows[0].est_sold, rows[0].expired_unknown), (2, 0));
    }

    #[test]
    fn commodity_partial_buy_counts_decrease() {
        let rows = diff(&[lt(1, 5, 10, TimeLeft::Long)], &[lt(1, 5, 4, TimeLeft::Long)], 3600);
        assert_eq!((rows[0].est_sold, rows[0].expired_unknown), (6, 0));
    }

    #[test]
    fn new_listings_and_increases_are_ignored() {
        let rows = diff(&[lt(1, 5, 1, TimeLeft::Long)], &[lt(1, 5, 3, TimeLeft::Long), lt(2, 6, 9, TimeLeft::Long)], 3600);
        assert!(rows.is_empty());
    }

    #[test]
    fn rows_are_per_item_and_variant() {
        let mut a = lt(1, 727, 1, TimeLeft::Long);
        a.variant_key = "12718".into();
        let mut b = lt(2, 727, 1, TimeLeft::Long);
        b.variant_key = "12721".into();
        assert_eq!(diff(&[a, b], &[], 3600).len(), 2);
    }

    #[test]
    fn gap_threshold() {
        assert!(!is_gap(5400, 3600));
        assert!(is_gap(5401, 3600));
    }
}
