use binrw::{BinRead, BinWrite};

#[derive(Debug, BinRead, BinWrite)]
#[br(little)]
#[bw(little)]
pub struct TargetMouse {
    pub target_id: u32,
}

#[derive(Debug, BinRead, BinWrite)]
#[br(little)]
#[bw(little)]
pub struct CombatDamage {
    pub target_id: u16,
    pub source_id: u16,
    pub damage_type: u8,
    pub spell_id: u32,
    pub damage: i32,
    pub force: f32,
    pub hit_heading: f32,
    pub hit_pitch: f32,
    pub secondary: u8,
    pub special: u32,
}

#[derive(Debug, BinRead, BinWrite)]
#[br(little)]
#[bw(little)]
pub struct ActionPacket {
    pub target_id: u16,
    pub source_id: u16,
    pub level: u16,
    pub unknown06: u32,
    pub instrument_mod: f32,
    pub force: f32,
    pub hit_heading: f32,
    pub hit_pitch: f32,
    pub action_type: u8, // 0xE7 for spells
    pub damage: u32,
    pub unknown31: u16,
    pub spell_id: u32,
    pub spell_level: u8,
    pub effect_flag: u8,
}

#[derive(Debug, BinRead, BinWrite)]
#[br(little)]
#[bw(little)]
pub struct ChannelMessage {
    pub targetname: [i8; 64],
    pub sender: [i8; 64],
    pub language: u32,
    pub channel: u32,
    pub unknown: [u32; 2],
    pub skill: u32,
    pub message: [i8; 256], // Fixed size for now, EqEmu uses variable tail but 256 is safe for basic Say
}

#[derive(Debug, BinRead, BinWrite)]
#[br(little)]
#[bw(little)]
pub struct MerchantClick {
    pub npc_id: u32,
    pub player_id: u32,
    pub command: u32, // 1=open, 0=close
    pub rate: f32,
    pub tab_display: i32,
    pub unknown02: i32,
}

#[derive(Debug, BinRead, BinWrite, Clone)]
#[br(little)]
#[bw(little)]
pub struct MerchantBuy {
    pub npc_id: u32,
    pub slot: i16,
    pub sub_index: i16,
    pub aug_index: i16,
    pub unknown01: i16,
    pub quantity: u32,
    pub price: u32,
}

#[derive(Debug, BinRead, BinWrite, Clone)]
#[br(little)]
#[bw(little)]
pub struct MerchantSell {
    pub npc_id: u32,
    pub player_id: u32,
    pub item_slot: u32,
    pub unknown12: u32,
    pub quantity: u32,
    pub unknown20: u32,
    pub price: u32,
    pub unknown28: u32,
}

#[derive(Debug, BinRead, BinWrite, Clone)]
#[br(little)]
#[bw(little)]
pub struct MerchantList {
    pub npc_id: u32,
    pub slot: u32,
    pub item_id: u32,
}

// --- Spell Packets (RoF2) ---

#[derive(Debug, BinRead, BinWrite, Clone)]
#[br(little)]
#[bw(little)]
pub struct InventorySlot {
    pub slot_type: i16,
    pub unknown02: i16,
    pub slot: i16,
    pub sub_index: i16,
    pub aug_index: i16,
    pub unknown01: i16,
}

#[derive(Debug, BinRead, BinWrite, Clone)]
#[br(little)]
#[bw(little)]
pub struct BeginCast {
    pub spell_id: u32,
    pub caster_id: u16,
    pub cast_time: u32,
}

#[derive(Debug, BinRead, BinWrite, Clone)]
#[br(little)]
#[bw(little)]
pub struct CastSpell {
    pub gem_slot: u32,
    pub spell_id: u32,
    pub inventory_slot: InventorySlot,
    pub target_id: u32,
    pub unknown: [u32; 2],
    pub y_pos: f32,
    pub x_pos: f32,
    pub z_pos: f32,
}

#[derive(Debug, BinRead, BinWrite, Clone)]
#[br(little)]
#[bw(little)]
pub struct InterruptCast {
    pub spawn_id: u32,
    pub message_id: u32,
}

#[derive(Debug, BinRead, BinWrite, Clone)]
#[br(little)]
#[bw(little)]
pub struct ZoneEntryResponse {
    pub name: [u8; 64],
    pub entity_id: u32,
    pub level: u8,
    pub unknown69: [u8; 4],
    pub npc: u8,
    pub unknown74: [u8; 52],
    pub stand_state: u8,
    pub unknown127: [u8; 1921], // Total 2048 array
}

impl Default for ZoneEntryResponse {
    fn default() -> Self {
        Self {
            name: [0; 64],
            entity_id: 0,
            level: 0,
            unknown69: [0; 4],
            npc: 0,
            unknown74: [0; 52],
            stand_state: 100,
            unknown127: [0; 1921],
        }
    }
}
