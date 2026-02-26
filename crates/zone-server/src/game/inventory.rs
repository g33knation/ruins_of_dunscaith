use tokio::sync::{mpsc, oneshot};
use std::collections::HashMap;

#[derive(Debug, sqlx::FromRow)]
pub struct InventoryItem {
    pub item_id: i32,
    pub slot_id: i16,
    pub quantity: i16,
    // Add other fields as needed
}

// Message Types
#[derive(Debug)]
pub enum InventoryRequest {
    Load { 
        char_id: u32, 
        respond_to: oneshot::Sender<Vec<InventoryItem>> 
    },
    // Future: Update, Move, etc.
}

pub struct InventoryManager {
    rx: mpsc::Receiver<InventoryRequest>,
    pool: Option<sqlx::PgPool>,
    // Cache?
    // cache: HashMap<u32, Vec<InventoryItem>>,
}

impl InventoryManager {
    pub fn new(pool: Option<sqlx::PgPool>) -> (Self, mpsc::Sender<InventoryRequest>) {
        let (tx, rx) = mpsc::channel(100);
        (Self { rx, pool }, tx)
    }

    pub async fn run(mut self) {
        log::info!("InventoryManager started.");
        while let Some(msg) = self.rx.recv().await {
             match msg {
                 InventoryRequest::Load { char_id, respond_to } => {
                     if self.pool.is_none() {
                         let _ = respond_to.send(vec![]); // Mock empty inventory
                         continue;
                     }
                     let res = sqlx::query_as::<_, InventoryItem>(
                         r#"
                         SELECT 
                            item_id, 
                            slot_id, 
                            quantity
                         FROM inventory_items 
                         WHERE char_id = $1
                         "#
                     )
                     .bind(char_id as i32)
                     .fetch_all(self.pool.as_ref().unwrap())
                     .await;
                     
                     match res {
                         Ok(items) => {
                             let _ = respond_to.send(items);
                         },
                         Err(e) => {
                             log::error!("Inventory Load Error for {}: {}", char_id, e);
                             let _ = respond_to.send(vec![]); // Return empty on error for now
                         }
                     }
                 }
             }
        }
    }
}
