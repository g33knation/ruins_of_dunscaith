use bytes::{BufMut};
use shared::game::char_profile::CharProfilePacket;
use shared::db::Character;
use std::io::Cursor;
use binrw::{BinWrite, BinRead};

// Constants removed in favor of shared::opcodes::OpCode


#[derive(BinWrite, Debug)]
#[bw(little)]
pub struct ZoneServerInfo {
    #[bw(map = |s: &String| {
        let mut b = [0u8; 128];
        let bytes = s.as_bytes();
        let len = bytes.len().min(127);
        b[..len].copy_from_slice(&bytes[..len]);
        b
    })]
    pub ip: String,
    pub port: u16,
}

pub fn build_zone_server_info(ip: &str, port: u16) -> Vec<u8> {
    let info = ZoneServerInfo { ip: ip.to_string(), port };
    let mut data = Cursor::new(Vec::new());
    info.write(&mut data).unwrap();
    data.into_inner()
}


// Client Echoes / CRCs removed in favor of OpCode enum

#[derive(BinRead, Debug)]
#[br(little)]
pub struct NameApprovalStruct {
    #[br(count = 64, map = |bytes: Vec<u8>| String::from_utf8_lossy(&bytes).trim_matches(char::from(0)).to_string())]
    pub name: String,
    pub race_id: u32,
    pub class_id: u32,
    pub deity: u32,
    pub unknown1: u32,
}

// Character Creation and Entry structures are handled via manual parsing or specific RoF2 structs below

#[derive(BinWrite, Debug)]
#[bw(little)]
pub struct MaxCharacters {
    pub count: u32,
    pub unknown1: u32,
    pub unknown2: u32,
}

pub fn build_send_max_characters() -> Vec<u8> {
    let info = MaxCharacters {
        count: 12,
        unknown1: 0,
        unknown2: 0,
    };
    let mut data = Cursor::new(Vec::new());
    info.write(&mut data).unwrap();
    data.into_inner()
}

#[derive(BinWrite, Debug)]
#[bw(little)]
pub struct ApproveWorld {
    pub data: [u8; 544],
}

impl Default for ApproveWorld {
    fn default() -> Self {
        let mut d = [0u8; 544];
        let magic: [u8; 90] = [
            0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x37,0x87,0x13,0xbe,0xc8,0xa7,0x77,0xcb,
            0x27,0xed,0xe1,0xe6,0x5d,0x1c,0xaa,0xd3,0x3c,0x26,0x3b,0x6d,0x8c,0xdb,0x36,0x8d,
            0x91,0x72,0xf5,0xbb,0xe0,0x5c,0x50,0x6f,0x09,0x6d,0xc9,0x1e,0xe7,0x2e,0xf4,0x38,
            0x1b,0x5e,0xa8,0xc2,0xfe,0xb4,0x18,0x4a,0xf7,0x72,0x85,0x13,0xf5,0x63,0x6c,0x16,
            0x69,0xf4,0xe0,0x17,0xff,0x87,0x11,0xf3,0x2b,0xb7,0x73,0x04,0x37,0xca,0xd5,0x77,
            0xf8,0x03,0x20,0x0a,0x56,0x8b,0xfb,0x35,0xff,0x59
        ];
        d[0..90].copy_from_slice(&magic);
        d[192 + 12] = 0x15;
        d[208] = 0x53;
        d[209] = 0xC3;
        d[528 + 12] = 0x01;
        Self { data: d }
    }
}

pub fn build_approve_world() -> Vec<u8> {
    let info = ApproveWorld::default();
    let mut data = Cursor::new(Vec::new());
    info.write(&mut data).unwrap();
    data.into_inner()
}


#[derive(BinWrite, Debug)]
#[bw(little)]
pub struct LogServer {
    #[bw(map = |s: &String| {
        let mut b = [0u8; 36];
        let bytes = s.as_bytes();
        let len = bytes.len().min(35);
        b[..len].copy_from_slice(&bytes[..len]);
        b
    })]
    pub master_branch: String,
    
    #[bw(map = |s: &String| {
        let mut b = [0u8; 28];
        let bytes = s.as_bytes();
        let len = bytes.len().min(27);
        b[..len].copy_from_slice(&bytes[..len]);
        b
    })]
    pub titanium: String,
}

pub fn build_log_server() -> Vec<u8> {
    let name = "Server".to_string();
    let info = LogServer {
        master_branch: name.clone(),
        titanium: name,
    };
    let mut data = Cursor::new(Vec::new());
    info.write(&mut data).unwrap();
    data.into_inner()
}



#[derive(BinWrite, Debug)]
#[bw(little)]
pub struct Motd {
    pub text: binrw::NullString,
}

pub fn build_motd() -> Vec<u8> {
    let msg = "Welcome to RuinsofDunscaith".to_string();
    let info = Motd { text: msg.into() };
    let mut data = Cursor::new(Vec::new());
    info.write(&mut data).unwrap();
    data.into_inner()
}

#[derive(BinWrite, Debug)]
#[bw(little)]
pub struct Weather {
    pub data: [u8; 12],
}

pub fn build_weather() -> Vec<u8> {
    let info = Weather { data: [0u8; 12] };
    let mut data = Cursor::new(Vec::new());
    info.write(&mut data).unwrap();
    data.into_inner()
}

#[derive(BinWrite, Debug)]
#[bw(little)]
pub struct TributeInfo {
    pub data: [u8; 48],
}

pub fn build_tribute_info() -> Vec<u8> {
    let info = TributeInfo { data: [0u8; 48] };
    let mut data = Cursor::new(Vec::new());
    info.write(&mut data).unwrap();
    data.into_inner()
}



#[derive(BinWrite, Debug)]
#[bw(little)]
pub struct GuildsList {
    pub head: [u8; 64],
    pub count: u32,
}

pub fn build_guilds_list() -> Vec<u8> {
    let info = GuildsList {
        head: [0u8; 64],
        count: 0,
    };
    let mut data = Cursor::new(Vec::new());
    info.write(&mut data).unwrap();
    data.into_inner()
}


pub fn build_character_create_request_response() -> Vec<u8> {
    // RoF2 Allocations/Combos structure
    let mut data = Vec::with_capacity(2048);
    data.put_u8(0);
    
    // Allocations
    // We provide one generic allocation profile for now to unlock UI.
    // Index 1
    data.put_u32_le(1); // Count = 1
    
    // Alloc 1
    data.put_u32_le(1); // Index
    // BaseStats[7] (STR, STA, DEX, AGI, INT, WIS, CHA)
    for _ in 0..7 { data.put_u32_le(75); }
    // DefaultPoints[7]
    for _ in 0..7 { data.put_u32_le(5); } // 5 extra points?
    
    // Combos: Race, Class, Deity, Zone
    // Using a broad list to attempt to unlock most common options.
    // Zone 202 (PoK) is a safe bet for emus.
    // Deity 211 (Agnostic) or 201 (Bertox) etc. Using 211 where applicable.
    // Combos: Race, Class, Deity, Zone
    // Using explicit RoF2 valid combinations to fix "Locked Races" and "Bad Restrictions".
    // Race IDs: 1-12 Classic, 128 Iksar, 130 VahShir, 330 Froglok, 522 Drakkin.
    // Class IDs: 1 War, 2 Clr, 3 Pal, 4 Rng, 5 SK, 6 Dru, 7 Mnk, 8 Brd, 9 Rog, 10 Shm, 11 Nec, 12 Wiz, 13 Mag, 14 Enc, 15 Bst, 16 Ber.
    // Zone: 202 (PoK)
    
    let races_to_classes = vec![
        (1, vec![1,2,3,4,5,6,7,8,9,11,12,13,14]), // Human
        (2, vec![1,9,10,15,16]), // Barbarian
        (3, vec![2,3,5,11,12,13,14]), // Erudite
        (4, vec![1,4,6,8,9,15]), // Wood Elf
        (5, vec![2,3,11,12,13]), // High Elf
        (6, vec![1,2,5,9,11,12,13,14]), // Dark Elf
        (7, vec![1,3,4,6,8,9]), // Half Elf
        (8, vec![1,2,3,9,16]), // Dwarf
        (9, vec![1,5,10,15,16]), // Troll
        (10, vec![1,5,10,15,16]), // Ogre
        (11, vec![1,2,4,6,9]), // Halfling
        (12, vec![1,2,3,9,11,12,13]), // Gnome
        (128, vec![1,5,7,10,11,15]), // Iksar
        (130, vec![1,8,9,10,15,16]), // Vah Shir
        (330, vec![1,2,3,5,9,10,11,12]), // Froglok
        (522, vec![1,2,3,5,6,7,8,9,11,12,13,14]), // Drakkin
    ];

    let mut combos = Vec::new();
    for (race, classes) in races_to_classes {
        for class in classes {
            // Pick a safe Deity:
            // Agnostic (211) works for War, Rog, Brd, Mnk, Ber, Wiz, Mag, Enc, Nec(sometimes), Rng(sometimes).
            // Priests (Clr, Dru, Shm) and Knights (Pal, SK) usually need a god.
            // Simplified logic: Send 211 (Agnostic) AND a specific valid god for religious classes to ensure appearance.
            // Actually, sending multiple entries for the same race/class (diff deity) is allowed.
            // To result in minimal working set:
            // - If Agnostic is valid, send it.
            // - If Religious, send one primary god.
            
            let deity = match (race, class) {
                 // Priests/Knights need gods
                 (_, 2) | (_, 3) | (_, 5) | (_, 6) | (_, 10) => {
                     match race {
                         6 | 9 | 10 | 128 => 203, // Evil (Cazic/Innoruuk=206). Using Cazic (203) generic evil?
                         4 | 5 | 7 => 215, // Tunare (Nature/Good)
                         2 | 8 => 214, // Tribunal/Brell(202)? Tribunal (214) is universal justice.
                         _ => 208, // Mithaniel Marr (Good Human/Frog/Drakkin)
                     }
                 },
                 // Necromancer sometimes restricted
                 (_, 11) => 203, // Cazic Thule (Safe bet for Necros)
                 // Others Agnostic
                 _ => 211
            };
            
            combos.push((race, class, deity));
            
            // Should we also push Agnostic for everything? 
            // Some checks fail if Agnostic provided for Clr. 
            // So relying on the specific one above is safer.
        }
    }
    
    data.put_u32_le(combos.len() as u32);
    
    for (race, class, deity) in combos {
        data.put_u32_le(0); // ExpansionRequired
        data.put_u32_le(race);
        data.put_u32_le(class);
        data.put_u32_le(deity); // Deity
        data.put_u32_le(1); // Alloc Index 1
        data.put_u32_le(202); // Start Zone (PoK)
    }
    
    data
}

// Character List Entry (Using RoF2CharacterSelectEntry below)

// RoF2 CharacterSelect_Struct (602 bytes)
// RoF2 CharacterSelect_Struct (602 bytes)
#[derive(BinWrite, Clone, Copy, Debug)]
#[bw(little)]
pub struct RoF2CharacterSelectEntry {
    pub name: [u8; 64],
    pub class: u8,
    pub race: u32,
    pub level: u8,
    pub shroud_class: u8,
    pub shroud_race: u32,
    pub zone: u16,
    pub instance: u16,
    pub gender: u8,
    pub face: u8,
    pub equip: [RoF2CharSelectEquip; 9],
    pub unknown15: u8, // Often 0xFF
    pub unknown19: u8, // Often 0xFF
    pub drakkin_tattoo: u32,
    pub drakkin_details: u32,
    pub deity: u32,
    pub primary_file: u32,
    pub secondary_file: u32,
    pub hair_color: u8,
    pub beard_color: u8,
    pub eye_color_1: u8,
    pub eye_color_2: u8,
    pub hair_style: u8,
    pub beard: u8,
    pub go_home: u8,
    pub tutorial: u8,
    pub drakkin_heritage: u32,
    pub unknown_enable: u8, // "Enabled" flag
    pub pad1: u8,
    pub last_login: u32,
    pub unknown2: u8,
    pub pad_final: [u8; 261], // Padding to reach 602 bytes (Akk-Stack/EQEmu standard)
}

impl Default for RoF2CharacterSelectEntry {
    fn default() -> Self {
        Self {
            name: [0u8; 64],
            class: 0,
            race: 0,
            level: 0,
            shroud_class: 0,
            shroud_race: 0,
            zone: 0,
            instance: 0,
            gender: 0,
            face: 0,
            equip: [RoF2CharSelectEquip::default(); 9],
            unknown15: 0,
            unknown19: 0,
            drakkin_tattoo: 0,
            drakkin_details: 0,
            deity: 0,
            primary_file: 0,
            secondary_file: 0,
            hair_color: 0,
            beard_color: 0,
            eye_color_1: 0,
            eye_color_2: 0,
            hair_style: 0,
            beard: 0,
            go_home: 0,
            tutorial: 0,
            drakkin_heritage: 0,
            unknown_enable: 0,
            pad1: 0,
            last_login: 0,
            unknown2: 0,
            pad_final: [0u8; 261],
        }
    }
}

#[derive(BinWrite, Default, Clone, Copy, Debug)]
#[bw(little)]
pub struct RoF2CharSelectEquip {
    pub material: u32,
    pub unknown1: u32,
    pub elite_model: u32,
    pub heros_forge_model: u32,
    pub unknown2: u32,
    pub color: u32, // TintStruct
}

pub fn build_char_info(chars: Vec<shared::db::Character>) -> Vec<u8> {
    if chars.is_empty() {
        return vec![0, 0, 0, 0]; // count = 0
    }
    
    let mut data = Vec::new();
    let mut writer = Cursor::new(&mut data);
    
    // 1. Write Count (u32)
    let count = chars.len() as u32;
    count.write_le(&mut writer).unwrap();
    
    // 2. Write Entries
    for ch in chars {
        let mut entry = RoF2CharacterSelectEntry::default();
        
        let name_bytes = ch.name.as_bytes();
        let len = name_bytes.len().min(63);
        entry.name[..len].copy_from_slice(&name_bytes[..len]);
        
        entry.class = ch.class as u8;
        entry.race = ch.race as u32;
        entry.level = ch.level as u8;
        entry.zone = 202; // PoK for safety
        entry.gender = ch.gender as u8;
        entry.face = ch.face as u8;
        entry.deity = 212; // Agnostic
        entry.hair_color = ch.hair_color as u8;
        entry.beard_color = ch.beard_color as u8;
        entry.eye_color_1 = ch.eye_color_1 as u8;
        entry.eye_color_2 = ch.eye_color_2 as u8;
        entry.hair_style = ch.hair_style as u8;
        entry.beard = ch.beard as u8;
        entry.go_home = 1;
        entry.unknown_enable = 1;
        entry.last_login = 1000;
        entry.unknown15 = 0xFF;
        entry.unknown19 = 0xFF;

        entry.write_le(&mut writer).unwrap();
    }
    
    data
}



// EnterWorld is handled via manual string parsing in session.rs

pub fn build_expansion_info() -> Vec<u8> {
    // RoF2 ExpansionInfo (0x590d)
    // Structure:
    // WindowMask (u32)
    // ContentMask (u32)
    // Expansion 0..23 (u32)
    let mut data = Vec::with_capacity(100);
    
    // WindowMask: Enable all UI
    data.put_u32_le(0xFFFFFFFF); 
    
    // ContentMask: Enable all Content features
    data.put_u32_le(0xFFFFFFFF); 
    
    // Expansions: 24 u32 slots seems to be the standard array size for RoF2
    // Some sources say 22, some 24. We'll send 24 to be safe (96 bytes + 8 = 104? No wait.)
    // Actually, capturing RoF2 logs usually shows a fixed size.
    // Let's assume standard EQ emu approach: 
    // u32 WindowMask
    // u32 ContentMask
    // u32 Expansions[24]
    
    for _ in 0..24 {
        data.put_u32_le(0xFFFFFFFF); // Enable everything
    }
    
    data
}


pub fn build_membership_details() -> Vec<u8> {
    // Membership_Details_Struct (RoF2) - Approx 1124 bytes
    // Copied logic from client.cpp SendMembershipSettings
    let mut data = Vec::with_capacity(1124);
    
    // membership_setting_count = 66 for RoF2 (client.cpp line 330)
    data.put_u32_le(66); 
    
    // Settings Logic: 22 IDs * 3 indices = 66 entries. 
    // Wait, struct has settings[72]. Client populates 66.
    
    let gold_settings: [i32; 22] = [
        -1,-1,-1,-1,-1,-1,1,1,1,-1,1,-1,-1,1,1,1,1,1,1,-1,-1,0
    ];
    
    // Populate 66 settings (3 for each ID)
    for setting_id in 0..22 {
        for setting_index in 0..3 {
            data.put_u32_le(setting_index as u32);
            data.put_u32_le(setting_id as u32);
            data.put_i32_le(gold_settings[setting_id as usize]); // setting_value
        }
    }

    // 2. Race/Class Permissions (15 each)
    // race_entry_count
    data.put_u32_le(15);
    // class_entry_count
    data.put_u32_le(15);

    // Arrays of MembershipEntry_Struct { purchase_id, bitwise_entry }
    // We must generate them first because the packet layout is:
    // [Settings] [RaceEntries] [ClassEntries]
    
    let mut race_entries: Vec<(u32, u32)> = Vec::with_capacity(15);
    let mut class_entries: Vec<(u32, u32)> = Vec::with_capacity(15);

    let mut cur_purchase_id = 90287;
    let mut cur_purchase_id2 = 90301;
    let mut cur_bitwise_value: u32 = 1;

    for entry_id in 0..15 {
        let mut r_purchase_id;
        let mut r_bitwise;
        let mut c_purchase_id;
        let mut c_bitwise;

        if entry_id == 0 {
            r_purchase_id = 1;
            r_bitwise = 0x1FFFF;
            c_purchase_id = 1;
            c_bitwise = 0x1FFFF;
        } else {
            r_purchase_id = cur_purchase_id;
            
            if entry_id < 3 {
                c_purchase_id = cur_purchase_id;
            } else {
                c_purchase_id = cur_purchase_id2;
                cur_purchase_id2 += 1;
            }
            
            // Logic for bitwise overrides:
            if entry_id == 1 {
                r_bitwise = 4110;
                c_bitwise = 4614;
            } else if entry_id == 2 {
                r_bitwise = 4110;
                c_bitwise = 4614;
            } else {
                if entry_id == 12 {
                    cur_bitwise_value *= 2; // Skip 4096
                }
                r_bitwise = cur_bitwise_value;
                c_bitwise = cur_bitwise_value;
            }
            
            cur_purchase_id += 1;
        }
        
        cur_bitwise_value *= 2;
        
        race_entries.push((r_purchase_id, r_bitwise));
        class_entries.push((c_purchase_id, c_bitwise));
    }

    // Write Races
    for (pid, bit) in &race_entries {
        data.put_u32_le(*pid);
        data.put_u32_le(*bit);
    }

    // Write Classes
    for (pid, bit) in &class_entries {
        data.put_u32_le(*pid);
        data.put_u32_le(*bit);
    }
    
    // Urls
    data.put_u32_le(0); // url1 len
    data.put_u32_le(0); // url2 len

    data
}

// PostEnterWorld is handled as size 0

pub fn build_membership() -> Vec<u8> {
    // Membership_Struct (RoF2) - Size 116 bytes
    // Based on rof2.cpp ENCODE(OP_SendMembership) lines 193-217
    // RoF2 expects: entrysize=25, 25 entries (last 4 set to 1), NO exit_url_length
    // membership(4) + races(4) + classes(4) + entrysize(4) + entries[25](100) = 116 bytes
    let mut data = Vec::with_capacity(116);
    data.put_u32_le(2); // membership (Gold = 2)
    data.put_u32_le(0xFFFFFFFF); // races (all races)
    data.put_u32_le(0xFFFFFFFF); // classes (all classes)
    data.put_u32_le(25); // entrysize = 25 for RoF2
    
    // int32 entries[25] - Gold values from EQEmu client.cpp lines 298-318
    // First 21 entries from server
    let entries: [i32; 21] = [
        -1, // 0: Max AA
        -1, // 1: Max Level
        -1, // 2: Max Char Slots
        -1, // 3: Spell Ranks
        -1, // 4: Inventory Size
        -1, // 5: Max Plat
        1,  // 6: Mail
        1,  // 7: Parcels
        1,  // 8: Voice Chat
        -1, // 9: Merc Tiers
        1,  // 10: Create Guild
        -1, // 11: Shared Bank
        -1, // 12: Max Journal
        1,  // 13: House
        1,  // 14: Prestige
        1,  // 15: Broker
        1,  // 16: Chat
        1,  // 17: Progression
        1,  // 18: Support
        -1, // 19: Unknown
        -1, // 20: Maximum
    ];

    for &val in entries.iter() {
        data.put_i32_le(val);
    }
    
    // RoF2 encoding adds 4 more entries (indices 21-24) set to 1
    // This removes "Buy Now" button from aug slots per rof2.cpp comment
    for _ in 0..4 {
        data.put_i32_le(1);
    }
    
    // NO exit_url_length for RoF2 (that's server-side struct only)
    
    data
}



pub fn build_time_of_day() -> Vec<u8> {
    // TimeOfDay_Struct (8 bytes)
    let mut data = Vec::with_capacity(8);
    data.put_u8(12); // Hour
    data.put_u8(0);  // Minute
    data.put_u8(1);  // Day
    data.put_u8(1);  // Month
    data.put_u16_le(3000); // Year (u16)
    data.put_u16_le(0);    // Padding
    data
}

pub fn build_send_zone_points() -> Vec<u8> {
    // ZonePointsStruct (RoF2) - 0x1818
    // Minimal impl: Just count=0 is often enough if we aren't transferring zones
    // Struct:
    // u32 count
    // RoF2 ZonePoint_Struct entries[count]
    // Structure (Based on EQEmu zone.h / common structs):
    // float y, x, z, heading (4x4 = 16)
    // u16 zone (2)
    // u16 instance (2)
    // ... padding/unknowns ...
    // Total Size: ~72-80 bytes? 
    // Let's try sending a minimal valid struct based on best guess:
    // ID (u32), y, x, z, heading, zone(u16), inst(u16), target...
    
    // Actually, let's look at `OP_SendZonePoints` (0x3234).
    // It's usually `count` + `ZonePoint_Struct` array.
    
    let mut data = Vec::with_capacity(4);
    
    // Count = 0 (Safe minimal packet)
    data.put_u32_le(0);
    
    println!("[PACKETS] Generated ZonePoints packet: {} bytes (Empty)", data.len());

    data
}

pub fn build_mercenary_data() -> Vec<u8> {
    // OpCode 0x3e98 (OP_MERCENARY_DATA)
    // Maps to MercenaryDataUpdate_Struct
    // If we send 4 bytes of 0, `MercStatus`=0 (Active), so client expects Count + Data -> Crash/Hang.
    // Correct "No Merc" packet:
    // MercStatus: i32 (-1)
    // MercCount: u32 (0)
    let mut data = Vec::with_capacity(8);
    data.put_i32_le(-1);
    data.put_u32_le(0);
    data
}



