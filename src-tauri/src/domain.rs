use serde::{Deserialize, Serialize};

/// Gap: consecutive feed snapshots further apart than GAP_FACTOR x cadence.
pub const GAP_FACTOR: f64 = 1.5;
/// Stale: a feed whose last_modified is older than STALE_FACTOR x cadence.
pub const STALE_FACTOR: i64 = 2;

/// Blizzard API region.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Region {
    #[default]
    Us,
    Eu,
}

impl Region {
    pub const ALL: [Region; 2] = [Region::Us, Region::Eu];

    pub fn as_str(self) -> &'static str {
        match self {
            Region::Us => "us",
            Region::Eu => "eu",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Region::Us => "US",
            Region::Eu => "EU",
        }
    }
}

/// WoW Forever realm type (Blizzard "ruleset"). `as_str` matches AHledger's market ids.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RealmType {
    #[default]
    Pvp,
    Normal,
    Rp,
    Rppvp,
}

impl RealmType {
    pub const ALL: [RealmType; 4] = [RealmType::Pvp, RealmType::Normal, RealmType::Rp, RealmType::Rppvp];

    pub fn as_str(self) -> &'static str {
        match self {
            RealmType::Pvp => "pvp",
            RealmType::Normal => "normal",
            RealmType::Rp => "rp",
            RealmType::Rppvp => "rppvp",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            RealmType::Pvp => "PvP",
            RealmType::Normal => "Normal",
            RealmType::Rp => "Roleplay",
            RealmType::Rppvp => "Roleplay PvP",
        }
    }

    /// Auction house ids (Alliance, Horde, Blackwater); the same in US and EU (verified 2026-11-15).
    fn house_ids(self) -> [u32; 3] {
        match self {
            RealmType::Pvp => [110, 113, 116],
            RealmType::Normal => [111, 114, 117],
            RealmType::Rp => [119, 120, 121],
            RealmType::Rppvp => [123, 124, 125],
        }
    }
}

/// Auction house side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Faction {
    Alliance,
    Horde,
    Blackwater,
}

#[allow(clippy::should_implement_trait)]
impl Faction {
    pub const ALL: [Faction; 3] = [Faction::Alliance, Faction::Horde, Faction::Blackwater];

    pub fn as_str(self) -> &'static str {
        match self {
            Faction::Alliance => "alliance",
            Faction::Horde => "horde",
            Faction::Blackwater => "blackwater",
        }
    }

    pub fn from_str(s: &str) -> Option<Faction> {
        Faction::ALL.into_iter().find(|f| f.as_str() == s)
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// The realm being tracked: region + realm type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Realm {
    pub region: Region,
    pub kind: RealmType,
}

impl Realm {
    pub fn house(self, faction: Faction) -> House {
        House { realm: self, faction }
    }

    /// The realm's three auction houses.
    pub fn houses(self) -> [House; 3] {
        Faction::ALL.map(|f| self.house(f))
    }

    /// e.g. "PvP · US"
    pub fn label(self) -> String {
        format!("{} · {}", self.kind.label(), self.region.label())
    }
}

/// One auction house: a realm's faction. The UI sees only the faction (it serializes as
/// "alliance" etc.); storage uses the region-qualified [`House::key`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct House {
    pub realm: Realm,
    pub faction: Faction,
}

impl House {
    /// Blizzard auction house id under `/data/wow/ruleset/{any}/auctions/{id}`.
    pub fn auction_house_id(self) -> u32 {
        self.realm.kind.house_ids()[self.faction.index()]
    }

    /// AHledger market id, e.g. `forever.rppvp.horde.eu`; Blackwater has none.
    pub fn ahledger_market(self) -> Option<String> {
        (self.faction != Faction::Blackwater)
            .then(|| format!("forever.{}.{}.{}", self.realm.kind.as_str(), self.faction.as_str(), self.realm.region.as_str()))
    }

    /// Storage key, e.g. `us-110`.
    pub fn key(self) -> String {
        format!("{}-{}", self.realm.region.as_str(), self.auction_house_id())
    }

    pub fn from_key(key: &str) -> Option<House> {
        let (region, id) = key.split_once('-')?;
        let region = Region::ALL.into_iter().find(|r| r.as_str() == region)?;
        let id: u32 = id.parse().ok()?;
        RealmType::ALL
            .into_iter()
            .flat_map(|kind| Realm { region, kind }.houses())
            .find(|h| h.auction_house_id() == id)
    }
}

impl Serialize for House {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.faction.serialize(s)
    }
}

/// PvP US shorthands for tests only, so production code can't silently assume a realm.
#[cfg(test)]
#[allow(non_upper_case_globals)]
impl House {
    pub const Alliance: House = House { realm: Realm { region: Region::Us, kind: RealmType::Pvp }, faction: Faction::Alliance };
    pub const Horde: House = House { realm: Realm { region: Region::Us, kind: RealmType::Pvp }, faction: Faction::Horde };
    pub const Blackwater: House = House { realm: Realm { region: Region::Us, kind: RealmType::Pvp }, faction: Faction::Blackwater };
    pub const ALL: [House; 3] = [House::Alliance, House::Horde, House::Blackwater];
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

    fn realm(region: Region, kind: RealmType) -> Realm {
        Realm { region, kind }
    }

    #[test]
    fn house_ids_match_verified_table() {
        for region in Region::ALL {
            let ids = |kind| realm(region, kind).houses().map(|h| h.auction_house_id());
            assert_eq!(ids(RealmType::Pvp), [110, 113, 116]);
            assert_eq!(ids(RealmType::Normal), [111, 114, 117]);
            assert_eq!(ids(RealmType::Rp), [119, 120, 121]);
            assert_eq!(ids(RealmType::Rppvp), [123, 124, 125]);
        }
    }

    #[test]
    fn ahledger_markets() {
        assert_eq!(House::Alliance.ahledger_market().as_deref(), Some("forever.pvp.alliance.us"));
        assert_eq!(realm(Region::Eu, RealmType::Rppvp).house(Faction::Horde).ahledger_market().as_deref(), Some("forever.rppvp.horde.eu"));
        assert_eq!(realm(Region::Us, RealmType::Normal).house(Faction::Blackwater).ahledger_market(), None);
    }

    #[test]
    fn keys_are_region_qualified_and_round_trip() {
        assert_eq!(House::Alliance.key(), "us-110");
        assert_eq!(realm(Region::Eu, RealmType::Rppvp).house(Faction::Horde).key(), "eu-124");
        for region in Region::ALL {
            for kind in RealmType::ALL {
                for h in realm(region, kind).houses() {
                    assert_eq!(House::from_key(&h.key()), Some(h));
                }
            }
        }
        for bad in ["alliance", "us-112", "xx-110", "us-", "us-abc"] {
            assert_eq!(House::from_key(bad), None, "{bad}");
        }
    }

    #[test]
    fn house_serializes_as_its_faction() {
        let h = realm(Region::Eu, RealmType::Rp).house(Faction::Horde);
        assert_eq!(serde_json::to_string(&h).unwrap(), "\"horde\"");
    }

    #[test]
    fn realm_label() {
        assert_eq!(realm(Region::Eu, RealmType::Rppvp).label(), "Roleplay PvP · EU");
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
    fn feed_and_faction_round_trip_strings() {
        for f in Faction::ALL {
            assert_eq!(Faction::from_str(f.as_str()), Some(f));
        }
        assert_eq!(Feed::Commodities.as_str(), "commodities");
        assert_eq!(Feed::from_str("items"), Some(Feed::Items));
    }
}
