extern crate fern;
extern crate r2d2;
extern crate r2d2_sqlite;
extern crate rusqlite;

use std::{collections::HashSet, env, io, sync::Arc};

use fern::*;
use log::{debug, info, trace, warn};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, NO_PARAMS, params, Result};
use serenity::{
    async_trait,
    client::bridge::gateway::ShardManager,
    framework::{
        standard::macros::group,
        StandardFramework,
    },
    http::Http,
    model::{channel::*, event::PresenceUpdateEvent, event::ResumedEvent, gateway::Ready, guild::*, id::*},
    prelude::*,
};

use commands::{
    ping::*,
    spotify::*,
};
use util::colour;

mod commands;
mod util;

const DB_PATH: &str = "t-na.db";

struct DBPool;

impl TypeMapKey for DBPool {
    type Value = r2d2::Pool<SqliteConnectionManager>;
}


struct Handler;

#[group]
#[commands(ping, spotify)]
struct General;

#[async_trait]
impl EventHandler for Handler {
    async fn guild_member_addition(&self, ctx: Context, guild_id: GuildId, mut member: Member) {
        let role_id = RoleId(637806086732120064);

        // Log info
        info!("New member joined {}: {}#{}", guild_id, member.user.name, member.user.discriminator);

        // Assign role
        member.add_role(ctx.http, role_id).await;
        info!("   - Added role {}", role_id);
    }


    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }

    async fn presence_update(&self, ctx: Context, data: PresenceUpdateEvent) {
        let presence = data.presence;
        match presence.activity {
            Some(activity) => {
                if activity.name == "Spotify" {
                    let song_name = activity.details.expect("No song?");
                    let assets = activity.state.expect("No artists?");
                    let artists: Vec<&str> = assets.split("; ").collect();
                    let album = activity.assets.expect("No assets?").large_text.expect("No album name?");

                    info!("Playing: {} by ({}) on {}", song_name, artists.join(", "), album);
                    add_listen(ctx, song_name, artists).await;
                }
            }
            None => {}
        };
    }
}

async fn add_listen(ctx: Context, song: String, artists: Vec<&str>) {

    // Create sqlite database
    let mut data = ctx.data.write().await;
    let pool = data.get_mut::<DBPool>().expect("Expected Connection in TypeMap.");

    let conn = pool.get().unwrap();

    conn.execute("INSERT INTO song (name, listen_count) VALUES (?1, ?2) ON CONFLICT(name) DO UPDATE SET listen_count=listen_count+1;", params![song, 1]).expect("Failed to update played count!");

    for artist in artists {
        conn.execute("INSERT INTO artist (name, listen_count) VALUES (?1, ?2) ON CONFLICT(name) DO UPDATE SET listen_count=listen_count+1;", params![artist, 1]).expect("Failed to update artist count!");
    }
}

#[tokio::main]
async fn main() {

    // Setup logging
    setup_logging().expect("failed to initialize logging.");

    info!("T-NA v0.0.1 starting up!");

    debug!("DEBUG output enabled.");
    trace!("TRACE output enabled.");

    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a token in the environment");

    let http = Http::new_with_token(&token);

    // We will fetch your bot's owners and id
    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    // Create the framework
    let framework = StandardFramework::new()
        .configure(|c| c
            .owners(owners)
            .prefix("!"))
        .group(&GENERAL_GROUP);


    // Setup database
    let pool = setup_database();

    let mut client = Client::new(&token)
        .framework(framework)
        .event_handler(Handler)
        .await
        .expect("Err creating client");
    {
        info!("Inserting connection into HttpContext...");
        let mut data = client.data.write().await;
        data.insert::<DBPool>(pool);
        info!("Done!");
    }

    if let Err(why) = client.start().await {
        warn!("Client error: {:?}", why);
    }
}

fn setup_database() -> Pool<SqliteConnectionManager> {
    info!("Opening database connection...");

    // Create sqlite database
    let manager = SqliteConnectionManager::file(DB_PATH);
    let pool = r2d2::Pool::new(manager).unwrap();


    pool.get().unwrap().execute(
        "CREATE TABLE IF NOT EXISTS song (
                  name            TEXT PRIMARY KEY NOT NULL,
                  listen_count    INTEGER NOT NULL
                  )",
        NO_PARAMS,
    ).expect("Failed to create table?");

    pool.get().unwrap().execute(
        "CREATE TABLE IF NOT EXISTS artist (
                  name            TEXT PRIMARY KEY NOT NULL,
                  listen_count    INTEGER NOT NULL
                  )",
        NO_PARAMS,
    ).expect("Failed to create table?");

    pool
}

fn setup_logging() -> Result<(), fern::InitError> {

    let base_config = fern::Dispatch::new()
        .level(log::LevelFilter::Debug)
        .level_for("overly-verbose-target", log::LevelFilter::Info);

    // Separate file config so we can include year, month and day in file logs
    let file_config = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .chain(fern::log_file("latest.log")?);

    let stdout_config = fern::Dispatch::new()
        .format(|out, message, record| {
            // special format for debug messages coming from our own crate.
            if record.level() > log::LevelFilter::Info && record.target() == "cmd_program" {
                out.finish(format_args!(
                    "---\nDEBUG: {}: {}\n---",
                    chrono::Local::now().format("%H:%M:%S"),
                    message
                ))
            } else {
                out.finish(format_args!(
                    "{}[{}][{}] {}",
                    chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                    record.target(),
                    record.level(),
                    message
                ))
            }
        })

        // Set base verbosity
        .level(log::LevelFilter::Debug)
        .level_for("overly-verbose-target", log::LevelFilter::Info)

        // Prevent these libraries from spamming the console with info that's not relevant to t-na
        .level_for("rustls", log::LevelFilter::Info)
        .level_for("hyper", log::LevelFilter::Info)
        .level_for("serenity", log::LevelFilter::Info)
        .level_for("reqwest", log::LevelFilter::Info)
        .level_for("h2", log::LevelFilter::Info)
        .level_for("tungstenite", log::LevelFilter::Info)
        .chain(io::stdout());

    // Build the log config
    base_config
        .chain(file_config)
        .chain(stdout_config)
        .apply()?;

    Ok(())
}