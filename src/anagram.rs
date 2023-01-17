use rand::seq::SliceRandom;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::spawn;
use tokio::task::JoinHandle;
use tokio::time::{Duration, Instant};
use tokio::{
    sync::{broadcast, Mutex},
    time::sleep_until,
};

pub struct Anagram {
    words: Vec<DatamuseRes>,
    players: HashSet<String>,
    pub round_status: Arc<Mutex<RoundStatus>>,
    pub player_to_points: HashMap<String, u32>,
    pub handle: Option<JoinHandle<()>>,
    pub tx: broadcast::Sender<String>,
}

impl Anagram {
    pub async fn new(populate_words: bool) -> reqwest::Result<Self> {
        let mut self_ = Self {
            tx: broadcast::channel(64).0,
            words: Default::default(),
            players: Default::default(),
            round_status: Default::default(),
            player_to_points: Default::default(),
            handle: Default::default(),
        };
        if populate_words {
            self_.populate_words().await?;
        }
        Ok(self_)
    }

    pub async fn populate_words(&mut self) -> reqwest::Result<()> {
        self.words = reqwest::get("http://api.datamuse.com/words?sp=?????&max=500")
            .await?
            .json()
            .await?;
        Ok(())
    }

    pub fn insert_player(&mut self, name: &str) -> bool {
        if self.players.contains(name) {
            return false;
        }

        self.players.insert(name.to_owned());
        true
    }

    pub fn remove_player(&mut self, name: &str) {
        self.players.remove(name);
    }

    pub async fn guess(&mut self, player: String, guess: String) {
        if let RoundStatus::Ongoing(status) = &*self.round_status.lock().await {
            if guess == status.answer {
                *self.player_to_points.entry(player.clone()).or_insert(0) += status.score;
                self.tx
                    .send(format!(
                        "@{}: you guessed it right! Your score: {} (+{})",
                        player, self.player_to_points[&player], status.score
                    ))
                    .unwrap();
                self.finalise().await;
            } else {
                self.tx.send(format!("@{player}: keep guessing!")).unwrap();
            }
        }
    }

    pub async fn start(&mut self, duration: u32) {
        if let RoundStatus::Ongoing(status) = &*self.round_status.lock().await {
            self.tx
                .send(format!(
                    "Please wait until the current game ends. Time left: {:?}",
                    status.ends_at - Instant::now()
                ))
                .unwrap();
            return;
        }

        let random = self.words.choose(&mut rand::thread_rng()).unwrap().clone();
        let mut chars: Vec<char> = random.word.chars().collect();
        chars.shuffle(&mut rand::thread_rng());
        let ends_at = Instant::now() + Duration::from_secs(duration as u64);
        let shuffled: String = chars.iter().collect();
        self.tx
            .send(format!(
                "Word: {}. You have {duration} second(s) to guess!",
                shuffled,
            ))
            .unwrap();
        *self.round_status.lock().await = RoundStatus::Ongoing(GameInfo {
            answer: random.word,
            score: random.score,
            shuffled,
            ends_at,
        });
        let tx = self.tx.clone();
        let round_status = self.round_status.clone();
        self.handle = Some(spawn(async move {
            Self::timeout(tx, round_status, ends_at).await;
        }));
    }

    async fn finalise(&self) {
        if let Some(handle) = &self.handle {
            handle.abort();
        }

        *self.round_status.lock().await = RoundStatus::Idle;
    }

    pub async fn timeout(
        tx: broadcast::Sender<String>,
        round_status: Arc<Mutex<RoundStatus>>,
        deadline: Instant,
    ) {
        sleep_until(deadline).await;
        if let RoundStatus::Ongoing(status) = &*round_status.lock().await {
            tx.send(format!(
                "No one has guessed the answer. Answer: {}",
                status.answer
            ))
            .unwrap();
        }
        *round_status.lock().await = RoundStatus::Idle;
    }
}

#[derive(Deserialize, Default, Clone, Eq, PartialEq)]
pub struct DatamuseRes {
    word: String,
    score: u32,
}

#[derive(Default, Eq, PartialEq)]
pub enum RoundStatus {
    #[default]
    Idle,
    Ongoing(GameInfo),
}

#[derive(Eq, PartialEq)]
pub struct GameInfo {
    pub answer: String,
    pub shuffled: String,
    pub score: u32,
    pub ends_at: Instant,
}
