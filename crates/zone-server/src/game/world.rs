use slotmap::{SlotMap, new_key_type};
use tokio::sync::{mpsc, oneshot};
use std::collections::{HashMap, HashSet};
use crate::game::merchant::{MerchantManager, MerchantItem};
use anyhow::Result;
use rand::Rng;

// --- Entity ID (Generational Index) ---
new_key_type! {
    pub struct EntityId;
}

// --- Components ---
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub heading: f32,
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub name: String,
    pub pos: Position,
    pub race: u32,
    pub class: u8,
    pub level: u8,
    pub cur_hp: u32,
    pub max_hp: u32,
    pub loottable_id: i32,
    pub merchant_id: i32,
    pub npc_type_id: i32,
    pub spawn_pos: Position,
    pub is_npc: bool,
}

#[derive(Debug, Clone)]
pub enum WorldEvent {
    ChatMessage {
        source_id: EntityId,
        sender_name: String,
        channel: u32,
        message: String,
    },
    Teleport {
        pos: Position,
    },
}

// --- Messages ---
#[derive(Debug)]
pub enum WorldCommand {
    Enter {
        name: String,
        pos: Position,
        event_tx: mpsc::Sender<WorldEvent>,
        respond_to: oneshot::Sender<EntityId>,
    },
    Move {
        id: EntityId,
        to_pos: Position,
    },
    GetVisible {
        id: EntityId,
        respond_to: oneshot::Sender<Vec<(EntityId, Entity)>>,
    },
    Remove {
        id: EntityId,
    },
    ApplyDamage {
        id: EntityId,
        damage: i32,
        source_id: EntityId,
    },
    BroadcastChatMessage {
        source_id: EntityId,
        channel: u32,
        message: String,
    },
    GetMerchantInventory {
        npc_id: EntityId,
        respond_to: oneshot::Sender<Result<Vec<MerchantItem>>>,
    },
    BuyItem {
        char_id: i32,
        npc_id: EntityId,
        item_slot: i32,
        quantity: i32,
        respond_to: oneshot::Sender<Result<(u32, u32)>>,
    },
    SellItem {
        char_id: i32,
        npc_id: EntityId,
        item_id: i32,
        quantity: i32,
        respond_to: oneshot::Sender<Result<u32>>,
    },
    SpawnNpc {
        npc_type_id: i32,
        pos: Position,
    },
    TeleportEntity {
        id: EntityId,
        pos: Position,
    },
}

// --- Spatial Grid ---
const GRID_CELL_SIZE: f32 = 100.0;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
struct GridKey(i32, i32);

struct SpatialGrid {
    cells: HashMap<GridKey, HashSet<EntityId>>,
}

impl SpatialGrid {
    fn new() -> Self {
        Self { cells: HashMap::new() }
    }

    fn get_key(pos: &Position) -> GridKey {
        GridKey(
            (pos.x / GRID_CELL_SIZE).floor() as i32,
            (pos.y / GRID_CELL_SIZE).floor() as i32,
        )
    }

    fn insert(&mut self, id: EntityId, pos: &Position) {
        let key = Self::get_key(pos);
        self.cells.entry(key).or_default().insert(id);
    }

    fn remove(&mut self, id: EntityId, pos: &Position) {
        let key = Self::get_key(pos);
        if let Some(cell) = self.cells.get_mut(&key) {
            cell.remove(&id);
            if cell.is_empty() {
                self.cells.remove(&key);
            }
        }
    }

    fn move_entity(&mut self, id: EntityId, old_pos: &Position, new_pos: &Position) {
        let old_key = Self::get_key(old_pos);
        let new_key = Self::get_key(new_pos);
        if old_key != new_key {
            self.remove(id, old_pos);
            self.insert(id, new_pos);
        }
    }

    fn query(&self, pos: &Position) -> Vec<EntityId> {
        let center = Self::get_key(pos);
        let mut results = Vec::new();
        for x in -1..=1 {
            for y in -1..=1 {
                let key = GridKey(center.0 + x, center.1 + y);
                if let Some(cell) = self.cells.get(&key) {
                    results.extend(cell.iter().cloned());
                }
            }
        }
        results
    }
}

// --- World Manager Actor ---
pub struct WorldManager {
    entities: SlotMap<EntityId, Entity>,
    event_txs: HashMap<EntityId, mpsc::Sender<WorldEvent>>,
    spatial: SpatialGrid,
    db_pool: Option<sqlx::PgPool>,
    loot_manager: super::loot::LootManager,
    merchant_manager: super::merchant::MerchantManager,
    rx: mpsc::Receiver<WorldCommand>,
}

impl WorldManager {
    pub fn new(db_pool: Option<sqlx::PgPool>) -> (Self, mpsc::Sender<WorldCommand>) {
        let (tx, rx) = mpsc::channel(100);
        (
            Self {
                entities: SlotMap::with_key(),
                event_txs: HashMap::new(),
                spatial: SpatialGrid::new(),
                loot_manager: super::loot::LootManager::new(db_pool.clone()),
                merchant_manager: super::merchant::MerchantManager::new(db_pool.clone()),
                db_pool,
                rx,
            },
            tx,
        )
    }

    pub async fn run(mut self, zone_id: i32) {
        log::info!("WorldManager started for zone_id: {}.", zone_id);
        let mut world_tick = tokio::time::interval(tokio::time::Duration::from_secs(5));
        
        if let Err(e) = self.load_npcs(zone_id).await {
            log::error!("Failed to load NPCs for zone {}: {}", zone_id, e);
        }

        loop {
            tokio::select! {
                _ = world_tick.tick() => {
                    self.handle_world_tick().await;
                }
                Some(cmd) = self.rx.recv() => {
                    match cmd {
                        WorldCommand::Enter { name, pos, event_tx, respond_to } => {
                            let entity = Entity { 
                                name: name.clone(), 
                                pos,
                                race: 1, class: 1, level: 1,
                                cur_hp: 100, max_hp: 100,
                                loottable_id: 0, merchant_id: 0, npc_type_id: 0,
                                spawn_pos: pos, is_npc: false,
                            };
                            let id = self.entities.insert(entity);
                            self.event_txs.insert(id, event_tx);
                            self.spatial.insert(id, &pos);
                            let _ = respond_to.send(id);
                        }
                        WorldCommand::Move { id, to_pos } => {
                            if let Some(entity) = self.entities.get_mut(id) {
                                self.spatial.move_entity(id, &entity.pos, &to_pos);
                                entity.pos = to_pos;
                            }
                        }
                        WorldCommand::GetVisible { id, respond_to } => {
                            let mut visible_entities = Vec::new();
                            if let Some(entity) = self.entities.get(id) {
                                let neighbor_ids = self.spatial.query(&entity.pos);
                                for neighbor_id in neighbor_ids {
                                    if neighbor_id != id {
                                        if let Some(neighbor) = self.entities.get(neighbor_id) {
                                            visible_entities.push((neighbor_id, neighbor.clone()));
                                        }
                                    }
                                }
                            }
                            let _ = respond_to.send(visible_entities);
                        }
                        WorldCommand::Remove { id } => {
                             if let Some(entity) = self.entities.remove(id) {
                                 self.spatial.remove(id, &entity.pos);
                                 self.event_txs.remove(&id);
                             }
                        }
                        WorldCommand::ApplyDamage { id, damage, source_id: _ } => {
                            if let Some(entity) = self.entities.get_mut(id) {
                                entity.cur_hp = entity.cur_hp.saturating_sub(damage.max(0) as u32);
                                if entity.cur_hp == 0 {
                                    if entity.loottable_id > 0 {
                                        let loot_manager = self.loot_manager.clone();
                                        let loottable_id = entity.loottable_id;
                                        tokio::spawn(async move {
                                            let _ = loot_manager.roll_loot(loottable_id).await;
                                        });
                                    }
                                }
                            }
                        }
                        WorldCommand::BroadcastChatMessage { source_id, channel, message } => {
                            if let Some(entity) = self.entities.get(source_id) {
                                let sender_name = entity.name.clone();
                                let observers = self.spatial.query(&entity.pos);
                                for obs_id in observers {
                                    if let Some(tx) = self.event_txs.get(&obs_id) {
                                        let _ = tx.send(WorldEvent::ChatMessage {
                                            source_id, sender_name: sender_name.clone(),
                                            channel, message: message.clone(),
                                        }).await;
                                    }
                                }
                            }
                        }
                        WorldCommand::GetMerchantInventory { npc_id, respond_to } => {
                            if let Some(entity) = self.entities.get(npc_id) {
                                let res = self.merchant_manager.get_merchant_inventory(entity.npc_type_id).await.map(|i| i.clone());
                                let _ = respond_to.send(res);
                            }
                        }
                        WorldCommand::BuyItem { char_id: _, npc_id, item_slot, quantity: _, respond_to } => {
                             if let Some(entity) = self.entities.get(npc_id) {
                                let npc_type_id = entity.npc_type_id;
                                let mut mm = self.merchant_manager.clone();
                                tokio::spawn(async move {
                                    let res = async {
                                        let items = mm.get_merchant_inventory(npc_type_id).await?.clone();
                                        if let Some(item) = items.iter().find(|i| i.slot == item_slot) {
                                            let price = mm.get_item_price(item.item_id).await?;
                                            Ok((item.item_id as u32, price))
                                        } else {
                                            Err(anyhow::anyhow!("Item not found"))
                                        }
                                    }.await;
                                    let _ = respond_to.send(res);
                                });
                            }
                        }
                        WorldCommand::SellItem { char_id: _, npc_id, item_id, quantity: _, respond_to } => {
                            if let Some(_) = self.entities.get(npc_id) {
                                let mut mm = self.merchant_manager.clone();
                                tokio::spawn(async move {
                                    let res = async {
                                        let p = mm.get_item_price(item_id).await?;
                                        Ok(mm.calculate_sell_price(item_id, p, 100))
                                    }.await;
                                    let _ = respond_to.send(res);
                                });
                            }
                        }
                        WorldCommand::SpawnNpc { npc_type_id, pos } => {
                            if let Err(e) = self.spawn_npc_at(npc_type_id, pos).await {
                                log::error!("AI Spawn failed for {}: {}", npc_type_id, e);
                            }
                        }
                        WorldCommand::TeleportEntity { id, pos } => {
                            if let Some(entity) = self.entities.get_mut(id) {
                                let old_pos = entity.pos;
                                entity.pos = pos;
                                self.spatial.move_entity(id, &old_pos, &pos);
                                log::info!("Teleported entity {} to {:?}", entity.name, pos);
                                
                                if let Some(tx) = self.event_txs.get(&id) {
                                    let _ = tx.try_send(WorldEvent::Teleport { pos });
                                }
                            }
                        }
                    }
                }
                else => break,
            }
        }
    }

    async fn handle_world_tick(&mut self) {
        let entity_ids: Vec<EntityId> = self.entities.keys().collect();
        for id in entity_ids {
            if let Some(entity) = self.entities.get(id) {
                if entity.is_npc {
                    self.handle_npc_roaming(id).await;
                    self.handle_npc_aggro(id).await;
                }
            }
        }
    }

    async fn handle_npc_roaming(&mut self, id: EntityId) {
        let mut rng = rand::thread_rng();
        if rng.gen_bool(0.2) {
            if let Some(entity) = self.entities.get_mut(id) {
                let dx = rng.gen_range(-10.0..10.0);
                let dy = rng.gen_range(-10.0..10.0);
                let mut new_pos = entity.pos;
                new_pos.x += dx;
                new_pos.y += dy;
                let dist_sq = (new_pos.x - entity.spawn_pos.x).powi(2) + (new_pos.y - entity.spawn_pos.y).powi(2);
                if dist_sq < 2500.0 {
                    self.spatial.move_entity(id, &entity.pos, &new_pos);
                    entity.pos = new_pos;
                }
            }
        }
    }

    async fn handle_npc_aggro(&mut self, id: EntityId) {
        if let Some(entity) = self.entities.get(id) {
            let nearby = self.spatial.query(&entity.pos);
            for target_id in nearby {
                if target_id == id { continue; }
                if let Some(target) = self.entities.get(target_id) {
                    if !target.is_npc {
                        let dist_sq = (target.pos.x - entity.pos.x).powi(2) + (target.pos.y - entity.pos.y).powi(2);
                        if dist_sq < 1600.0 {
                            log::info!("NPC {} aggroed on player {}", entity.name, target.name);
                        }
                    }
                }
            }
        }
    }

    async fn load_npcs(&mut self, zone_id: i32) -> Result<(), sqlx::Error> {
        if self.db_pool.is_none() { return Ok(()); }
        let rows = sqlx::query(
            r#"SELECT n.id as npc_type_id, n.name, n.race, n.class, n.level, n.loottable_id, n.merchant_id,
               s2.x, s2.y, s2.z, s2.heading FROM spawn2 s2
               JOIN spawngroup sg ON s2."spawngroupID" = sg.id
               JOIN spawnentry se ON sg.id = se."spawngroupID"
               JOIN npc_types n ON se."npcID" = n.id
               WHERE s2.zone = (SELECT short_name FROM zone WHERE zoneidnumber = $1 LIMIT 1)"#
        )
        .bind(zone_id as i64)
        .fetch_all(self.db_pool.as_ref().unwrap())
        .await?;

        for row in rows {
            use sqlx::Row;
            let pos = Position { x: row.get("x"), y: row.get("y"), z: row.get("z"), heading: row.get("heading") };
            let entity = Entity {
                name: row.get("name"), pos,
                race: row.get::<i32, _>("race") as u32,
                class: row.get::<i16, _>("class") as u8,
                level: row.get::<i16, _>("level") as u8,
                cur_hp: 100, max_hp: 100,
                loottable_id: row.get::<i64, _>("loottable_id") as i32,
                merchant_id: row.get::<i64, _>("merchant_id") as i32,
                npc_type_id: row.get::<i32, _>("npc_type_id"),
                spawn_pos: pos, is_npc: true,
            };
            let id = self.entities.insert(entity.clone());
            self.spatial.insert(id, &entity.pos);
        }
        Ok(())
    }

    pub async fn spawn_npc_at(&mut self, npc_type_id: i32, pos: Position) -> Result<(), sqlx::Error> {
        if self.db_pool.is_none() {
            log::info!("MOCK: Spawning NPC type {} at {:?}", npc_type_id, pos);
            return Ok(());
        }
        
        let row = sqlx::query(
            r#"SELECT name, race, class, level, loottable_id, merchant_id 
               FROM npc_types WHERE id = $1"#
        )
        .bind(npc_type_id)
        .fetch_optional(self.db_pool.as_ref().unwrap())
        .await?;

        if let Some(row) = row {
            use sqlx::Row;
            let entity = Entity {
                name: row.get("name"),
                pos,
                race: row.get::<i32, _>("race") as u32,
                class: row.get::<i16, _>("class") as u8,
                level: row.get::<i16, _>("level") as u8,
                cur_hp: 100,
                max_hp: 100,
                loottable_id: row.get::<i64, _>("loottable_id") as i32,
                merchant_id: row.get::<i64, _>("merchant_id") as i32,
                npc_type_id,
                spawn_pos: pos,
                is_npc: true,
            };
            let id = self.entities.insert(entity.clone());
            self.spatial.insert(id, &entity.pos);
            log::info!("AI Spawned NPC: {} ({}) at {:?}", entity.name, npc_type_id, pos);
        } else {
            log::warn!("NPC type {} not found in database", npc_type_id);
        }
        
        Ok(())
    }
}
