use bytes::{BufMut};
use shared::game::char_profile::CharProfilePacket;
use shared::db::Character;
use std::io::{Cursor, Write};
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
        // Correct RoF2 Offsets from C++ client.cpp source
        d[268] = 0x15;
        d[280] = 0x53;
        d[281] = 0xC3;
        d[540] = 0x01;
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
    // LogServer_Struct (RoF2) - 560 bytes
    let mut data = Vec::with_capacity(560);
    
    // Fill with zeros
    for _ in 0..560 {
        data.put_u8(0);
    }
    
    // Offsets based on rof2_structs.h and rof2.cpp ENCODE
    // /*004*/ uint8 enable_pvp;
    data[4] = 0; // 0 = non-pvp
    // /*008*/ uint8 enable_FV;
    data[8] = 0; // 0 = standard
    
    // rof2.cpp: eq->unknown012 = htonl(1); (Offset 12 = 00 00 00 01)
    data[12..16].copy_from_slice(&1u32.to_be_bytes());
    
    // rof2.cpp: eq->unknown016 = htonl(1); (Offset 16 = 00 00 00 01)
    data[16..20].copy_from_slice(&1u32.to_be_bytes());
    
    // rof2.cpp: eq->unknown020[0] = 1;
    data[20] = 1;
    
    // /*036*/ char worldshortname[32];
    let name = "Server".as_bytes();
    let len = name.len().min(31);
    data[36..36 + len].copy_from_slice(&name[..len]);
    
    // rof2.cpp unknown249 flags
    // eq->unknown249[0] = 1; eq->unknown249[1] = 1; eq->unknown249[8] = 1; ...
    data[249] = 1;
    data[250] = 1;
    data[249 + 8] = 1; // 257
    data[249 + 9] = 1; // 258
    data[249 + 12] = 1; // 261
    data[249 + 14] = 1; // 263
    data[249 + 15] = 1; // 264
    data[249 + 16] = 1; // 265 - was missing!

    // rof2.cpp: eq->unknown276[0] = 1.0f; eq->unknown276[1] = 1.0f; eq->unknown276[6] = 1.0f;
    let f1 = 1.0f32.to_le_bytes();
    data[276..280].copy_from_slice(&f1); // unknown276[0]
    data[280..284].copy_from_slice(&f1); // unknown276[1]
    // unknown276[6] = offset 276 + 6*4 = 300
    data[300..304].copy_from_slice(&f1); // unknown276[6]

    data
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
    // TributeInfo_Struct (RoF2) - Exactly 3144 bytes
    let mut data = Vec::with_capacity(3144);
    for _ in 0..3144 {
        data.push(0);
    }
    data
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
    pub pad_final: [u8; 264], // Padding to reach exactly 602 bytes (Akk-Stack/EQEmu standard)
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
            unknown15: 0xFF,
            unknown19: 0xFF,
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
            unknown_enable: 1,
            pad1: 0,
            last_login: 0,
            unknown2: 0,
            pad_final: [0u8; 264],
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

pub fn build_char_info(mut chars: Vec<shared::db::Character>) -> Vec<u8> {
    // If empty, force 1 character to test if the client crashes on 0 characters
    if chars.is_empty() {
        chars.push(shared::db::Character {
            id: 1,
            account_id: 1,
            name: "Antigravity".to_string(),
            last_name: None,
            zone_id: 202, // PoK
            zone_instance: 0,
            y: 0.0,
            x: 0.0,
            z: 0.0,
            heading: 0.0,
            gender: 0,
            race: 1,
            class: 1,
            level: 50,
            exp: 0,
            practice_points: 0,
            mana: 100,
            cur_hp: 100,
            endurance: 100,
            str: 100,
            sta: 100,
            cha: 100,
            dex: 100,
            int: 100,
            agi: 100,
            wis: 100,
            face: 1,
            hair_style: 1,
            hair_color: 1,
            beard: 0,
            beard_color: 1,
            eye_color_1: 1,
            eye_color_2: 1,
            drakkin_heritage: 0,
            drakkin_tattoo: 0,
            drakkin_details: 0,
            deity: 211,
        });
    }

    let mut data = Vec::with_capacity(1024);
    let mut writer = Cursor::new(&mut data);
    
    // 1. Write Header (RoF2 CharacterSelect_Struct is just CharCount)
    let count = chars.len() as u32;
    count.write_le(&mut writer).unwrap(); // CharCount (4 bytes)
    
    // 2. Write Entries (Variable length: Null-terminated Name + 274 bytes of data)
    for ch in chars {
        // Name: Null-terminated string
        let name_bytes = ch.name.as_bytes();
        writer.write_all(name_bytes).unwrap();
        writer.write_all(&[0u8]).unwrap(); // Null terminator
        
        // Exact struct fields immediately following the name (No alignment padding!)
        (ch.class as u8).write_le(&mut writer).unwrap(); // Class
        (ch.race as u32).write_le(&mut writer).unwrap(); // Race
        (ch.level as u8).write_le(&mut writer).unwrap(); // Level
        (0u8).write_le(&mut writer).unwrap(); // ShroudClass
        (0u32).write_le(&mut writer).unwrap(); // ShroudRace
        (202u16).write_le(&mut writer).unwrap(); // Zone (PoK)
        (0u16).write_le(&mut writer).unwrap(); // Instance
        (ch.gender as u8).write_le(&mut writer).unwrap(); // Gender
        (ch.face as u8).write_le(&mut writer).unwrap(); // Face
        
        // Equip[9] (9 * 24 bytes = 216 bytes)
        for _ in 0..9 {
            RoF2CharSelectEquip::default().write_le(&mut writer).unwrap();
        }
        
        // Remaining fields (41 bytes)
        (0xFFu8).write_le(&mut writer).unwrap(); // Unknown15
        (0xFFu8).write_le(&mut writer).unwrap(); // Unknown19
        (0u32).write_le(&mut writer).unwrap();  // DrakkinTattoo
        (0u32).write_le(&mut writer).unwrap();  // DrakkinDetails
        (212u32).write_le(&mut writer).unwrap(); // Deity (Agnostic)
        (0u32).write_le(&mut writer).unwrap();  // PrimaryIDFile
        (0u32).write_le(&mut writer).unwrap();  // SecondaryIDFile
        (ch.hair_color as u8).write_le(&mut writer).unwrap(); // HairColor
        (ch.beard_color as u8).write_le(&mut writer).unwrap(); // BeardColor
        (ch.eye_color_1 as u8).write_le(&mut writer).unwrap(); // EyeColor1
        (ch.eye_color_2 as u8).write_le(&mut writer).unwrap(); // EyeColor2
        (ch.hair_style as u8).write_le(&mut writer).unwrap(); // HairStyle
        (ch.beard as u8).write_le(&mut writer).unwrap(); // Beard
        (1u8).write_le(&mut writer).unwrap(); // GoHome
        (1u8).write_le(&mut writer).unwrap(); // Tutorial
        (0u32).write_le(&mut writer).unwrap(); // DrakkinHeritage
        (0u8).write_le(&mut writer).unwrap(); // Unknown1
        (1u8).write_le(&mut writer).unwrap(); // Enabled
        (1000u32).write_le(&mut writer).unwrap(); // LastLogin
        (0u8).write_le(&mut writer).unwrap(); // Unknown2
    }
    
    data
}



// EnterWorld is handled via manual string parsing in session.rs

pub fn build_expansion_info() -> Vec<u8> {
    // EQEmu ExpansionInfo_Struct: 68 bytes
    // 64 bytes of padding, then 4-byte bitmask at offset 64.
    let mut data = Vec::with_capacity(68);
    for _ in 0..64 {
        data.put_u8(0);
    }
    data.put_u32_le(0x01FFFFFF); // All expansions through RoF2+ as required by modern clients
    data
}


pub fn build_membership_details() -> Vec<u8> {
    // Membership_Details_Struct (RoF2) - Exactly 1124 bytes
    // 4 + (72 * 12) + 4 + (15 * 8) + 4 + (15 * 8) + 4 + 4 = 1124 bytes
    let mut data = Vec::with_capacity(1124);
    
    // membership_setting_count = 72 for RoF2
    data.put_u32_le(72); 
    
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
    
    // Last 6 new settings fields are all 0s on Live as of 12/29/14 (Matches EQEmu rof2.cpp logic)
    for _ in 66..72 {
        data.put_u32_le(0); // setting_index
        data.put_u32_le(0); // setting_id
        data.put_i32_le(0); // setting_value
    }

    // 2. Race/Class Permissions (15 each)

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
    // race_entry_count (Offset 868)
    data.put_u32_le(15);

    // Write Races (Starts at Offset 872)
    for (pid, bit) in &race_entries {
        data.put_u32_le(*pid);
        data.put_u32_le(*bit);
    }

    // class_entry_count (Offset 992)
    data.put_u32_le(15);

    // Write Classes (Starts at Offset 996)
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
    // Membership_Struct (EQEmu) - matches client.cpp SendMembership()
    // membership(4) + races(4) + classes(4) + entrysize(4) + entries[21](84) + exit_url_length(4) = 104 bytes
    let mut data = Vec::with_capacity(104);
    data.put_u32_le(2);       // membership (Gold = 2)
    data.put_u32_le(0x1ffff); // races (all races)
    data.put_u32_le(0x1ffff); // classes (all classes)
    data.put_u32_le(21);      // entrysize = 21 (EQEmu standard)
    
    // int32 entries[21] - Gold values from EQEmu client.cpp
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
        0,  // 20: Maximum
    ];

    for &val in entries.iter() {
        data.put_i32_le(val);
    }
    
    // exit_url_length = 0 (no URL)
    data.put_u32_le(0);
    
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
    // 8 bytes: count (u32), padding (u32)
    let mut data = Vec::with_capacity(8);
    data.put_u32_le(0);
    data.put_u32_le(0);
    data
}

pub fn build_mercenary_data() -> Vec<u8> {
    // OpCode 0x3e98 (OP_MERCENARY_DATA) -> NoMercenaryHired_Struct
    // Fields: int32 MercStatus, uint32 MercCount, uint32 MercID
    let mut data = Vec::with_capacity(12);
    data.put_i32_le(-1); // MercStatus
    data.put_u32_le(0);  // MercCount
    data.put_u32_le(1);  // MercID (Observed as 1 in RoF2 for no merc)
    data
}





