use sqlx::{PgPool, Row};
use rand::Rng;

#[derive(Debug, Clone)]
pub struct LootDrop {
    pub item_id: i32,
    pub chance: f32,
}

#[derive(Clone)]
pub struct LootManager {
    db_pool: PgPool,
}

impl LootManager {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    pub async fn roll_loot(&self, loottable_id: i32) -> Result<Vec<i32>, sqlx::Error> {
        let mut dropped_items = Vec::new();

        // 1. Get loottable entries (probability of the entire table/group)
        // Note: Simplified logic. In real EQEmu, there are multiple layers and "multiplier" logic.
        let entries = sqlx::query(
            r#"
            SELECT id, probability 
            FROM loottable_entries 
            WHERE loottable_id = $1
            "#,
        )
        .bind(loottable_id)
        .fetch_all(&self.db_pool)
        .await?;

        for entry in entries {
            let id: i32 = entry.get("id");
            let probability: f32 = entry.get("probability");

            if rand::thread_rng().gen::<f32>() * 100.0 <= probability {
                // 2. Get loot drops for this entry/group
                let drops = sqlx::query(
                    r#"
                    SELECT item_id, chance 
                    FROM lootdrop_entries 
                    WHERE lootdrop_id = (SELECT lootdrop_id FROM loottable_entries WHERE id = $1 LIMIT 1)
                    "#,
                )
                .bind(id)
                .fetch_all(&self.db_pool)
                .await?;

                for drop in drops {
                    let item_id: i32 = drop.get("item_id");
                    let chance: f32 = drop.get("chance");

                    if rand::thread_rng().gen::<f32>() * 100.0 <= chance {
                        dropped_items.push(item_id);
                    }
                }
            }
        }

        Ok(dropped_items)
    }
}
