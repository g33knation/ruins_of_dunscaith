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
