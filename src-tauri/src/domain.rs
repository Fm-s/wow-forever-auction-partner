use serde::{Deserialize, Serialize};

/// Gap: consecutive feed snapshots further apart than GAP_FACTOR x cadence.
pub const GAP_FACTOR: f64 = 1.5;
/// Stale: a feed whose last_modified is older than STALE_FACTOR x cadence.
pub const STALE_FACTOR: i64 = 2;

/// The three WoW Forever PvP US auction houses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum House {
    Alliance,
    Horde,
    Blackwater,
}

#[allow(clippy::should_implement_trait)]
impl House {
    pub const ALL: [House; 3] = [House::Alliance, House::Horde, House::Blackwater];

    /// Blizzard auction house id under `/data/wow/ruleset/1/auctions/{id}`.
    pub fn auction_house_id(self) -> u32 {
        match self {
            House::Alliance => 110,
            House::Horde => 113,
            House::Blackwater => 116,
        }
    }

    /// AHledger market id; Blackwater has none.
    pub fn ahledger_market(self) -> Option<&'static str> {
        match self {
            House::Alliance => Some("forever.pvp.alliance.us"),
            House::Horde => Some("forever.pvp.horde.us"),
            House::Blackwater => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            House::Alliance => "alliance",
            House::Horde => "horde",
            House::Blackwater => "blackwater",
        }
    }

    pub fn from_str(s: &str) -> Option<House> {
        House::ALL.into_iter().find(|h| h.as_str() == s)
    }
}

/// The two Blizzard feeds of one house.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Feed {
    Items,
    Commodities,
}

#[allow(clippy::should_implement_trait)]
impl Feed {
    pub const ALL: [Feed; 2] = [Feed::Items, Feed::Commodities];

    pub fn as_str(self) -> &'static str {
        match self {
            Feed::Items => "items",
            Feed::Commodities => "commodities",
        }
    }

    pub fn from_str(s: &str) -> Option<Feed> {
        Feed::ALL.into_iter().find(|f| f.as_str() == s)
    }
}

/// Blizzard `time_left` bucket of a listing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimeLeft {
    Short,
    Medium,
    Long,
    VeryLong,
}

#[allow(clippy::should_implement_trait)]
impl TimeLeft {
    /// Lower bound of the bucket's remaining time, in seconds.
    pub fn min_remaining_secs(self) -> i64 {
        match self {
            TimeLeft::Short => 0,
            TimeLeft::Medium => 1800,
            TimeLeft::Long => 7200,
            TimeLeft::VeryLong => 43200,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            TimeLeft::Short => "SHORT",
            TimeLeft::Medium => "MEDIUM",
            TimeLeft::Long => "LONG",
            TimeLeft::VeryLong => "VERY_LONG",
        }
    }

    pub fn from_str(s: &str) -> Option<TimeLeft> {
        [TimeLeft::Short, TimeLeft::Medium, TimeLeft::Long, TimeLeft::VeryLong]
            .into_iter()
            .find(|t| t.as_str() == s)
    }
}

/// One auction listing, normalized from either feed. Prices are copper per unit.
#[derive(Clone, Debug, PartialEq)]
pub struct Listing {
    pub auction_id: i64,
    pub item_id: i64,
    pub variant_key: String,
    /// None for bid-only auctions.
    pub unit_price: Option<i64>,
    pub quantity: i64,
    pub time_left: TimeLeft,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn house_ids() {
        assert_eq!(House::Alliance.auction_house_id(), 110);
        assert_eq!(House::Horde.auction_house_id(), 113);
        assert_eq!(House::Blackwater.auction_house_id(), 116);
    }

    #[test]
    fn ahledger_markets() {
        assert_eq!(House::Alliance.ahledger_market(), Some("forever.pvp.alliance.us"));
        assert_eq!(House::Horde.ahledger_market(), Some("forever.pvp.horde.us"));
        assert_eq!(House::Blackwater.ahledger_market(), None);
    }

    #[test]
    fn time_left_minimums() {
        assert_eq!(TimeLeft::Short.min_remaining_secs(), 0);
        assert_eq!(TimeLeft::Medium.min_remaining_secs(), 1800);
        assert_eq!(TimeLeft::Long.min_remaining_secs(), 7200);
        assert_eq!(TimeLeft::VeryLong.min_remaining_secs(), 43200);
    }

    #[test]
    fn time_left_parses_api_strings() {
        assert_eq!(TimeLeft::from_str("VERY_LONG"), Some(TimeLeft::VeryLong));
        assert_eq!(TimeLeft::from_str("bogus"), None);
    }

    #[test]
    fn house_and_feed_round_trip_strings() {
        for h in House::ALL {
            assert_eq!(House::from_str(h.as_str()), Some(h));
        }
        assert_eq!(Feed::Commodities.as_str(), "commodities");
        assert_eq!(Feed::from_str("items"), Some(Feed::Items));
    }
}
