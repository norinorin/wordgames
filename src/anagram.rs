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

use crate::server_message::{chat, finish, ongoing_round, ServerMessage};

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

    pub async fn guess(&mut self, player: &str, guess: &str) -> bool {
        let mut round_status = self.round_status.lock().await;
        if let RoundStatus::Ongoing(info) = &*round_status {
            if guess.to_lowercase() == info.answer.to_lowercase() {
                *self.player_to_points.entry(player.to_owned()).or_insert(0) += info.score;
                tracing::debug!(
                    "{player} guessed it right. Score: {} (+{}).",
                    self.player_to_points[player],
                    info.score
                );
                self.tx
                    .send(chat!(
                        "@{}: You guessed it in {:.2?}! Your score: {} (+{}).",
                        player,
                        info.starts_at.elapsed(),
                        self.player_to_points[player],
                        info.score
                    ))
                    .unwrap();
                self.tx
                    .send(chat!("Answer was \"{}.\"", info.answer))
                    .unwrap();
                self.tx.send(finish!()).unwrap();
                tracing::debug!("Changing round status to idle.");
                *round_status = RoundStatus::Idle;
                if let Some(handle) = &self.handle {
                    tracing::debug!("Aborting timeout task.");
                    handle.abort();
                }
                return true;
            }
        }

        false
    }

    pub async fn start(&mut self, duration: u32) {
        if let RoundStatus::Ongoing(info) = &*self.round_status.lock().await {
            tracing::debug!("Can't run multiple games simultaneously.");
            self.tx
                .send(chat!(
                    "Please wait until the current game ends. Time left: {:.2?}.",
                    info.ends_at - Instant::now()
                ))
                .unwrap();
            return;
        }

        let random = self.words.choose(&mut rand::thread_rng()).unwrap().clone();
        let shuffled = Self::shuffle_word(&random.word);
        let ends_at = Instant::now() + Duration::from_secs(duration as u64);
        self.tx.send(chat!("Game started!")).unwrap();
        self.tx.send(ongoing_round!(shuffled, ends_at)).unwrap();
        *self.round_status.lock().await = RoundStatus::Ongoing(RoundInfo {
            answer: random.word,
            score: random.score,
            shuffled,
            ends_at,
            starts_at: Instant::now(),
        });
        let tx = self.tx.clone();
        let round_status = self.round_status.clone();
        self.handle = Some(spawn(async move {
            Self::timeout(tx, round_status, ends_at).await;
        }));
    }

    pub async fn timeout(
        tx: broadcast::Sender<String>,
        round_status: Arc<Mutex<RoundStatus>>,
        deadline: Instant,
    ) {
        tracing::debug!(
            "Invalidating game in {:?} seconds.",
            (deadline - Instant::now()).as_secs()
        );
        sleep_until(deadline).await;
        let mut round_status = round_status.lock().await;
        if let RoundStatus::Ongoing(info) = &*round_status {
            tx.send(finish!()).unwrap();
            tx.send(chat!(
                "No one has guessed the answer. Answer: {}.",
                info.answer
            ))
            .unwrap();
            *round_status = RoundStatus::Idle;
        }
        tracing::debug!("Game invalidated.");
    }

    fn shuffle_word(word: &str) -> String {
        tracing::debug!("Shuffling word: {word}.");
        let mut chars: Vec<char> = word.chars().collect();
        loop {
            chars.shuffle(&mut rand::thread_rng());
            let shuffled: String = chars.iter().collect();
            // If the word has less than 4 chars,
            // don't bother retrying
            if chars.len() <= 3 || shuffled != word {
                tracing::debug!("Shuffled {word} -> {shuffled}.");
                break shuffled;
            }
            tracing::debug!("{0} == {0}, reshuffling.", word);
        }
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
    Ongoing(RoundInfo),
}

#[derive(Eq, PartialEq)]
pub struct RoundInfo {
    pub answer: String,
    pub shuffled: String,
    pub score: u32,
    pub ends_at: Instant,
    pub starts_at: Instant,
}
