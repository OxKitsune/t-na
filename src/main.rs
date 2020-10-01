extern crate fern;
extern crate r2d2;
extern crate r2d2_sqlite;
extern crate rspotify;
extern crate rusqlite;
extern crate reqwest;
extern crate image;

use std::{collections::HashSet, env, io, sync::Arc, path, fs};

use fern::*;
use log::{debug, info, trace, warn};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rspotify::client::Spotify;
use rspotify::oauth2::SpotifyClientCredentials;
use rspotify::util::get_token;
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
use serenity::model::gateway::Activity;

use commands::{
    ping::*,
    profile::*,
    spotify::*,
};
use user::*;
use util::colour;
use rspotify::model::search::SearchResult;
use std::fs::File;
use std::io::Write;
use std::error::Error;

mod commands;
mod util;
mod user;

const DB_PATH: &str = "t-na.db";
const SPOTIFY_CACHE_DIR: &str = "./spotify_cache/";
const SPOTIFY_CACHE_ARTIST_DIR: &str = "./spotify_cache/artists/";
const SPOTIFY_CACHE_SONG_DIR: &str = "./spotify_cache/songs/";

struct DBPool;

struct TopArtist;

struct SpotifyClient;

impl TypeMapKey for TopArtist {
    type Value = String;
}

impl TypeMapKey for DBPool {
    type Value = r2d2::Pool<SqliteConnectionManager>;
}

impl TypeMapKey for SpotifyClient {
    type Value = Spotify;
}

struct Handler;

#[group]
#[commands(ping, spotify, profile)]
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


    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
        set_activity(&ctx).await;
    }

    async fn presence_update(&self, ctx: Context, data: PresenceUpdateEvent) {
        let presence = data.presence;

        for activity in presence.activities {
            if activity.name == "Spotify" {
                let mut song_name = activity.details.expect("No song?");
                let assets = activity.state.expect("No artists?");
                let artists: Vec<&str> = assets.split("; ").collect();
                let mut assets = activity.assets.expect("No assets");
                let album = assets.large_text.expect("No album name?");

                let query = format!("{} {}", song_name, artists.join(", "));

                handle_song_cache(&ctx, &*query).await;

                info!("Playing: {} by ({}) on {}", song_name, artists.join(", "), album);
                add_listen(&ctx, song_name, artists).await;
                set_activity(&ctx).await;
            }
        }
    }
}

async fn handle_song_cache (ctx: &Context, song_name: &str) {
    let mut data = ctx.data.write().await;
    let spotify = data.get_mut::<SpotifyClient>().expect("Expected Connection in TypeMap.");

    let search = spotify.search(&*song_name, rspotify::senum::SearchType::Track, 1, 0, Option::None, Option::None).await;

    match search {
        Ok(search_result) => {

            match search_result {
                SearchResult::Tracks(page) => {

                    for track in page.items {
                        info!("Track id: {}", track.id.unwrap_or(String::from("TRACK ID")));
                        info!("Album art: {}", track.album.images[0].url);

                        let output_dir = format!("{}/{}.jpg", SPOTIFY_CACHE_SONG_DIR, song_name);
                        download_image(&*track.album.images[2].url, &*output_dir).await;
                    }

                }
                _ => {}
            }

        },
        Err(error) => warn!("Failed to get track: {}", error)
    }
}



pub async fn download_image (url: &str, out: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut response = reqwest::get(url).await?;
    let mut dest = File::create(out)?;

    let content = response.bytes().await?;
    io::copy(&mut content.as_ref(), &mut dest);
    Ok(())
}


/// Set the activity to the most listened artist
async fn set_activity(ctx: &Context) {
    let artist = get_top_artist(ctx).await;
    let mut data = ctx.data.write().await;
    let top = data.get::<TopArtist>().unwrap();

    if &artist != top {
        info!("Updating activity to top artist: {}", artist);
        ctx.set_activity(Activity::listening(&artist)).await;
        data.insert::<TopArtist>(artist);
    }
}

/// Get the top artist from the database
async fn get_top_artist(ctx: &Context) -> String {
    let mut data = ctx.data.write().await;
    let pool = data.get_mut::<DBPool>().expect("Expected Connection in TypeMap.");

    let conn = pool.get().unwrap();
    let artist = conn.query_row("SELECT * FROM artist ORDER BY listen_count DESC LIMIT 1;", NO_PARAMS, |row| {
        Ok(ListenEntry {
            name: row.get(0).unwrap(),
            listen_count: row.get(1).unwrap(),
        })
    }).unwrap();

    artist.name
}

async fn add_listen(ctx: &Context, song: String, artists: Vec<&str>) {

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

    info!("Setting up spotify cache...");
    let mut cache_path = path::Path::new(SPOTIFY_CACHE_DIR);
    let mut artist_cache_path = path::Path::new(SPOTIFY_CACHE_ARTIST_DIR);
    let mut song_cache_path = path::Path::new(SPOTIFY_CACHE_SONG_DIR);

    if !cache_path.exists() {
        fs::create_dir(cache_path);
    }

    if !artist_cache_path.exists() {
        fs::create_dir(artist_cache_path);
    }

    if !song_cache_path.exists() {
        fs::create_dir(song_cache_path);
    }

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
            .prefix("d!"))
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
        data.insert::<TopArtist>(String::from("-"));
        data.insert::<SpotifyClient>(setup_spotify());
        info!("Done!");
    }

    if let Err(why) = client.start().await {
        warn!("Client error: {:?}", why);
    }
}

fn setup_spotify() -> Spotify {

    let mut client_id = env::var("RSPOTIFY_CLIENT_ID")
        .expect("Expected a client id in the environment");
    let mut client_secret = env::var("RSPOTIFY_CLIENT_SECRET")
        .expect("Expected a client secret in the environment");

    let client_credential = SpotifyClientCredentials::default()
        .client_id(&*client_id)
        .client_secret(&*client_secret)
        .build();

    // Build the spotify client
    Spotify::default()
        .client_credentials_manager(client_credential)
        .build()
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

    pool.get().unwrap().execute(
        "CREATE TABLE IF NOT EXISTS user (
                  id             TEXT PRIMARY KEY NOT NULL,
                  currency       INTEGER NOT NULL
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
        .level_for("overly-verbose-target", log::LevelFilter::Debug)

        // Prevent these libraries from spamming the console with info that's not relevant to t-na
        .level_for("rustls", log::LevelFilter::Info)
        .level_for("hyper", log::LevelFilter::Info)
        .level_for("serenity", log::LevelFilter::Info)
        .level_for("reqwest", log::LevelFilter::Info)
        .level_for("h2", log::LevelFilter::Info)
        .level_for("tungstenite", log::LevelFilter::Info)
        .level_for("rspotify", log::LevelFilter::Debug)
        .chain(io::stdout());

    // Build the log config
    base_config
        .chain(file_config)
        .chain(stdout_config)
        .apply()?;

    Ok(())
}