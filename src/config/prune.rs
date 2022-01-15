#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Prune {
    pub enabled: bool,
    pub keep: Keep,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Keep {
    pub hourly: u32,
    pub daily: u32,
    pub weekly: u32,
    pub monthly: u32,
    pub yearly: u32,
}

impl Default for Keep {
    fn default() -> Self {
        Self {
            hourly: 48,
            daily: 14,
            weekly: 4,
            monthly: 12,
            yearly: 10,
        }
    }
}
