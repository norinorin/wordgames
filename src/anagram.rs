use rand::seq::SliceRandom;
use reqwest;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use tokio::spawn;
use tokio::task::JoinHandle;
use tokio::time::{Duration, Instant};
use tokio::{sync::broadcast, time::sleep_until};

pub struct Anagram {
    words: Vec<DatamuseRes>,
    players: HashSet<String>,
    pub round_status: RoundStatus,
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

    fn ensure_state(&mut self) {
        match &self.round_status {
            RoundStatus::Ongoing(status) if status.ends_at < Instant::now() => {
                self.round_status = RoundStatus::Idle
            }
            _ => {}
        }
    }

    pub fn guess(&mut self, player: String, guess: String) {
        self.ensure_state();
        if let RoundStatus::Ongoing(status) = &self.round_status {
            if guess == status.answer {
                *self.player_to_points.entry(player.clone()).or_insert(0) += status.score;
                self.tx
                    .send(format!(
                        "@{}: you guessed it right! Your score: {} (+{})",
                        player, self.player_to_points[&player], status.score
                    ))
                    .unwrap();
                self.finalise();
            } else {
                self.tx.send(format!("@{player}: keep guessing!")).unwrap();
            }
        }
    }

    pub fn start(&mut self, duration: u32) {
        self.ensure_state();
        if let RoundStatus::Ongoing(status) = &self.round_status {
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
                "Word: {} ({}). You have {duration} second(s) to guess!",
                shuffled.clone(),
                random.word.clone() // remove later
            ))
            .unwrap();
        self.round_status = RoundStatus::Ongoing(GameInfo {
            answer: random.word.clone(),
            score: random.score,
            shuffled,
            ends_at,
        });
        let tx = self.tx.clone();
        let answer = random.word;
        self.handle = Some(spawn(Anagram::timeout(tx, answer, ends_at)));
    }

    fn finalise(&mut self) {
        if let Some(handle) = &self.handle {
            handle.abort();
        }
        self.round_status = RoundStatus::Idle;
    }

    pub async fn timeout(tx: broadcast::Sender<String>, answer: String, deadline: Instant) {
        sleep_until(deadline).await;
        tx.send(format!("No one has guessed the answer. Answer: {}", answer))
            .unwrap();
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
