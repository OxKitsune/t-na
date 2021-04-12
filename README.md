# t-na
A discord bot written in rust

## Functionality
This bot uses rich presence events to track spotify activity in a discord server. 
The bot is then able to send a top 10 list of most popular artists and songs.

## Setup
To run the bot first make sure that the bot token is present in the environment variables, 
by using `export DISCORD_TOKEN=<token>` on *nix based systems and `SET DISCORD_TOKEN=<token>` on Windows systems.

After you've exported the token, simply run `cargo run --release` in the root directory to run the bot.
