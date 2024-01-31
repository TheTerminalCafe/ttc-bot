# DEPCRECATED! Only accepting important fixes

# Welcome to The Terminal cafe Support Bot Repository

Hello, hope you are having a nice day. This is the official repository for The Terminal Cafe Support bot. It was created the to manage the support questions in an automated and organized way. 


# Technologies used

While creating this bot we tried to use efficient and modern technologies . 
We based the main logic on **Rust** and the main database is handled by **PostgreSQL**.

The rust libraries in use are listed here:
- `serenity-rs`: The main discord bot library.
- `sqlx`: A library for accessing the sql database.
- `tokio`: A fast asynchronous runtime.
- `serde`: Data serialization and deserialization for the config file.
- `serde_yaml`: Serde implementation for YAML.
- `clap`: Launch argument parsing.
- `regex`: Regex for input sanitazion.
- `chrono`: Timestamps for the database.
- `dotenv`: `.env` file reading for easier environment variables during compile time.
- `log`: A widely used logging facade for logging.
- `env_logger`: The logging implementation for the `log` crate facades.

## Running

The bot requires a postgresql database with a table based on the schema in `sql/ttc-bot.sql` to function, as well as a YAML config file with the following format:
```
---
  token: <The bot token you wish to use>
  application_id: <The application id for the bot on discord, used for interactions>
  sqlx_config: <A string for sqlx to connect to the database, postgres://username:password@host:port/database_name>
  support_channel: <Discord channel id for the support channel>
  conveyance_channel: <Discord channel id for the conveyance channel>
  conveyance_blacklisted_channels: <Discord channel ids to be excluded from conveyance, [<channel_id>, <channel_id>...]>
  welcome_channel: <Discord channel id for channel to send user welcome messages in>
  welcome_messages: <Array of welcome messages, "%user%" is substituted for the joined members mention, ["*%user% joined*", ...]>
  owners: <Array of owner user ids, [<user_id>, <user_id>...]>
  verified_role: <Role id for the verified role>
  moderator_role: <Role id for the moderator role>
```

You need to set the `DATABASE_URL` variable in `.env` to the same value as `sqlx_config` in the config file to allow for compile time checking of database calls.
Running is done with `cargo run -- -c <path/to/config/file>`.

## Dependencies 

If you want to install TTC support bot in your own system these following dependencies are required 

- Latest rust toolchain is preferable
- PostgreSQL database 

## Contribute 

If you want to contribute to our project feel free to do it 😃. To start you can make a fork of our project, if you fork our project we request you to change the name of your forked one to minimize confusion. If you want to join our team and contribute to the main project feel free to join our Discord server **The Terminal Cafe**, we would love to see you there.

## Naming conversion

If you work on the project please follow the naming conversion to minimize confusion. Use self explanatory class and function names, also try to comment out your code. 

Example:
```
// This line prints the inc_id read from the DB
println!("{}", inc_id);
```
Please don't add anything offensive or political into the code base don't make it uncomfortable for others.

# Credits

If you like our project and want to make a fork of it please add credits to the main repository.

## Data collection

This bot only stores the following data when you create a support ticket.

``` 
userID - to manage the incident creator
timestamp - to find out when the incident is created 
threadID - to store the conversertation thread for future referances 
questions - to store the question for better searching ability
```

# Contributors 

Thanks to these people we are able to develop this project 😉😉

|                    |               | 
|--------------------|---------------|
|Kiro				 | Lead Rust dev | 
|Ereshkigal          | SQL and logic |
|UltimateNyn|Contributer and Rust dev|
|Stargirl|Project lead|

Thanks for using out software. See you in the discord server.
# Made with 💛 and code

[Discord](https://discord.gg/qsBmyM9)
