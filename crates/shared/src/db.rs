use tokio::sync::{mpsc, oneshot};
use sqlx::{PgPool, FromRow};
use std::sync::Arc;

#[derive(Debug)]
pub enum DbRequest {

    SetSessionKey {
        account_id: i32,
        key: u32,
        respond_to: oneshot::Sender<bool>,
    },
    GetWorldServer {
        server_id: i32,
        respond_to: oneshot::Sender<Result<Option<String>, sqlx::Error>>,
    },
    VerifySession {
        account_id: i32,
        session_key: u32,
        respond_to: oneshot::Sender<Option<i32>>,
    },
    GetCharacters {
        account_id: i32,
        respond_to: oneshot::Sender<Vec<Character>>,
    },
    GetCharacterZone {
        char_id: i32,
        respond_to: oneshot::Sender<Option<String>>,
    },
}

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Character {
    pub id: i32,
    pub account_id: i32,
    pub name: String,
    pub last_name: Option<String>,
    pub zone_id: i32,
    pub zone_instance: i32,
    pub y: f32,
    pub x: f32,
    pub z: f32,
    pub heading: f32,
    pub gender: i32,
    pub race: i32,
    pub class: i32,
    pub level: i32,
    pub exp: i32,
    pub practice_points: i32,
    pub mana: i32,
    pub cur_hp: i32,
    pub endurance: i32,
    pub str: i32,
    pub sta: i32,
    pub cha: i32,
    pub dex: i32,
    pub int: i32,
    pub agi: i32,
    pub wis: i32,
    pub face: i32,
    pub hair_style: i32,
    pub hair_color: i32,
    pub beard: i32,
    pub beard_color: i32,
    pub eye_color_1: i32,
    pub eye_color_2: i32,
    pub drakkin_heritage: i32,
    pub drakkin_tattoo: i32,
    pub drakkin_details: i32,
    pub deity: i32,
}

#[derive(FromRow)]
struct WorldIpRow { ip_address: String }

pub struct DbWorker {
    pub pool: Option<Arc<PgPool>>,
    pub rx: mpsc::Receiver<DbRequest>,
}

#[derive(FromRow)]
struct AccountIdRow {
    id: i32,
}

impl DbWorker {
    pub async fn run(mut self) {
        log::info!("DbWorker started");
        while let Some(req) = self.rx.recv().await {
            match req {

                DbRequest::SetSessionKey { account_id, key, respond_to } => {
                    if self.pool.is_none() {
                        let _ = respond_to.send(true);
                        continue;
                    }
                    let key_str = format!("{:010}", key);
                    let res = sqlx::query("UPDATE account SET ls_session_key = $1 WHERE id = $2")
                        .bind(&key_str)
                        .bind(account_id)
                        .execute(&**self.pool.as_ref().unwrap())
                        .await;

                    match res {
                        Ok(_) => { let _ = respond_to.send(true); },
                        Err(e) => {
                            log::error!("DB SetSessionKey Error: {}", e);
                            let _ = respond_to.send(false);
                        }
                    }
                },
                DbRequest::GetWorldServer { server_id: _, respond_to } => {
                    if self.pool.is_none() {
                        let _ = respond_to.send(Ok(Some("127.0.0.1".to_string())));
                        continue;
                    }
                    let res = sqlx::query_as::<_, WorldIpRow>("SELECT ip_address FROM world_servers WHERE id = $1")
                        .bind(1)
                        .fetch_optional(&**self.pool.as_ref().unwrap())
                        .await;
                    
                    let reply = res.map(|opt| opt.map(|r| r.ip_address));
                    let _ = respond_to.send(reply);
                },
                DbRequest::VerifySession { account_id, session_key: _, respond_to } => {
                    if self.pool.is_none() {
                        let _ = respond_to.send(Some(account_id));
                        continue;
                    }
                    let session_key_str = format!("{:010}", 0); // Placeholder
                    let res = sqlx::query_as::<_, AccountIdRow>("SELECT id FROM account WHERE id = $1 AND ls_session_key = $2")
                        .bind(account_id)
                        .bind(&session_key_str)
                        .fetch_optional(&**self.pool.as_ref().unwrap())
                        .await;
                    
                    match res {
                        Ok(Some(row)) => { let _ = respond_to.send(Some(row.id)); },
                        Ok(None) => { let _ = respond_to.send(None); },
                        Err(e) => {
                            log::error!("DB VerifySession Error: {}", e);
                            let _ = respond_to.send(None);
                        }
                    }
                },
                DbRequest::GetCharacters { account_id, respond_to } => {
                    if self.pool.is_none() {
                        let mut c = Character::default();
                        c.id = 1;
                        c.account_id = account_id;
                        c.name = "TestChar".to_string();
                        c.level = 1;
                        let _ = respond_to.send(vec![c]);
                        continue;
                    }
                    let res = sqlx::query_as::<_, Character>(
                        "SELECT id, account_id, name, last_name, zone_id, zone_instance, \
                         y, x, z, heading, gender, race, class, level, exp, points as practice_points, \
                         mana, cur_hp, endurance, str, sta, cha, dex, int, agi, wis, \
                         face, hair_style, hair_color, beard, beard_color, eye_color_1, eye_color_2, \
                         drakkin_heritage, drakkin_tattoo, drakkin_details, deity \
                         FROM character_data WHERE account_id = $1"
                    )
                    .bind(account_id)
                    .fetch_all(&**self.pool.as_ref().unwrap())
                    .await;

                    match res {
                        Ok(chars) => { let _ = respond_to.send(chars); },
                        Err(e) => {
                            log::error!("DB GetCharacters Error: {}", e);
                            let _ = respond_to.send(vec![]); 
                        }
                    }
                },
                DbRequest::GetCharacterZone { char_id: _, respond_to } => {
                    if self.pool.is_none() {
                        let _ = respond_to.send(Some("freportn".to_string()));
                        continue;
                    }
                    #[derive(FromRow)]
                    struct ZoneRow { short_name: String }
                    
                    let res = sqlx::query_as::<_, ZoneRow>("SELECT short_name FROM character_data cd JOIN zone z ON cd.zone_id = z.id WHERE cd.id = $1")
                        .bind(1)
                        .fetch_optional(&**self.pool.as_ref().unwrap())
                        .await;
                    
                    match res {
                        Ok(Some(row)) => { let _ = respond_to.send(Some(row.short_name)); },
                        Ok(None) => { let _ = respond_to.send(None); },
                        Err(e) => {
                            log::error!("DB GetCharacterZone Error: {}", e);
                            let _ = respond_to.send(None);
                        }
                    }
                }
            }
        }
        log::info!("DbWorker stopped");
    }
}
