use regex::Regex;
use serenity::{
    client::bridge::gateway::ShardManager,
    model::id::UserId,
    prelude::{Mutex, TypeMapKey},
};
use sqlx::PgPool;
use std::sync::Arc;

// Types to be used in the ctx.data typemap which is available for all commands

pub struct ShardManagerType;
impl TypeMapKey for ShardManagerType {
    type Value = Arc<Mutex<ShardManager>>;
}

pub struct ThreadNameRegexType;
impl TypeMapKey for ThreadNameRegexType {
    type Value = Regex;
}

pub struct UsersCurrentlyQuestionedType;
impl TypeMapKey for UsersCurrentlyQuestionedType {
    type Value = Vec<UserId>;
}

pub struct PgPoolType;
impl TypeMapKey for PgPoolType {
    type Value = PgPool;
}
