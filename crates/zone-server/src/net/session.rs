use tokio::sync::mpsc;
use tokio::sync::oneshot;
use std::net::SocketAddr;
use std::io::Cursor;
use binrw::{BinRead, BinWrite};
use slotmap::Key;
use crate::net::packet::{PlayerProfile, ClientUpdate, Spawn};
use crate::net::key_manager::KeyManagerRequest;
use crate::net::client_socket::{InboundPacket, OutboundPacket};
use shared::net::eq_stream::{parse_eqstream, EQStreamPacket, EqStreamSession as SharedSession, ProcessPacketResult};
use shared::opcodes::OpCode;
use shared::packets::{TargetMouse, CombatDamage, ChannelMessage, MerchantClick, MerchantBuy, MerchantSell, MerchantList};
use crate::game::world::{Position, EntityId, WorldEvent, WorldCommand};
use crate::game::merchant::MerchantItem;
use anyhow::Result;

#[derive(sqlx::FromRow, Debug, Clone)]
struct CharacterDbData {
    id: i32,
    account_id: i32,
    name: String,
    last_name: String,
    level: i32, 
    race: i32,
    class: i32,
    gender: i32,
    deity: i32,
    zone_id: i32,
    zone_instance: i32,
    x: f32,
    y: f32,
    z: f32,
    heading: f32,
    face: i32,
    hair_color: i32,
    hair_style: i32,
    beard: i32,
    beard_color: i32,
    eye_color_1: i32,
    eye_color_2: i32,
    drakkin_heritage: i32,
    drakkin_tattoo: i32,
    drakkin_details: i32,
    str: i32,
    sta: i32,
    dex: i32,
    agi: i32,
    int: i32,
    wis: i32,
    cha: i32,
    cur_hp: i32,
    mana: i32,
    endurance: i32,
    intoxication: i32,
    toxicity: i32,
    hunger_level: i32,
    thirst_level: i32,
    exp: i32,
    aa_points_spent: i32,
    aa_exp: i32,
    aa_points: i32,
    points: i32,
    air_remaining: i32,
    show_helm: i32,
    #[sqlx(rename = "RestTimer")]
    rest_timer: i32,
    platinum: i32,
    gold: i32,
    silver: i32,
    copper: i32,
}

use crate::game::inventory::InventoryRequest; 
// Duplicate imports removed

pub struct ClientSystem {
    addr: SocketAddr,
    db_pool: sqlx::PgPool,
    key_tx: mpsc::Sender<KeyManagerRequest>,
    inv_tx: mpsc::Sender<InventoryRequest>,
    world_tx: mpsc::Sender<WorldCommand>,
    
    // Networking
    session: SharedSession,
    
    // State
    entity_id: Option<EntityId>,
    char_id: Option<i32>,
    pos: Position,
    zone_id: i32,
    level: i16,
    exp: i32,
    platinum: i32,
    gold: i32,
    silver: i32,
    copper: i32,

    rx_inbound: mpsc::Receiver<InboundPacket>,
    tx_outbound: mpsc::Sender<OutboundPacket>,
    rx_world: mpsc::Receiver<WorldEvent>,
    tx_world_event: mpsc::Sender<WorldEvent>,

    // Visibility
    visible_entities: std::collections::HashMap<EntityId, Position>,
    spawn_id_to_entity_id: std::collections::HashMap<u32, EntityId>,

    // Combat
    target_id: Option<u32>,
    auto_attack: bool,
}

impl ClientSystem {
    pub fn new(
        addr: SocketAddr,
        db_pool: sqlx::PgPool,
        key_tx: mpsc::Sender<KeyManagerRequest>,
        inv_tx: mpsc::Sender<InventoryRequest>,
        world_tx: mpsc::Sender<WorldCommand>,
        rx_inbound: mpsc::Receiver<InboundPacket>,
        tx_outbound: mpsc::Sender<OutboundPacket>,
    ) -> Self {
        let mut session = SharedSession::new(0); // Session ID will be set by handshake
        session.crc_key = 0; // Zone usually uses 0 or 0xFFFFFFFF
        session.enable_combined();
        session.enable_compression();

        let (tx_world_event, rx_world) = mpsc::channel(100);

        Self {
            addr,
            db_pool,
            key_tx,
            inv_tx,
            world_tx,
            session,
            entity_id: None,
            char_id: None,
            pos: Position { x: 0.0, y: 0.0, z: 0.0, heading: 0.0 },
            zone_id: 202,
            level: 0,
            exp: 0,
            platinum: 0,
            gold: 0,
            silver: 0,
            copper: 0,
            rx_inbound,
            tx_outbound,
            rx_world,
            tx_world_event,
            visible_entities: std::collections::HashMap::new(),
            spawn_id_to_entity_id: std::collections::HashMap::new(),
            target_id: None,
            auto_attack: false,
        }
    }

    pub async fn run(mut self) {
        log::info!("Zone ClientSystem (Logic) started for {}", self.addr);
        let mut visibility_interval = tokio::time::interval(tokio::time::Duration::from_millis(500));
        let mut combat_interval = tokio::time::interval(tokio::time::Duration::from_millis(1500));
        
        loop {
            tokio::select! {
                Some(packet) = self.rx_inbound.recv() => {
                    match packet {
                        InboundPacket::Raw(data) => {
                            log::debug!("Zone RECV Raw ({} bytes) from {}: {:02X?}", data.len(), self.addr, &data[..std::cmp::min(data.len(), 16)]);
                            self.handle_raw_packet(&data).await;
                        }
                    }
                }
                Some(event) = self.rx_world.recv() => {
                    self.handle_world_event(event).await;
                }
                _ = visibility_interval.tick() => {
                    if self.entity_id.is_some() {
                        self.update_visibility().await;
                    }
                }
                _ = combat_interval.tick() => {
                    if self.auto_attack && self.target_id.is_some() && self.entity_id.is_some() {
                        self.handle_combat_tick().await;
                    }
                }
                else => break,
            }
        }
    }

    async fn handle_world_event(&mut self, event: WorldEvent) {
        match event {
            WorldEvent::ChatMessage { sender_name, channel, message, .. } => {
                log::info!("Chat Event: [{}] {}: {}", channel, sender_name, message);
                let mut cm = ChannelMessage {
                    targetname: [0; 64],
                    sender: [0; 64],
                    language: 0,
                    channel,
                    unknown: [0; 2],
                    skill: 0,
                    message: [0; 256],
                };

                // Copy sender name
                let sender_bytes = sender_name.as_bytes();
                let slen = std::cmp::min(sender_bytes.len(), 64);
                for i in 0..slen { cm.sender[i] = sender_bytes[i] as i8; }

                // Copy message
                let msg_bytes = message.as_bytes();
                let mlen = std::cmp::min(msg_bytes.len(), 256);
                for i in 0..mlen { cm.message[i] = msg_bytes[i] as i8; }

                let mut payload = Vec::new();
                let mut cursor = Cursor::new(&mut payload);
                if let Ok(_) = cm.write(&mut cursor) {
                    self.send_app_packet(OpCode::ChannelMessage, &payload).await;
                }
            }
        }
    }

    async fn handle_raw_packet(&mut self, data: &[u8]) {
        // Handle very short probe/ping packets (1-2 bytes)
        // The EQ client sends these as keep-alive/handshake probes
        // They must be echoed back to complete the connection
        if data.len() <= 2 {
            log::debug!("Zone: Echoing probe packet ({} bytes) to {}", data.len(), self.addr);
            self.send_raw(data.to_vec()).await;
            return;
        }
        
        match parse_eqstream(data) {
            Ok((_, pkt)) => {
                match pkt {
                    EQStreamPacket::SessionRequest(req) => {
                        log::info!("Handling Zone Session Request for {} (SessionID={:08X})", self.addr, req.session_id);
                        self.session.session_id = req.session_id;
                        let response = self.session.handle_session_request(&req);
                        self.send_raw(response).await;
                    }
                    EQStreamPacket::Unknown(transport_op, payload) => {
                         log::debug!("Zone Transport Packet: Op={:04X} Len={}", transport_op, payload.len());
                         self.process_transport_packet(transport_op, &payload).await;
                    }
                    _ => {}
                }
            },
            Err(e) => {
                log::warn!("Zone parse_eqstream Error from {}: {:?}", self.addr, e);
            }
        }
    }

    async fn process_transport_packet(&mut self, transport_opcode: u16, payload: &[u8]) {
        let results = self.session.process_packet(transport_opcode, payload);
        
        for res in results {
            match res {
                ProcessPacketResult::Response(pkt) => self.send_raw(pkt).await,
                ProcessPacketResult::Application(app_op, app_data) => {
                    self.handle_application_packet(app_op, app_data).await;
                }
            }
        }
    }

    async fn handle_application_packet(&mut self, app_opcode: OpCode, data: Vec<u8>) {
         let decompressed: Vec<u8> = match SharedSession::decompress_payload(data).await {
            Ok(d) => d,
            Err(e) => {
                log::error!("Decompression failure: {}", e);
                return;
            }
        };

        match app_opcode {
            OpCode::ZoneEntry | OpCode::ZoneEntry2 | OpCode::ZoneEntry3 => self.handle_zone_entry(&decompressed, app_opcode).await, // OP_ZoneEntry catch-all
            OpCode::ClientUpdate => self.handle_client_update(&decompressed).await, // OP_ClientUpdate
            OpCode::ClientReady => self.handle_client_ready().await, // OP_ClientReady
            OpCode::ReqClientSpawn => self.handle_req_client_spawn().await, // OP_ReqClientSpawn
            OpCode::TargetMouse => self.handle_target_mouse(&decompressed).await,
            OpCode::Attack => self.handle_attack(&decompressed).await,
            OpCode::ChannelMessage => self.handle_channel_message(&decompressed).await,
            OpCode::ShopRequest => self.handle_shop_request(&decompressed).await,
            OpCode::ShopBuy => self.handle_shop_buy(&decompressed).await,
            OpCode::ShopSell => self.handle_shop_sell(&decompressed).await,
            OpCode::ShopEnd => log::info!("Shop End from {}", self.addr),
            _ => log::info!("ClientSystem {} ignored AppOpCode {:?} (Len={})", self.addr, app_opcode, decompressed.len()),
        }
    }

    async fn handle_channel_message(&mut self, payload: &[u8]) {
        let mut cursor = Cursor::new(payload);
        if let Ok(cm) = ChannelMessage::read(&mut cursor) {
            // Extract message string
            let msg_bytes: Vec<u8> = cm.message.iter().take_while(|&&b| b != 0).map(|&b| b as u8).collect();
            let message = String::from_utf8_lossy(&msg_bytes).to_string();
            
            log::info!("Chat RECV: [Channel {}] {}", cm.channel, message);

            if cm.channel == 1 { // Say
                if let Some(source_id) = self.entity_id {
                    let _ = self.world_tx.send(WorldCommand::BroadcastChatMessage {
                        source_id,
                        channel: cm.channel,
                        message,
                    }).await;
                }
            }
        }
    }

    async fn handle_client_update(&mut self, payload: &[u8]) {
        let mut cursor = Cursor::new(payload);
        if let Ok(update) = ClientUpdate::read(&mut cursor) {
             let new_pos = Position {
                 x: update.x_pos,
                 y: update.y_pos,
                 z: update.z_pos,
                 heading: update.heading(),
             };
             self.pos = new_pos.clone();
             
             // Broadcast move to WorldManager
             if let Some(id) = self.entity_id {
                 let _ = self.world_tx.send(WorldCommand::Move {
                     id,
                     to_pos: new_pos,
                 }).await;
             }
        }
    }

    async fn handle_target_mouse(&mut self, payload: &[u8]) {
        let mut cursor = Cursor::new(payload);
        if let Ok(target) = TargetMouse::read(&mut cursor) {
            log::info!("Client {} targeted entity ID {}", self.addr, target.target_id);
            self.target_id = Some(target.target_id);
            
            // Re-broadcast target to client to confirm (RoF2 often echoes OP_TargetMouse back)
            let mut response_buf = Vec::new();
            let mut response_cursor = Cursor::new(&mut response_buf);
            if let Ok(_) = target.write(&mut response_cursor) {
                self.send_app_packet(OpCode::TargetMouse, &response_buf).await;
            }
        }
    }

    async fn handle_attack(&mut self, payload: &[u8]) {
        if payload.len() >= 1 {
            let active = payload[0] != 0;
            log::info!("Client {} auto-attack set to: {}", self.addr, active);
            self.auto_attack = active;
        }
    }

    async fn handle_combat_tick(&mut self) {
        let target_id = self.target_id.unwrap();
        let entity_id = self.entity_id.unwrap();
        
        // Find the EntityId from u32
        // In a real system, we'd have a mapping. For now, we'll try to find it in visible_entities
        let world_entity_id = self.visible_entities.keys()
            .find(|id| id.data().as_ffi() as u32 == target_id);
            
        if let Some(&victim_id) = world_entity_id {
            let damage = 10; // Basic skeleton damage
            
            log::info!("Attacking entity {:?} for {} damage", victim_id, damage);
            
            // 1. Tell WorldManager to apply damage
            let _ = self.world_tx.send(WorldCommand::ApplyDamage {
                id: victim_id,
                damage: damage as i32,
                source_id: entity_id,
            }).await;
            
            // 2. Broadcast Damage packet to client (and others)
            let combat_damage = CombatDamage {
                target_id: target_id as u16,
                source_id: (entity_id.data().as_ffi() as u32) as u16,
                damage_type: 0, // Melee
                spell_id: 0,
                damage: damage as i32,
            };
            
            let mut payload = Vec::new();
            let mut cursor = Cursor::new(&mut payload);
            if let Ok(_) = combat_damage.write(&mut cursor) {
                self.send_app_packet(OpCode::Damage, &payload).await;
            }
        } else {
            log::warn!("Target entity ID {} not in visible range, stopping attack.", target_id);
            self.auto_attack = false;
        }
    }

    async fn handle_zone_entry(&mut self, payload: &[u8], opcode: OpCode) {
        // Debug: Log raw payload bytes (look for Juggs = 4A 75 67 67 73)
        log::info!("ZoneEntry Payload (Op={:?}, {} bytes): {:02X?}", 
            opcode,
            payload.len(), 
            &payload[..64.min(payload.len())]);
        
        // EQEmu RoF2 ClientZoneEntry_Struct:
        // 0x00: uint32 unknown00
        // 0x04: char char_name[64]
        // 0x44: uint32 unknown68
        // 0x48: uint32 unknown72
        
        let mut char_name = String::new();

        if opcode != OpCode::ZoneEntry3 {
            if payload.len() < 68 {
                log::warn!("ZoneEntry payload too short ({} bytes, need 68) from {}", payload.len(), self.addr);
                // Don't return, try Handoff fallback
            } else {
                let name_data = &payload[4..68]; // Name is at offset 4, 64 bytes
            
                // Parse character name from payload (null-terminated string)
                char_name = name_data.iter()
                    .position(|&b| b == 0)
                    .map(|pos| String::from_utf8_lossy(&name_data[..pos]).to_string())
                    .unwrap_or_else(|| String::from_utf8_lossy(&name_data[..64.min(name_data.len())]).to_string())
                    .trim().to_string();
            }
        } else {
            log::info!("Received Large Unknown Packet 0x3747. Skipping payload parse, force checking Handoff...");
        }
        
        if char_name.is_empty() {
            log::warn!("Empty character name in ZoneEntry from {}. Checking Handoff...", self.addr);
            // Handoff Fallback (File)
             if let Ok(content) = std::fs::read_to_string("handoff.txt") {
                let parts: Vec<&str> = content.trim().split('=').collect();
                if parts.len() == 2 {
                    let handoff_ip = parts[0];
                    let handoff_name = parts[1];
                    let client_ip = self.addr.ip().to_string();
                    
                    // Allow match if IPs are identical, OR if one of them is localhost (loopback routing)
                    let public_ip = std::env::var("PUBLIC_IP").unwrap_or("127.0.0.1".to_string());
                    if handoff_ip == client_ip || handoff_ip == "127.0.0.1" || handoff_ip == public_ip || client_ip == "127.0.0.1" {
                         log::info!("Found Handoff for IP {}: Character '{}'", client_ip, handoff_name);
                         char_name = handoff_name.to_string();
                    } else {
                        log::warn!("Handoff IP mismatch: expected {}, got {}", client_ip, handoff_ip);
                    }
                }
            }
        }

        if char_name.is_empty() {
             log::warn!("Failed to resolve Character Name for {}", self.addr);
             return;
        }

        log::info!("ClientSystem {} entering as Character '{}'", self.addr, char_name);

        let char_data_res = sqlx::query_as::<_, CharacterDbData>(
            "SELECT cd.*, cc.platinum, cc.gold, cc.silver, cc.copper FROM character_data cd \
             LEFT JOIN character_currency cc ON cd.id = cc.id WHERE cd.name = $1"
        )
        .bind(&char_name)
        .fetch_optional(&self.db_pool)
        .await;

        match char_data_res {
            Ok(Some(c)) => {
                self.process_character_entry(c.clone(), &c.name).await;
            },
            Ok(None) => {
                // Fallback: Character name parsing failed, try to find the most recent character
                log::warn!("Character '{}' not found, trying fallback lookup...", char_name);
                
                let fallback_res = sqlx::query_as::<_, CharacterDbData>(
                    "SELECT cd.*, cc.platinum, cc.gold, cc.silver, cc.copper FROM character_data cd \
                     LEFT JOIN character_currency cc ON cd.id = cc.id ORDER BY cd.last_login DESC LIMIT 1"
                )
                .fetch_optional(&self.db_pool)
                .await;
                
                match fallback_res {
                    Ok(Some(c)) => {
                        log::info!("Fallback found character: '{}'", c.name);
                        self.process_character_entry(c.clone(), &c.name).await;
                    },
                    Ok(None) => log::error!("No characters exist in database!"),
                    Err(e) => log::error!("DB Error in fallback lookup: {}", e),
                }
            },
            Err(e) => log::error!("DB Error fetching character '{}': {}", char_name, e),
        }
    }
    
    async fn process_character_entry(&mut self, c: CharacterDbData, char_name: &str) {
        self.char_id = Some(c.id);
        let mut char_data = c.clone();
        if char_data.zone_id == 189 {
             log::info!("Auto-Aligning Character '{}' from Zone 189 to 202 (PoK)", char_name);
             char_data.zone_id = 202;
             char_data.x = -326.0;
             char_data.y = 0.0;
             char_data.z = -16.0;
             char_data.heading = 0.0;
             self.pos = Position { x: -326.0, y: 0.0, z: -16.0, heading: 0.0 };
             self.zone_id = 202;
             self.save_character().await;
        } else {
             self.pos = Position { x: c.x, y: c.y, z: c.z, heading: c.heading };
             self.zone_id = c.zone_id;
        }

        let (w_tx, w_rx) = oneshot::channel();
        let mut assigned_id = 0;
        
         if let Err(e) = self.world_tx.send(WorldCommand::Enter {
            name: char_data.name.clone(),
            pos: self.pos.clone(),
            event_tx: self.tx_world_event.clone(),
            respond_to: w_tx,
        }).await {
            log::error!("WorldManager unavailable: {}", e);
        } else {
            match w_rx.await {
                Ok(id) => {
                    self.entity_id = Some(id);
                    assigned_id = id.data().as_ffi() as u32;
                    log::info!("Registered EntityID {:?} for {}", id, char_data.name);
                },
                Err(_) => log::error!("WorldManager dropped Enter request"),
            }
        }
        
        self.level = char_data.level as i16;
        self.exp = char_data.exp;
            
        log::info!("Sending PlayerProfile for '{}' (Level {} {} {}) EntityID: {}", 
            char_name, char_data.level, char_data.race, char_data.class, assigned_id);
            
        self.send_player_profile(char_data.clone(), assigned_id).await;
        
        // Stage 1: Send PlayerProfile + Environment + ZoneEntry Response
        self.send_zone_entry_response(char_data.clone(), assigned_id).await;
        self.send_time_of_day().await;
        self.send_weather().await;
    }

    async fn send_zone_entry_response(&mut self, c: CharacterDbData, entity_id: u32) {
         log::info!("Sending OP_ZoneEntry (Response) - OpCode::ZoneEntry");
         // The OP_ZoneEntry (Response) in RoF2 is a Spawn_Struct.
         let mut payload = vec![0u8; 2048];
         
         // 0000: Name (64 bytes)
         let name = c.name.as_bytes();
         let len = std::cmp::min(name.len(), 64);
         payload[0..len].copy_from_slice(&name[..len]);
         
         // 0064: SpawnID (u32)
         let spawn_id_bytes = entity_id.to_le_bytes();
         payload[64..68].copy_from_slice(&spawn_id_bytes);

         // 0068: Level (u8)
         payload[68] = c.level as u8;

         // 0073: NPC (u8) - 0 for Player
         payload[73] = 0;
         
         // 0126: StandState (u8) - 100 for Standing
         payload[126] = 100;

         // Position data (Spawn_Struct has a complex bitfield, but simple floats often work in older versions
         // or specific offsets in RoF2)
         // RoF2 Spawn_Struct_Position is bitpacked, but let's try some common offsets first.
         // Wait, according to rof2_structs.h, Position is at offset 479... no, offset 429 is spawn struct.
         // Position is usually around 489.
         
         // Sending as 0x5089 (OP_ZoneEntry)
         self.send_app_packet(OpCode::ZoneEntry, &payload).await;
    }



    async fn handle_client_ready(&mut self) {
        log::info!("Received OP_ClientReady. Starting Stage 2 Initialization...");
        
        // Stage 2: Send Character Data (Inventory, Guilds, AA, Exp)
        
        // 1. Experience (FULL: 152 bytes)
        // OP_SendExpZonein (0x5f8e)
        // struct SendExpZonein_Struct { uint16 spawn_id, uint16 type, uint32 param, uint32 exp, uint32 expAA, ... }
        let mut exp_buf = vec![0u8; 152];
        let spawn_id = self.entity_id.map(|id| id.data().as_ffi() as u32).unwrap_or(0) as u16;
        exp_buf[0..2].copy_from_slice(&spawn_id.to_le_bytes()); // spawn_id
        exp_buf[8..12].copy_from_slice(&(self.exp as u32).to_le_bytes()); // exp
        self.send_app_packet(OpCode::SendExpZonein, &exp_buf).await;
        
        // 2. AA Stats (Stub: Empty)
        // OP_SendAAStats (0x43c8)
        self.send_app_packet(OpCode::SendAAStats, &[]).await;
        
        // 3. Tributes (Stub: Empty)
        // OP_SendTributes (0x729b)
        self.send_app_packet(OpCode::SendTributes, &[]).await;

        // 4. Level Update (0x1eec)
        let mut level_buf = [0u8; 12];
        level_buf[0..4].copy_from_slice(&(self.level as u32).to_le_bytes());
        level_buf[4..8].copy_from_slice(&(self.level as u32).to_le_bytes());
        self.send_app_packet(OpCode::LevelUpdate, &level_buf).await;

        // 5. Stamina (0x2a79) - Food/Water
        let mut stamina_buf = [0u8; 8];
        stamina_buf[0..4].copy_from_slice(&100u32.to_le_bytes());
        stamina_buf[4..8].copy_from_slice(&100u32.to_le_bytes());
        self.send_app_packet(OpCode::Stamina, &stamina_buf).await;

        // 4. Inventory (Stub: Empty)
        // OP_CharInventory (0x5ca6)
        // ItemPacketType::ItemPacketCharInventory = 0x69 (u32)
        self.send_app_packet(OpCode::CharInventory, &0x69u32.to_le_bytes()).await;

        // 5. Guilds (Stub: Empty)
        // OP_GuildsList (0x507a)
        // GuildsList_Struct head[64]
        self.send_app_packet(OpCode::GuildsList, &[0u8; 64]).await;
        
        log::info!("Sent Stage 2 Packets (Exp, AA, Tributes, Inventory, Guilds)");
    }

    async fn handle_req_client_spawn(&mut self) {
        log::info!("Received OP_ReqClientSpawn. Starting Stage 3 Initialization...");
        
        // Stage 3: Finalize Spawn (ZonePoints, Spawns, Enable)
        
        // 1. Zone Points
        self.send_zone_points().await;
        
        // 2. Client Ready Echo
        // OP_ClientReady (0x345d)
        self.send_app_packet(OpCode::ClientReady, &[]).await;
        
        log::info!("Sent Stage 3 Packets (ZonePoints, ClientReady)");
    }

    async fn save_character(&self) {
        if let Some(char_id) = self.char_id {
            log::info!("Saving Character {}", char_id);
            let res = sqlx::query(
                "UPDATE character_data SET x = $1, y = $2, z = $3, heading = $4, \
                 zone_id = $5, level = $6, exp = $7 WHERE id = $8"
            )
            .bind(self.pos.x)
            .bind(self.pos.y)
            .bind(self.pos.z)
            .bind(self.pos.heading)
            .bind(self.zone_id)
            .bind(self.level as i32)
            .bind(self.exp)
            .bind(char_id)
            .execute(&self.db_pool)
            .await;

            if let Err(e) = res { log::error!("Failed to save character {}: {}", char_id, e); }
        }
    }

    async fn send_player_profile(&mut self, c: CharacterDbData, entity_id: u32) {
        let mut name_bytes = [0u8; 64];
        let name = c.name.as_bytes();
        let len = std::cmp::min(name.len(), 64);
        name_bytes[..len].copy_from_slice(&name[..len]);

        let mut last_name_bytes = [0u8; 32];
        let last_name = c.last_name.as_bytes();
        let last_len = std::cmp::min(last_name.len(), 32);
        last_name_bytes[..last_len].copy_from_slice(&last_name[..last_len]);
        // Basic profile construction
        let profile = PlayerProfile {
            name: name_bytes,
            last_name: last_name_bytes,
            level: c.level as u8,
            race: c.race as u16,
            class: c.class as u8,
            gender: c.gender as u8,
            deity: c.deity as u16,
            entity_id,
            zone_id: if c.zone_id > 0 { c.zone_id as u16 } else { 202 },
            zone_instance: c.zone_instance as u16,

            // Appearances
            face: c.face as u8,
            hair_color: c.hair_color as u8,
            hair_style: c.hair_style as u8,
            beard: c.beard as u8,
            beard_color: c.beard_color as u8,
            eye_color_1: c.eye_color_1 as u8,
            eye_color_2: c.eye_color_2 as u8,
            drakkin_heritage: c.drakkin_heritage as u32,
            drakkin_tattoo: c.drakkin_tattoo as u32,
            drakkin_details: c.drakkin_details as u32,
            
            // Stats from DB
            cur_hp: c.cur_hp as u32,
            mana: c.mana as u32,
            endurance: c.endurance as u32,
            str: c.str as u32,
            sta: c.sta as u32,
            dex: c.dex as u32,
            agi: c.agi as u32,
            int: c.int as u32,
            wis: c.wis as u32,
            cha: c.cha as u32,

            // Status
            intoxication: c.intoxication as u32,
            toxicity: c.toxicity as u32,
            hunger_level: c.hunger_level as u32,
            thirst_level: c.thirst_level as u32,

            // Currency from DB
            platinum: c.platinum as u32, 
            gold: c.gold as u32,
            silver: c.silver as u32,
            copper: c.copper as u32,

            // Experience
            exp: c.exp as u32,
            points: c.points as u32,
            
            x: c.x as f32,
            y: c.y as f32,
            z: if c.z == 0.0 { 10.0 } else { c.z as f32 }, // Safe default if 0
            heading: c.heading as f32,
        };

        let mut payload = Vec::new();
        let mut cursor = Cursor::new(&mut payload);
        if let Err(e) = profile.write_options(&mut cursor, binrw::Endian::Little, ()) {
             log::error!("Failed to serialize profile: {}", e);
             return;
        }
        
        self.send_app_packet(OpCode::PlayerProfile, &payload).await; // OP_PlayerProfile
    }

    async fn update_visibility(&mut self) {
        let (tx, rx) = oneshot::channel();
        let entity_id = self.entity_id.unwrap();
        
        if let Err(e) = self.world_tx.send(WorldCommand::GetVisible {
            id: entity_id,
            respond_to: tx,
        }).await {
            log::error!("Visibility query failed: {}", e);
            return;
        }

        match rx.await {
            Ok(visible) => {
                let mut current_ids = std::collections::HashSet::new();
                for (id, entity) in visible {
                    current_ids.insert(id);
                    if let Some(last_pos) = self.visible_entities.get(&id) {
                        // Already visible, check for movement
                        if (entity.pos.x - last_pos.x).abs() > 0.1 || 
                           (entity.pos.y - last_pos.y).abs() > 0.1 || 
                           (entity.pos.z - last_pos.z).abs() > 0.1 {
                            self.send_client_update(id, entity.pos).await;
                            self.visible_entities.insert(id, entity.pos);
                        }
                    } else {
                        // Newly visible
                        log::info!("New Entity Visible: {} (ID={:?})", entity.name, id);
                        self.send_spawn(id, entity.clone()).await;
                        self.visible_entities.insert(id, entity.pos);
                        let spawn_id = id.data().as_ffi() as u32;
                        self.spawn_id_to_entity_id.insert(spawn_id, id);
                    }
                }

                // Cleanup no longer visible
                let mut to_remove = Vec::new();
                for &id in self.visible_entities.keys() {
                    if !current_ids.contains(&id) {
                        to_remove.push(id);
                    }
                }
                for id in to_remove {
                    self.visible_entities.remove(&id);
                    let spawn_id = id.data().as_ffi() as u32;
                    self.spawn_id_to_entity_id.remove(&spawn_id);
                }

                // Handle entities that moved out of range
                let to_remove: Vec<EntityId> = self.visible_entities.keys()
                    .filter(|id| !current_ids.contains(id))
                    .cloned()
                    .collect();

                for id in to_remove {
                    log::info!("Entity Out of Range: (ID={:?})", id);
                    self.send_despawn(id).await;
                    self.visible_entities.remove(&id);
                }
            }
            Err(_) => log::error!("WorldManager dropped GetVisible request"),
        }
    }

    async fn send_spawn(&mut self, entity_id: EntityId, entity: crate::game::world::Entity) {
        let id_u32 = entity_id.data().as_ffi() as u32;
        let spawn = Spawn {
            name: entity.name,
            last_name: String::new(),
            spawn_id: id_u32,
            level: 1,
            race: 1,
            class: 1,
            gender: 0,
            cur_hp: 100,
            max_hp: 100,
            x: entity.pos.x,
            y: entity.pos.y,
            z: entity.pos.z,
            heading: entity.pos.heading,
            npc: 0, // Assume player for now
        };

        let mut payload = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut payload);
        if let Ok(_) = spawn.write_options(&mut cursor, binrw::Endian::Little, ()) {
            self.send_app_packet(OpCode::Spawn, &payload).await;
        }
    }

    async fn send_despawn(&mut self, entity_id: EntityId) {
        let id_u32 = entity_id.data().as_ffi() as u32;
        self.send_app_packet(OpCode::DeleteSpawn, &id_u32.to_le_bytes()).await;
    }

    async fn send_client_update(&mut self, entity_id: EntityId, pos: Position) {
        let id_u16 = (entity_id.data().as_ffi() as u32) as u16;
        let update = ClientUpdate {
            spawn_id: id_u16,
            sequence: 0,
            unknown0004: 0,
            x_pos: pos.x,
            y_pos: pos.y,
            z_pos: pos.z,
            delta_x: 0.0,
            delta_y: 0.0,
            delta_z: 0.0,
            flags1: 0x00010000, // Basic standing/animation flags
            flags2: ((pos.heading / 360.0 * 4096.0) as u32) << 10, // Packed heading
        };

        let mut payload = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut payload);
        if let Ok(_) = update.write_options(&mut cursor, binrw::Endian::Little, ()) {
            self.send_app_packet(OpCode::ClientUpdate, &payload).await;
        }
    }

    async fn send_app_packet(&mut self, opcode: OpCode, data: &[u8]) {
        let packets = self.session.create_raw_packets(opcode, data);
        for pkt in packets {
            self.send_raw(pkt).await;
        }
    }

    async fn send_raw(&mut self, data: Vec<u8>) {
        let _ = self.tx_outbound.send(OutboundPacket::Raw(data)).await;
    }
    async fn send_zone_points(&mut self) {
        log::info!("Sending OP_ZonePoints");
        // Opcode 0x69a4 (RoF2) - Count = 0 (u32)
        // Opcode 0x69a4 (RoF2) - Count = 0 (u32) + 1 Empty Entry (24 bytes) = 28 bytes
        // Akk-Stack: sizeof(ZonePoints) + ((count + 1) * sizeof(ZonePoint_Entry))
        let payload = [0u8; 28];
        self.send_app_packet(OpCode::SendZonePoints, &payload).await;
    }

    async fn send_time_of_day(&mut self) {
        log::info!("Sending OP_TimeOfDay");
        // Opcode 0x5070 (RoF2) - h,m,d,m,y
        let mut data = Vec::new();
        data.push(12u8); // Hour
        data.push(0u8); // Min
        data.push(1u8); // Day
        data.push(1u8); // Month
        data.extend_from_slice(&3000u32.to_le_bytes()); // Year
        self.send_app_packet(OpCode::TimeOfDay, &data).await;
    }

    async fn send_weather(&mut self) {
        log::info!("Sending OP_Weather");
        // Opcode 0x661e (RoF2) - Zone(4), Type(4)
        self.send_app_packet(OpCode::Weather, &[0u8; 8]).await;
    }
    async fn handle_shop_request(&mut self, payload: &[u8]) {
        let mut cursor = Cursor::new(payload);
        if let Ok(click) = MerchantClick::read(&mut cursor) {
            log::info!("Shop Request for NPC ID {}", click.npc_id);
            
            let (tx, rx) = oneshot::channel();
            let ent_id = slotmap::KeyData::from_ffi(click.npc_id as u64).into();
            
            if let Err(e) = self.world_tx.send(WorldCommand::GetMerchantInventory {
                npc_id: ent_id,
                respond_to: tx,
            }).await {
                log::error!("Failed to send GetMerchantInventory command: {}", e);
                return;
            }

            match rx.await {
                Ok(Ok(items)) => {
                    log::info!("Merchant has {} items", items.len());
                    let items_vec: Vec<MerchantItem> = items;
                    for item in items_vec {
                        let list_pkt = MerchantList {
                            npc_id: click.npc_id,
                            slot: item.slot as u32,
                            item_id: item.item_id as u32,
                        };
                        
                        let mut pkt_payload = Vec::new();
                        let mut pkt_cursor = Cursor::new(&mut pkt_payload);
                        if let Ok(_) = list_pkt.write(&mut pkt_cursor) {
                            self.send_app_packet(OpCode::ShopList, &pkt_payload).await;
                        }
                    }
                }
                Ok(Err(e)) => log::error!("DB error loading merchant inventory: {}", e),
                Err(e) => log::error!("oneshot error loading merchant inventory: {}", e),
            }
        }
    }

    async fn handle_shop_buy(&mut self, payload: &[u8]) {
        let mut cursor = Cursor::new(payload);
        if let Ok(buy) = MerchantBuy::read(&mut cursor) {
            log::info!("Shop BUY Request: NPC SpawnID {}, Slot {}, Qty {}, Price {}", buy.npc_id, buy.slot, buy.quantity, buy.price);
            
            if let Some(&npc_entity_id) = self.spawn_id_to_entity_id.get(&buy.npc_id) {
                let (tx, rx) = oneshot::channel();
                if let Err(e) = self.world_tx.send(WorldCommand::BuyItem {
                    char_id: self.char_id.unwrap_or(0),
                    npc_id: npc_entity_id,
                    item_slot: buy.slot as i32,
                    quantity: buy.quantity as i32,
                    respond_to: tx,
                }).await {
                    log::error!("Failed to send BuyItem command: {}", e);
                    return;
                }

                match rx.await {
                    Ok(Ok((item_id, base_price))) => {
                        // For now, use a simple 1.1x markup or similar
                        // In reality, charisma affects this.
                        let total_price = (base_price as f32 * 1.1) as u32 * buy.quantity;
                        let player_total_money = (self.platinum as u32 * 1000) + (self.gold as u32 * 100) + (self.silver as u32 * 10) + self.copper as u32;
                        
                        if player_total_money >= total_price {
                            log::info!("Purchase authorized: ItemID {} for {} copper (Total Copper: {})", item_id, total_price, player_total_money);
                            
                            // 1. Subtract money from DB
                            let new_total = player_total_money - total_price;
                            let new_plat = new_total / 1000;
                            let new_gold = (new_total % 1000) / 100;
                            let new_silver = (new_total % 100) / 10;
                            let new_copper = new_total % 10;

                            if let Some(char_id) = self.char_id {
                                let res = sqlx::query(
                                    "UPDATE character_currency SET platinum = $1, gold = $2, silver = $3, copper = $4 WHERE id = $5"
                                )
                                .bind(new_plat as i32)
                                .bind(new_gold as i32)
                                .bind(new_silver as i32)
                                .bind(new_copper as i32)
                                .bind(char_id)
                                .execute(&self.db_pool)
                                .await;

                                if let Ok(_) = res {
                                    // 2. Add item to inventory
                                    let inv_res = sqlx::query(
                                        "INSERT INTO inventory_items (char_id, item_id, slot_id, quantity) \
                                         VALUES ($1, $2, (SELECT COALESCE(MAX(slot_id), 22) + 1 FROM inventory_items WHERE char_id = $1), $3)"
                                    )
                                    .bind(char_id)
                                    .bind(item_id as i32)
                                    .bind(buy.quantity as i16)
                                    .execute(&self.db_pool)
                                    .await;

                                    if let Ok(_) = inv_res {
                                        // Update local state and notify client
                                        self.platinum = new_plat as i32;
                                        self.gold = new_gold as i32;
                                        self.silver = new_silver as i32;
                                        self.copper = new_copper as i32;
                                        
                                        // Send MoneyUpdate (OP_MoneyUpdate = 0x4859)
                                        let mut money_buf = vec![0u8; 16];
                                        money_buf[0..4].copy_from_slice(&(self.platinum as u32).to_le_bytes());
                                        money_buf[4..8].copy_from_slice(&(self.gold as u32).to_le_bytes());
                                        money_buf[8..12].copy_from_slice(&(self.silver as u32).to_le_bytes());
                                        money_buf[12..16].copy_from_slice(&(self.copper as u32).to_le_bytes());
                                        self.send_app_packet(OpCode::MoneyUpdate, &money_buf).await;
                                        
                                        log::info!("Item {} purchased successfully.", item_id);
                                    }
                                }
                            }
                        } else {
                            log::warn!("Insufficient funds: Have {}, Need {}", player_total_money, total_price);
                        }
                    }
                    Ok(Err(e)) => log::error!("Merchant Buy error: {}", e),
                    Err(e) => log::error!("oneshot error in handle_shop_buy: {}", e),
                }
            }
        }
    }

    async fn handle_shop_sell(&mut self, payload: &[u8]) {
        let mut cursor = Cursor::new(payload);
        if let Ok(sell) = MerchantSell::read(&mut cursor) {
            log::info!("Shop SELL Request: NPC SpawnID {}, ItemSlot {}, Qty {}, Price {}", sell.npc_id, sell.item_slot, sell.quantity, sell.price);
            
            if let Some(char_id) = self.char_id {
                // 1. Find item_id in inventory
                let item_res = sqlx::query!(
                    "SELECT item_id FROM inventory_items WHERE char_id = $1 AND slot_id = $2 LIMIT 1",
                    char_id,
                    sell.item_slot as i16
                )
                .fetch_optional(&self.db_pool)
                .await;

                match item_res {
                    Ok(Some(row)) => {
                        let item_id = row.item_id;
                        
                        if let Some(&npc_entity_id) = self.spawn_id_to_entity_id.get(&sell.npc_id) {
                            let (tx, rx) = oneshot::channel();
                            if let Err(e) = self.world_tx.send(WorldCommand::SellItem {
                                char_id,
                                npc_id: npc_entity_id,
                                item_id,
                                quantity: sell.quantity as i32,
                                respond_to: tx,
                            }).await {
                                log::error!("Failed to send SellItem command: {}", e);
                                return;
                            }

                            match rx.await {
                                Ok(Ok(sell_price_per_item)) => {
                                    let total_profit = sell_price_per_item * sell.quantity;
                                    log::info!("Sale authorized: ItemID {} for {} copper total", item_id, total_profit);

                                    // 2. Remove item (or decrease quantity) from inventory
                                    // For simplicity, we just delete the row if quantity = 1 or all sold
                                    let del_res = sqlx::query(
                                        "DELETE FROM inventory_items WHERE char_id = $1 AND slot_id = $2"
                                    )
                                    .bind(char_id)
                                    .bind(sell.item_slot as i16)
                                    .execute(&self.db_pool)
                                    .await;

                                    if let Ok(_) = del_res {
                                        // 3. Add money
                                        let player_total_money = (self.platinum as u32 * 1000) + (self.gold as u32 * 100) + (self.silver as u32 * 10) + self.copper as u32;
                                        let new_total = player_total_money + total_profit;
                                        
                                        let new_plat = new_total / 1000;
                                        let new_gold = (new_total % 1000) / 100;
                                        let new_silver = (new_total % 100) / 10;
                                        let new_copper = new_total % 10;

                                        let money_res = sqlx::query(
                                            "UPDATE character_currency SET platinum = $1, gold = $2, silver = $3, copper = $4 WHERE id = $5"
                                        )
                                        .bind(new_plat as i32)
                                        .bind(new_gold as i32)
                                        .bind(new_silver as i32)
                                        .bind(new_copper as i32)
                                        .bind(char_id)
                                        .execute(&self.db_pool)
                                        .await;

                                        if let Ok(_) = money_res {
                                            self.platinum = new_plat as i32;
                                            self.gold = new_gold as i32;
                                            self.silver = new_silver as i32;
                                            self.copper = new_copper as i32;

                                            // Send MoneyUpdate
                                            let mut money_buf = vec![0u8; 16];
                                            money_buf[0..4].copy_from_slice(&(self.platinum as u32).to_le_bytes());
                                            money_buf[4..8].copy_from_slice(&(self.gold as u32).to_le_bytes());
                                            money_buf[8..12].copy_from_slice(&(self.silver as u32).to_le_bytes());
                                            money_buf[12..16].copy_from_slice(&(self.copper as u32).to_le_bytes());
                                            self.send_app_packet(OpCode::MoneyUpdate, &money_buf).await;
                                            
                                            log::info!("Item {} sold successfully for {} copper.", item_id, total_profit);
                                        }
                                    }
                                }
                                Ok(Err(e)) => log::error!("Merchant Sell error: {}", e),
                                Err(e) => log::error!("oneshot error in handle_shop_sell: {}", e),
                            }
                        }
                    }
                    Ok(None) => log::warn!("Item at slot {} not found in inventory for sale.", sell.item_slot),
                    Err(e) => log::error!("DB error checking inventory for sale: {}", e),
                }
            }
        }
    }
}
