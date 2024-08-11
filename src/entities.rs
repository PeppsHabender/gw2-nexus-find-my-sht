use crate::constants::{
    ASCENDED_COLOR, EXOTIC_COLOR, FINE_COLOR, JUNK_COLOR, LEGENDARY_COLOR, MASTERWORK_COLOR,
    RARE_COLOR,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LoadingState<T> {
    Init,
    Loading,
    Success(T),
    Error(&'static str),
}

#[derive(Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Gw2Permission {
    Account,
    Builds,
    Characters,
    Guilds,
    Inventories,
    Progression,
    Pvp,
    #[serde(alias = "tradingpost")]
    TradingPost,
    Unlocks,
    Wallet,
}

#[derive(Serialize, Deserialize)]
pub struct Gw2ApiKey {
    pub id: String,
    pub name: String,
    pub permissions: Vec<Gw2Permission>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Gw2PlayerItem {
    pub id: usize,
    pub count: usize,
    pub charges: Option<usize>,
    pub skin: Option<usize>,
    pub upgrades: Option<Vec<usize>>,
    pub infusions: Option<Vec<usize>>,
    pub binding: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Gw2Bag {
    pub id: usize,
    pub size: usize,
    pub inventory: Vec<Option<Gw2PlayerItem>>,
}

#[derive(Serialize, Deserialize)]
pub struct Gw2Inventory {
    pub bags: Vec<Gw2Bag>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum Gw2ItemType {
    Armor,
    Back,
    Bag,
    Consumable,
    Container,
    CraftingMaterial,
    Gathering,
    Gizmo,
    JadeTechModule,
    Key,
    MiniPet,
    PowerCore,
    Relic,
    Tool,
    Trait,
    Trinket,
    Trophy,
    UpgradeComponent,
    Weapon,
    #[default]
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum Gw2Rarity {
    Junk,
    #[default]
    Basic,
    Fine,
    Masterwork,
    Rare,
    Exotic,
    Ascended,
    Legendary,
}

impl Gw2Rarity {
    pub fn color(&self) -> [f32; 4] {
        match self {
            Gw2Rarity::Junk => JUNK_COLOR,
            Gw2Rarity::Basic => [1.0, 1.0, 1.0, 1.0],
            Gw2Rarity::Fine => FINE_COLOR,
            Gw2Rarity::Masterwork => MASTERWORK_COLOR,
            Gw2Rarity::Rare => RARE_COLOR,
            Gw2Rarity::Exotic => EXOTIC_COLOR,
            Gw2Rarity::Ascended => ASCENDED_COLOR,
            Gw2Rarity::Legendary => LEGENDARY_COLOR,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Gw2Item {
    pub id: usize,
    pub name: String,
    pub description: Option<String>,
    pub rarity: Gw2Rarity,
    #[serde(alias = "type")]
    pub item_type: Gw2ItemType,
    pub icon: Option<String>,
}
