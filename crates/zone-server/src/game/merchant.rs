use sqlx::PgPool;
use anyhow::{Result, Context};
use log::{info, error};
use std::collections::HashMap;
use crate::game::world::EntityId;

#[derive(Debug, Clone)]
pub struct MerchantItem {
    pub item_id: i32,
    pub slot: i32,
    pub probability: f32,
}

#[derive(Clone)]
pub struct MerchantManager {
    db_pool: PgPool,
    // Cache for merchant inventories: npc_type_id -> items
    inventory_cache: HashMap<i32, Vec<MerchantItem>>,
}

impl MerchantManager {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            db_pool,
            inventory_cache: HashMap::new(),
        }
    }

    pub async fn get_merchant_inventory(&mut self, npc_type_id: i32) -> Result<&Vec<MerchantItem>> {
        if !self.inventory_cache.contains_key(&npc_type_id) {
            let items = self.load_inventory(npc_type_id).await?;
            self.inventory_cache.insert(npc_type_id, items);
        }
        Ok(self.inventory_cache.get(&npc_type_id).unwrap())
    }

    async fn load_inventory(&self, npc_type_id: i32) -> Result<Vec<MerchantItem>> {
        info!("Loading merchant inventory for NPC type {}", npc_type_id);

        // 1. Get merchant_id from npc_types
        let merchant_id: Option<i32> = sqlx::query_scalar(
            "SELECT merchant_id FROM npc_types WHERE id = $1"
        )
        .bind(npc_type_id)
        .fetch_optional(&self.db_pool)
        .await
        .context("Failed to query merchant_id from npc_types")?;

        let Some(m_id) = merchant_id else {
            return Ok(Vec::new());
        };

        if m_id == 0 {
            return Ok(Vec::new());
        }

        // 2. Load items from merchantlist
        let rows = sqlx::query(
            "SELECT item, slot, probability FROM merchantlist WHERE merchant_id = $1 ORDER BY slot ASC"
        )
        .bind(m_id)
        .fetch_all(&self.db_pool)
        .await
        .context("Failed to query merchantlist")?;

        let mut items = Vec::new();
        for row in rows {
            use sqlx::Row;
            items.push(MerchantItem {
                item_id: row.get("item"),
                slot: row.get("slot"),
                probability: row.get("probability"),
            });
        }

        Ok(items)
    }

    pub async fn get_item_price(&self, item_id: i32) -> Result<u32> {
        let price: Option<i32> = sqlx::query_scalar(
            "SELECT price FROM items WHERE id = $1"
        )
        .bind(item_id)
        .fetch_optional(&self.db_pool)
        .await
        .context("Failed to query item price")?;

        Ok(price.unwrap_or(0) as u32)
    }

    pub fn calculate_sell_price(&self, _item_id: i32, base_price: u32, _charisma: i32) -> u32 {
        // Basic implementation for now: 20% markup/markdown
        // Original EqEmu uses a complex formula involving charisma and faction
        (base_price as f32 * 0.8) as u32
    }

    pub fn calculate_buy_price(&self, _item_id: i32, base_price: u32, _charisma: i32) -> u32 {
        (base_price as f32 * 1.1) as u32
    }
}
