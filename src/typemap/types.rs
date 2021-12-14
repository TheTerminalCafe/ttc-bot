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

pub struct SupportChannelType;
impl TypeMapKey for SupportChannelType {
    type Value = u64;
}

pub struct BoostLevelType;
impl TypeMapKey for BoostLevelType {
    type Value = u64;
}

pub struct ConveyanceChannelType;
impl TypeMapKey for ConveyanceChannelType {
    type Value = u64;
}

pub struct WelcomeChannelType;
impl TypeMapKey for WelcomeChannelType {
    type Value = u64;
}

pub struct WelcomeMessagesType;
impl TypeMapKey for WelcomeMessagesType {
    type Value = Vec<String>;
}

pub struct ConveyanceBlacklistedChannelsType;
impl TypeMapKey for ConveyanceBlacklistedChannelsType {
    type Value = Vec<u64>;
}
