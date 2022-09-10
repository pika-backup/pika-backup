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

impl Keep {
    pub fn is_greater_eq_everywhere(&self, other: &Keep) -> bool {
        self.hourly >= other.hourly
            && self.daily >= other.daily
            && self.weekly >= other.weekly
            && self.monthly >= other.monthly
            && self.yearly >= other.yearly
    }
}
