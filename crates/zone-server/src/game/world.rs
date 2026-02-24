use slotmap::{SlotMap, new_key_type};
use tokio::sync::{mpsc, oneshot};
use std::collections::{HashMap, HashSet};

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
    pub merchant_id: i32, // For identifying merchant NPCs
    pub npc_type_id: i32, // For loading merchant inventory
}
#[derive(Debug, Clone)]
pub enum WorldEvent {
    ChatMessage {
        source_id: EntityId,
        sender_name: String,
        channel: u32,
        message: String,
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
        respond_to: oneshot::Sender<Result<(u32, u32)>>, // Returns (item_id, base_price)
    },
    SellItem {
        char_id: i32,
        npc_id: EntityId,
        item_id: i32,
        quantity: i32,
        respond_to: oneshot::Sender<Result<u32>>, // Returns sell_price
    },
}

use crate::game::merchant::{MerchantManager, MerchantItem};
use anyhow::Result;

// --- Spatial Grid ---
const GRID_CELL_SIZE: f32 = 100.0; // 100 units per cell

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
        // Query 3x3 grid around the entity
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
    db_pool: sqlx::PgPool,
    loot_manager: super::loot::LootManager,
    merchant_manager: super::merchant::MerchantManager,
    rx: mpsc::Receiver<WorldCommand>,
}

impl WorldManager {
    pub fn new(db_pool: sqlx::PgPool) -> (Self, mpsc::Sender<WorldCommand>) {
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
        
        // Initial NPC load
        if let Err(e) = self.load_npcs(zone_id).await {
            log::error!("Failed to load NPCs for zone {}: {}", zone_id, e);
        }

        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                WorldCommand::Enter { name, pos, event_tx, respond_to } => {
                    let entity = Entity { 
                        name, 
                        pos,
                        race: 1,
                        class: 1,
                        level: 1,
                        cur_hp: 100,
                        max_hp: 100,
                        loottable_id: 0,
                        merchant_id: 0,
                        npc_type_id: 0,
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
                            if neighbor_id != id { // Don't include self
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
                        log::debug!("Entity {:?} took {} damage, health now {}", id, damage, entity.cur_hp);
                        
                        if entity.cur_hp == 0 {
                            log::info!("Entity {:?} ({}) died.", id, entity.name);
                            if entity.loottable_id > 0 {
                                let loot_manager = self.loot_manager.clone();
                                let loottable_id = entity.loottable_id;
                                tokio::spawn(async move {
                                    match loot_manager.roll_loot(loottable_id).await {
                                        Ok(items) => {
                                            let items: Vec<i32> = items;
                                            if !items.is_empty() {
                                                log::info!("Loot rolled for {:?}: {:?}", loottable_id, items);
                                                // TODO: Spawn corpse with items
                                            }
                                        }
                                        Err(e) => log::error!("Failed to roll loot for {}: {}", loottable_id, e),
                                    }
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
                                    source_id,
                                    sender_name: sender_name.clone(),
                                    channel,
                                    message: message.clone(),
                                }).await;
                            }
                        }
                    }
                }
                WorldCommand::GetMerchantInventory { npc_id, respond_to } => {
                    if let Some(entity) = self.entities.get(npc_id) {
                        let result: Result<Vec<MerchantItem>> = self.merchant_manager.get_merchant_inventory(entity.npc_type_id).await
                            .map(|items| items.clone());
                        let _ = respond_to.send(result);
                    }
                }
                WorldCommand::BuyItem { char_id: _, npc_id, item_slot, quantity: _, respond_to } => {
                    if let Some(entity) = self.entities.get(npc_id) {
                        let npc_type_id = entity.npc_type_id;
                        let mut merchant_manager = self.merchant_manager.clone();
                        tokio::spawn(async move {
                            let result = async {
                                let items = merchant_manager.get_merchant_inventory(npc_type_id).await?.clone();
                                if let Some(item) = items.iter().find(|i| i.slot == item_slot) {
                                    let price = merchant_manager.get_item_price(item.item_id).await?;
                                    Ok((item.item_id as u32, price))
                                } else {
                                    Err(anyhow::anyhow!("Item not found in merchant inventory"))
                                }
                            }.await;
                            let _ = respond_to.send(result);
                        });
                    } else {
                        let _ = respond_to.send(Err(anyhow::anyhow!("Merchant NPC not found")));
                    }
                }
                WorldCommand::SellItem { char_id: _, npc_id, item_id, quantity: _, respond_to } => {
                    if let Some(_) = self.entities.get(npc_id) {
                         let mut merchant_manager = self.merchant_manager.clone();
                         tokio::spawn(async move {
                             let result = async {
                                 let price = merchant_manager.get_item_price(item_id).await?;
                                 let sell_price = merchant_manager.calculate_sell_price(item_id, price, 100); // 100 cha stub
                                 Ok(sell_price)
                             }.await;
                             let _ = respond_to.send(result);
                         });
                    } else {
                        let _ = respond_to.send(Err(anyhow::anyhow!("Merchant NPC not found")));
                    }
                }
            }
        }
    }

    async fn load_npcs(&mut self, zone_id: i32) -> Result<(), sqlx::Error> {
        log::info!("Loading NPCs for zone {}...", zone_id);
        
        // Simplified query for NPC spawns in the PEQ schema
        // We join spawn2 -> spawngroup -> spawnentry -> npc_types
        // Note: In real EQEmu there are more nuances (time of day, chance, etc.)
        let rows = sqlx::query(
            r#"
            SELECT 
                n.id as npc_type_id, n.name, n.race, n.class, n.level, n.loottable_id, n.merchant_id,
                s2.x, s2.y, s2.z, s2.heading
            FROM spawn2 s2
            JOIN spawngroup sg ON s2."spawngroupID" = sg.id
            JOIN spawnentry se ON sg.id = se."spawngroupID"
            JOIN npc_types n ON se."npcID" = n.id
            WHERE s2.zone = (SELECT short_name FROM zone WHERE zoneidnumber = $1 LIMIT 1)
            "#
        )
        .bind(zone_id as i64)
        .fetch_all(&self.db_pool)
        .await?;

        for row in rows {
            use sqlx::Row;
            let entity = Entity {
                name: row.get("name"),
                pos: Position {
                    x: row.get("x"),
                    y: row.get("y"),
                    z: row.get("z"),
                    heading: row.get("heading"),
                },
                race: row.get::<i32, _>("race") as u32,
                class: row.get::<i32, _>("class") as u8,
                level: row.get::<i32, _>("level") as u8,
                cur_hp: 100, // TODO: Load actual HP from database if available
                max_hp: 100,
                loottable_id: row.get::<i32, _>("loottable_id"),
                merchant_id: row.get::<i32, _>("merchant_id"),
                npc_type_id: row.get::<i32, _>("npc_type_id"),
            };
            let id = self.entities.insert(entity.clone());
            self.spatial.insert(id, &entity.pos);
            log::debug!("Spawned NPC: {} at ({}, {}, {})", entity.name, entity.pos.x, entity.pos.y, entity.pos.z);
        }

        log::info!("Loaded {} NPCs.", self.entities.len());
        Ok(())
    }
}
