// https://github.com/PixelSam123/pixelsam123.github.io/blob/2b2263f842096385dc53bda03c3af899136f999c/assets/Minigames-90435252.js
use serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "type", content = "content")]
pub enum ServerMessage {
    ChatMessage(String),
    OngoingRoundInfo {
        word_to_guess: String,
        round_finish_time: chrono::DateTime<chrono::Utc>,
    },
    FinishedGame,
    FinishedRoundInfo {
        word_answer: String,
        to_next_round_time: chrono::DateTime<chrono::Utc>,
    },
}

macro_rules! chat {
    ($($t:tt)*) => {
        serde_json::to_string(&ServerMessage::ChatMessage(format!($($t)*))).unwrap()
    };
}

macro_rules! ongoing_round {
    ($wd:expr, $eat:expr) => {
        serde_json::to_string(&ServerMessage::OngoingRoundInfo {
            word_to_guess: $wd.to_owned(),
            round_finish_time: chrono::Utc::now()
                + chrono::Duration::from_std(($eat - Instant::now())).unwrap(),
        })
        .unwrap()
    };
}

macro_rules! finish {
    () => {
        serde_json::to_string(&ServerMessage::FinishedGame).unwrap()
    };
}

macro_rules! next {
    ($wd:expr, $eat:expr) => {
        serde_json::to_string(&ServerMessage::FinishedRoundInfo {
            word_answer: $wd.to_owned(),
            to_next_round_time: chrono::Utc::now()
                + chrono::Duration::from_std(($eat - Instant::now())).unwrap(),
        })
        .unwrap()
    };
}

pub(crate) use chat;
pub(crate) use finish;
pub(crate) use next;
pub(crate) use ongoing_round;
