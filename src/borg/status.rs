use super::log_json::*;
use itertools::Itertools;
use std::collections::VecDeque;

#[derive(Default, Debug, Clone)]
pub struct GeneralStatus {
    pub run: Run,
    pub started: Option<chrono::DateTime<chrono::Local>>,
    /// History per borg command execution. There can be multiple command
    /// executions because of disconnects. The first entry stores early, the
    // second late log messages.
    pub message_history: Vec<(LogCollection, LogCollection)>,
}

#[derive(Default, Debug, Clone)]
pub struct Status {
    pub estimated_size: Option<SizeEstimate>,
    pub started: Option<chrono::DateTime<chrono::Local>>,
    pub total: f64,
    pub copied: f64,
    pub stalled: bool,
    pub data_rate_history: DataRateHistory,
}

fn positive(n: f64) -> f64 {
    if n.is_finite() && n > 0. {
        n
    } else {
        0.
    }
}

impl GeneralStatus {
    pub const MESSAGE_HISTORY_LENGTH: usize = 50;

    pub fn add_message(&mut self, msg: &LogEntry) {
        let mut history = if let Some(h) = self.message_history.pop() {
            h
        } else {
            Default::default()
        };

        if history.0.len() < Self::MESSAGE_HISTORY_LENGTH {
            history.0.push(msg.clone());
        } else if history.1.len() < Self::MESSAGE_HISTORY_LENGTH {
            history.1.push(msg.clone());
        } else {
            if let Some(position) = history.1.iter().position(|x| x.level() < msg.level()) {
                history.1.remove(position);
            } else {
                history.1.remove(0);
            }
            history.1.push(msg.clone());
        }

        self.message_history.push(history);
    }

    pub fn runs_concat_message_history(&self) -> (LogCollection, LogCollection) {
        self.message_history.clone().into_iter().fold(
            (LogCollection::new(), LogCollection::new()),
            |(x1, y1), (x2, y2)| ([x1, x2].concat(), [y1, y2].concat()),
        )
    }

    pub fn all_combined_message_history(&self) -> LogCollection {
        let combined = self.runs_concat_message_history();
        [combined.0, combined.1].concat()
    }

    pub fn last_combined_message_history(&self) -> LogCollection {
        if let Some((x, y)) = self.message_history.clone().last() {
            [x.clone(), y.clone()].concat()
        } else {
            LogCollection::new()
        }
    }
}

impl Status {
    pub fn time_remaining(&self) -> Option<chrono::Duration> {
        if let (Some(skip_remaining_size), Some(copy_remaining_size)) =
            (self.skip_remaining(), self.copy_remaining())
        {
            // Do not trust early estimates
            if Some(true)
                == self
                    .started
                    .map(|x| (chrono::Local::now() - x) < chrono::Duration::seconds(10))
            {
                return None;
            }

            let skip_remaining_time = skip_remaining_size * self.data_rate_history.beta_skipped();
            let copy_remaining_time = copy_remaining_size * self.data_rate_history.beta_copied();

            let remaining_time = positive(skip_remaining_time) + positive(copy_remaining_time);

            if !remaining_time.is_normal() {
                return None;
            }

            Some(chrono::Duration::seconds(remaining_time as i64))
        } else {
            None
        }
    }

    pub fn skip_remaining(&self) -> Option<f64> {
        self.estimated_size
            .as_ref()
            .map(|size| positive(size.unchanged() as f64 - self.skipped()))
    }

    pub fn copy_remaining(&self) -> Option<f64> {
        if let Some(size) = &self.estimated_size {
            let copy = size.changed as f64
                - self.copied
                - positive(self.skipped() - size.unchanged() as f64);

            // If we have to copy more than expected, further estimates are useless
            if !copy.is_normal() {
                return None;
            }

            Some(copy)
        } else {
            None
        }
    }

    pub fn skipped(&self) -> f64 {
        self.total - self.copied
    }
}

#[derive(Debug, Clone)]
pub struct SizeEstimate {
    pub total: u64,
    pub changed: u64,
}

impl SizeEstimate {
    pub const fn unchanged(&self) -> u64 {
        self.total - self.changed
    }
}

#[derive(Default, Debug, Clone)]
pub struct DataRateHistory {
    pub skipped: VecDeque<DataRate>,
    pub copied: VecDeque<DataRate>,
}

/// This struct provides betas from linear regression based on the model
///
/// > `interval(skipped, copied) = beta_skipped * skipped + beta_copied * copied`.
///
/// This model is used to estimate the backup duration. The processing rates
/// are given by `1/beta` in bytes per second.
impl DataRateHistory {
    /// Samples often only span over 0.2 seconds. This choice should ensure for the
    /// analysis to span over at least 3 minute.
    const STORE_SAMPLES: usize = 3 * 60 * 5;
    const GROUP_SAMPLES: usize = 10;

    pub fn insert(&mut self, mut data: DataRate) {
        data.skipped = positive(data.skipped);
        data.copied = positive(data.copied);

        if data.skipped > 0. {
            Self::insert_(&mut self.skipped, data.clone());
        }

        if data.copied > 0. {
            Self::insert_(&mut self.copied, data);
        }
    }

    fn insert_(v: &mut VecDeque<DataRate>, data: DataRate) {
        v.push_front(data);
        v.truncate(Self::STORE_SAMPLES);
    }

    fn chunk(history: VecDeque<DataRate>) -> VecDeque<DataRate> {
        history
            .into_iter()
            .rev()
            .chunks(Self::GROUP_SAMPLES)
            .into_iter()
            .map(std::iter::Sum::sum)
            .collect()
    }

    /// Estimate duration in seconds for skipping and downloading one byte
    ///
    /// ```
    /// # use pika_backup::*;
    /// let mut history = borg::DataRateHistory::default();
    ///
    /// history.insert(borg::DataRate {
    ///     interval: 3.0,
    ///     skipped: 0.0,
    ///     copied: 6.0,
    /// });
    /// history.insert(borg::DataRate {
    ///     interval: 0.5,
    ///     skipped: 2.0,
    ///     copied: 3.0,
    /// });
    /// history.insert(borg::DataRate {
    ///     interval: 1.0,
    ///     skipped: 0.25,
    ///     copied: 0.0,
    /// });
    ///
    /// assert_eq!(borg::DataRateHistory::linear_regression(&history.skipped).0, 4.0);
    /// assert_eq!(borg::DataRateHistory::linear_regression(&history.copied).1, 0.5);
    /// ```
    pub fn linear_regression(history: &VecDeque<DataRate>) -> (f64, f64) {
        // Renaming the original model to
        // `t(x, y) = a * x + b * y`
        // for simpler notation

        let xt: f64 = history.iter().map(|v| v.interval * v.skipped).sum();
        let yt: f64 = history.iter().map(|v| v.interval * v.copied).sum();
        let xy: f64 = history.iter().map(|v| v.skipped * v.copied).sum();
        let x2: f64 = history.iter().map(|v| v.skipped.powi(2)).sum();
        let y2: f64 = history.iter().map(|v| v.copied.powi(2)).sum();

        // least squares for the model (derived on paper)
        let b = (x2 * yt - xy * xt) / (x2 * y2 - xy.powi(2));
        let a = (xt - b * xy) / x2;

        (a, b)
    }

    /// Duration to skip one byte (in seconds)
    pub fn beta_skipped(&self) -> f64 {
        Self::linear_regression(&Self::chunk(self.skipped.clone())).0
    }

    /// Duration to copy one byte (in seconds)
    pub fn beta_copied(&self) -> f64 {
        //Self::linear_regression(&self.copied).1
        Self::linear_regression(&Self::chunk(self.copied.clone())).1
    }
}

#[derive(Debug, Clone, Default)]
pub struct DataRate {
    /// seconds
    pub interval: f64,
    pub skipped: f64,
    pub copied: f64,
}

impl std::ops::Add for DataRate {
    type Output = Self;

    fn add(mut self, other: Self) -> Self {
        self.interval += other.interval;
        self.skipped += other.skipped;
        self.copied += other.copied;
        self
    }
}

impl std::iter::Sum for DataRate {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        iter.fold(Self::default(), |x, y| x + y)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Run {
    Init,
    Running,
    Stalled,
    Reconnecting(std::time::Duration),
    Stopping,
}

impl Default for Run {
    fn default() -> Self {
        Self::Init
    }
}
