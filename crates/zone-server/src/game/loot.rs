use sqlx::{PgPool, Row};
use rand::Rng;

#[derive(Debug, Clone)]
pub struct LootDrop {
    pub item_id: i32,
    pub chance: f32,
}

#[derive(Clone)]
pub struct LootManager {
    db_pool: Option<PgPool>,
}

impl LootManager {
    pub fn new(db_pool: Option<PgPool>) -> Self {
        Self { db_pool }
    }

    /// Rolls for loot based on a loottable_id.
    pub async fn roll_loot(&self, loottable_id: i32) -> Result<Vec<i32>, sqlx::Error> {
        if self.db_pool.is_none() {
            return Ok(vec![]);
        }
        let pool = self.db_pool.as_ref().unwrap();
        let mut dropped_items = Vec::new();

        // 1. Get loottable entries
        let table_entries = sqlx::query(
            "SELECT lootdrop_id, probability, multiplier FROM loottable_entries WHERE loottable_id = $1"
        )
        .bind(loottable_id)
        .fetch_all(pool)
        .await?;

        for row in table_entries {
            let lootdrop_id: i32 = row.get("lootdrop_id");
            let probability: f32 = row.get("probability");
            let multiplier: i32 = row.try_get("multiplier").unwrap_or(1);

            // Roll for this group without binding RNG to a variable that persists across awaits
            let dropped_group = rand::thread_rng().gen_range(0.0..100.0) <= probability;

            if dropped_group {
                for _ in 0..multiplier {
                    let items_in_drop = sqlx::query(
                        "SELECT item_id, chance FROM lootdrop_entries WHERE lootdrop_id = $1"
                    )
                    .bind(lootdrop_id)
                    .fetch_all(pool)
                    .await?;

                    for item_row in items_in_drop {
                        let item_id: i32 = item_row.get("item_id");
                        let chance: f32 = item_row.get("chance");

                        // Roll for each item
                        if rand::thread_rng().gen_range(0.0..100.0) <= chance {
                            dropped_items.push(item_id);
                        }
                    }
                }
            }
        }

        if !dropped_items.is_empty() {
            log::info!("Loot rolled for loottable {}: {:?}", loottable_id, dropped_items);
        }

        Ok(dropped_items)
    }
}
