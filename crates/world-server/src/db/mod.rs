
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::env;
use anyhow::{Result, Context};
use tracing::{info, warn};
use sqlx::FromRow;

#[derive(FromRow)]
struct WorldIpRow {
    pub zone_id: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub heading: f64,
}

#[derive(FromRow)]
struct AccountIdRow {
    pub id: i32,
}

#[derive(Clone, Debug)]
pub struct DatabaseManager {
    pool: Pool<Postgres>,
}

impl DatabaseManager {
    pub async fn new() -> Result<Self> {
        let database_url = env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
        
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&database_url)
            .await
            .context("Failed to connect to database")?;
            
        info!("DatabaseManager connected to Postgres.");
        
        Ok(Self { pool })
    }
    
    // Example Method for Char Create
    pub async fn create_character(
        &self,
        account_id: i32,
        name: &str,
        race: i16,
        class_id: i16,
        gender: i16,
        level: i16,
        base_str: i16,
        base_sta: i16,
        base_dex: i16,
        base_agi: i16,
        base_int: i16,
        base_wis: i16,
        base_cha: i16,
        deity: i16,
        start_zone: i32,
    ) -> Result<i32> {
        let mut tx = self.pool.begin().await?;

        let rec = sqlx::query_as::<_, (i32,)>(
            "INSERT INTO character_data (account_id, name, race, class, gender, level, 
                str, sta, dex, agi, int, wis, cha, deity, zone_id, cur_hp, mana, endurance)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, 100, 100, 100)
             RETURNING id"
        )
        .bind(account_id).bind(name).bind(race).bind(class_id).bind(gender).bind(level)
        .bind(base_str).bind(base_sta).bind(base_dex).bind(base_agi).bind(base_int).bind(base_wis).bind(base_cha)
        .bind(deity).bind(start_zone)
        .fetch_one(&mut *tx).await?;
        
        let char_id = rec.0;

        tx.commit().await?;
        
        Ok(char_id)
    }
    
    pub async fn get_characters(&self, account_id: i32) -> Result<Vec<shared::db::Character>> {
        let rows = sqlx::query_as::<_, shared::db::Character>(
            "SELECT id, account_id, name, last_name, zone_id, zone_instance,
             y, x, z, heading, gender, race, class, level, exp, points as practice_points,
             mana, cur_hp, endurance, str, sta, cha, dex, int, agi, wis,
             face, hair_style, hair_color, beard, beard_color, eye_color_1, eye_color_2,
             drakkin_heritage, drakkin_tattoo, drakkin_details, deity
             FROM character_data WHERE account_id = $1"
        )
        .bind(account_id)
        .fetch_all(&self.pool).await?;

        info!("get_characters returning {} chars", rows.len());
        Ok(rows)
    }

    pub async fn check_name_availability(&self, name: &str) -> Result<bool> {
        let rec = sqlx::query_as::<_, (Option<i64>,)>("SELECT count(*) FROM character_data WHERE name = $1")
            .bind(name)
            .fetch_one(&self.pool).await?;
        let count = rec.0.unwrap_or(0);
        
        Ok(count == 0)
    }

    pub async fn get_character_location(&self, name: &str) -> Result<(i32, f32, f32, f32, f32)> {
        let rec = sqlx::query_as::<_, WorldIpRow>(
            "SELECT zone_id, x, y, z, heading FROM character_data WHERE name = $1"
        )
        .bind(name)
        .fetch_optional(&self.pool).await?;
        
        match rec {
            Some(r) => Ok((
                r.zone_id, 
                r.x as f32, 
                r.y as f32, 
                r.z as f32, 
                r.heading as f32
            )),
            None => Err(anyhow::anyhow!("Character not found")),
        }
    }

    pub async fn delete_character(&self, account_id: i32, name: &str) -> Result<bool> {
        let char_id_rec = sqlx::query_as::<_, AccountIdRow>(
            "SELECT id FROM character_data WHERE account_id = $1 AND name = $2"
        )
        .bind(account_id)
        .bind(name)
        .fetch_optional(&self.pool).await?;
        
        if let Some(rec) = char_id_rec {
            let char_id = rec.id;
            let result = sqlx::query("DELETE FROM character_data WHERE id = $1")
                .bind(char_id)
                .execute(&self.pool).await;
            Ok(result.is_ok())
        } else {
            Ok(false)
        }
    }

    pub async fn verify_session(&self, account_id: i32, session_key: &str) -> Result<bool> {
        let rec = sqlx::query_as::<_, AccountIdRow>(
            "SELECT id FROM account WHERE id = $1 AND ls_session_key = $2"
        )
        .bind(account_id)
        .bind(session_key)
        .fetch_optional(&self.pool).await?;
        
        Ok(rec.is_some())
    }
}
