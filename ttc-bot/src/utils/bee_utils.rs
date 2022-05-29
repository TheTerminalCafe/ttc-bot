use std::collections::HashMap;

use crate::utils::bee_script;
use lazy_static::lazy_static;
use poise::serenity_prelude::{ChannelId, Timestamp, UserId, Webhook};
use rand::Rng;

lazy_static! {
    static ref BEE_LINE_VEC: Vec<String> = bee_lines();
}

pub struct BeeifiedUser {
    pub timestamp: Timestamp,
    pub webhooks: HashMap<ChannelId, Webhook>,
    pub beelate: bool,
}

pub struct BeezoneChannel {
    pub timestamp: Timestamp,
    pub webhook: Option<Webhook>,
    pub users: Vec<UserId>,
    pub beelate: bool,
}

impl BeeifiedUser {
    pub fn new(timestamp: Timestamp, webhooks: HashMap<ChannelId, Webhook>, beelate: bool) -> Self {
        Self {
            timestamp,
            webhooks,
            beelate,
        }
    }
}

impl BeezoneChannel {
    pub fn new(timestamp: Timestamp, beelate: bool) -> Self {
        Self {
            timestamp,
            webhook: None,
            users: Vec::new(),
            beelate,
        }
    }
}

fn bee_lines() -> Vec<String> {
    bee_script::BEE_SCRIPT
        .lines()
        .map(|line| line.to_string())
        .collect()
}

pub fn beelate(string: &str) -> String {
    let mut best_score = (0.0, 0);
    for i in 0..BEE_LINE_VEC.len() {
        let score = strsim::normalized_damerau_levenshtein(string, &BEE_LINE_VEC[i]);
        if score > best_score.0 {
            best_score = (score, i);
        }
    }
    BEE_LINE_VEC[best_score.1].clone()
}

pub fn get_bee_line(index: Option<usize>) -> String {
    let index = match index {
        Some(index) => index,
        None => {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..BEE_LINE_VEC.len())
        }
    };
    BEE_LINE_VEC[index].clone()
}
