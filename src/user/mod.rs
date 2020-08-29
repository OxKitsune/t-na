use log::{debug, info, trace, warn};
use rusqlite::{Connection, Error, NO_PARAMS, params, Result};
use serenity::{
    client::Context,
    model::id::UserId,
};

use crate::DBPool;

pub mod user;

/// Struct that holds all TNaUser related data
pub struct TNaUser {
    /// The id of the TNaUser
    pub id: UserId,
    /// The amount of currency the user has stored
    pub currency: i32,
}

pub struct TNaUserLoadError {
    pub reason: String
}

impl TNaUserLoadError {
    /// Creates a new TNaUsderLoadError from an existing rusqlite error
    pub fn from(err: Error) -> TNaUserLoadError {
        TNaUserLoadError {
            reason: err.to_string()
        }
    }
}

impl TNaUser {
    /// Loads the TNaUser from the database
    /// This will automatically append the user to the UserContainer
    pub async fn load_user(ctx: &Context, id: &UserId) -> Result<TNaUser> {

        // Get database
        let mut data = ctx.data.write().await;
        let pool = data.get_mut::<DBPool>().expect("Expected Connection in TypeMap.");

        let mut conn = pool.get().unwrap();

        let user = conn.query_row("SELECT * FROM USER WHERE id=?1", params![id.to_string()], |row| {
            Ok(TNaUser {
                id: UserId::from(id),
                currency: row.get(1).unwrap(),
            })
        }).unwrap();

        info!("Loaded {} from database!", id);


        Result::Ok(user)
    }
}