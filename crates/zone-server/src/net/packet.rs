use binrw::{BinRead, BinWrite};

#[derive(Debug, BinRead)]
#[br(little)]
pub struct ZoneSessionRequest {
    pub unknown: u32,
    pub session_id: u32,
}

#[derive(Debug, BinRead)]
#[br(little)]
pub struct ZoneEntry {
    // RoF2 OP_ZoneEntry (0x1900) appears to just be the character name
    // as a null-terminated string (not fixed 64 bytes)
    // We'll read the first 64 bytes and parse the null-terminated string
    pub char_name: [u8; 64],
}

pub struct PlayerProfile {
    // Header
    pub name: [u8; 64],
    pub last_name: [u8; 32],
    pub level: u8,
    pub race: u16,
    pub class: u8,
    pub gender: u8,
    pub deity: u16,
    pub entity_id: u32,
    pub zone_id: u16,
    pub zone_instance: u16,
    
    // Appearances
    pub face: u8,
    pub hair_color: u8,
    pub hair_style: u8,
    pub beard: u8,
    pub beard_color: u8,
    pub eye_color_1: u8,
    pub eye_color_2: u8,
    pub drakkin_heritage: u32,
    pub drakkin_tattoo: u32,
    pub drakkin_details: u32,

    // Stats
    pub cur_hp: u32,
    pub mana: u32,
    pub endurance: u32,
    pub str: u32,
    pub sta: u32,
    pub dex: u32,
    pub agi: u32,
    pub int: u32,
    pub wis: u32,
    pub cha: u32,

    // Status
    pub intoxication: u32,
    pub toxicity: u32,
    pub hunger_level: u32,
    pub thirst_level: u32,

    // Currency
    pub platinum: u32,
    pub gold: u32,
    pub silver: u32,
    pub copper: u32,

    // Experience
    pub exp: u32,
    pub points: u32, // Practice points

    // Coordinates
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub heading: f32,
}

impl Default for PlayerProfile {
    fn default() -> Self {
        Self {
            name: [0u8; 64],
            last_name: [0u8; 32],
            level: 1,
            race: 1, 
            class: 1,
            gender: 0,
            deity: 201,
            entity_id: 0,
            zone_id: 202,
            zone_instance: 0,
            face: 0,
            hair_color: 0,
            hair_style: 0,
            beard: 0,
            beard_color: 0,
            eye_color_1: 0,
            eye_color_2: 0,
            drakkin_heritage: 0,
            drakkin_tattoo: 0,
            drakkin_details: 0,
            cur_hp: 100,
            mana: 100,
            endurance: 100,
            str: 100,
            sta: 100,
            dex: 100,
            agi: 100,
            int: 100,
            wis: 100,
            cha: 100,
            intoxication: 0,
            toxicity: 0,
            hunger_level: 1000,
            thirst_level: 1000,
            platinum: 0,
            gold: 0,
            silver: 0,
            copper: 0,
            exp: 0,
            points: 0,
            x: 0.0,
            y: 0.0,
            z: 10.0,
            heading: 0.0,
        }
    }
}

impl binrw::BinWrite for PlayerProfile {
    type Args<'a> = ();
    fn write_options<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        endian: binrw::Endian,
        _: Self::Args<'_>,
    ) -> binrw::BinResult<()> {
        let start = writer.stream_position()?;
        const TOTAL_SIZE: u64 = 19572;
        
        let pad_to = |w: &mut W, offset: u64| -> binrw::BinResult<()> {
            let current = w.stream_position()?;
            let target = start + offset;
            if current < target {
                let diff = (target - current) as usize;
                 w.write_all(&vec![0u8; diff])?;
            }
            Ok(())
        };

        // 0x0000: Checksum (u32)
        0u32.write_options(writer, endian, ())?;
        // 0x0004: Checksum Size (u32)
        (TOTAL_SIZE as u32 - 9).write_options(writer, endian, ())?;
        
        // 0x0010 (16): Gender (u8)
        pad_to(writer, 16)?;
        self.gender.write_options(writer, endian, ())?;
        
        // 0x0011 (17): Race (u32)
        pad_to(writer, 17)?;
        (self.race as u32).write_options(writer, endian, ())?;
        
        // 0x0015 (21): Class (u8)
        pad_to(writer, 21)?;
        self.class.write_options(writer, endian, ())?;
        
        // 0x0016 (22): Level (u8)
        pad_to(writer, 22)?;
        self.level.write_options(writer, endian, ())?;
        
        // 0x0017 (23): Level1 (u8)
        self.level.write_options(writer, endian, ())?;

        // 0x0018 (24): Bind Count (u32)
        pad_to(writer, 24)?;
        1u32.write_options(writer, endian, ())?;

        // 0x001C (28): Bind Point 1 (Zone, X, Y, Z, Heading)
        pad_to(writer, 28)?;
        (self.zone_id as u32).write_options(writer, endian, ())?;
        self.x.write_options(writer, endian, ())?;
        self.y.write_options(writer, endian, ())?;
        self.z.write_options(writer, endian, ())?;
        self.heading.write_options(writer, endian, ())?;

        // 0x0080 (128): Deity (u32)
        pad_to(writer, 128)?;
        (self.deity as u32).write_options(writer, endian, ())?;

        // Appearances (offsets from rof2_structs.h)
        // 0x0378 (888): haircolor (u8)
        pad_to(writer, 888)?;
        self.hair_color.write_options(writer, endian, ())?;
        self.beard_color.write_options(writer, endian, ())?;
        pad_to(writer, 894)?;
        self.eye_color_1.write_options(writer, endian, ())?;
        self.eye_color_2.write_options(writer, endian, ())?;
        self.hair_style.write_options(writer, endian, ())?;
        self.beard.write_options(writer, endian, ())?;
        self.face.write_options(writer, endian, ())?;
        self.drakkin_heritage.write_options(writer, endian, ())?;
        self.drakkin_tattoo.write_options(writer, endian, ())?;
        self.drakkin_details.write_options(writer, endian, ())?;

        // 0x03AC (940): Practice Points (u32)
        pad_to(writer, 940)?;
        self.points.write_options(writer, endian, ())?;

        // 0x03B0 (944): Mana (u32)
        self.mana.write_options(writer, endian, ())?;

        // 0x03B4 (948): CurHP (u32)
        self.cur_hp.write_options(writer, endian, ())?;

        // 0x03B8 (952): STR, STA, CHA, DEX, INT, AGI, WIS (u32s)
        self.str.write_options(writer, endian, ())?;
        self.sta.write_options(writer, endian, ())?;
        self.cha.write_options(writer, endian, ())?;
        self.dex.write_options(writer, endian, ())?;
        self.int.write_options(writer, endian, ())?;
        self.agi.write_options(writer, endian, ())?;
        self.wis.write_options(writer, endian, ())?;

        // Status
        // 0x3265 (12901): intoxication
        pad_to(writer, 12901)?;
        self.intoxication.write_options(writer, endian, ())?;
        self.toxicity.write_options(writer, endian, ())?;
        pad_to(writer, 12913)?;
        self.thirst_level.write_options(writer, endian, ())?;
        self.hunger_level.write_options(writer, endian, ())?;

        // 0x3245 (12869): Platinum (u32)
        pad_to(writer, 12869)?;
        self.platinum.write_options(writer, endian, ())?;
        self.gold.write_options(writer, endian, ())?;
        self.silver.write_options(writer, endian, ())?;
        self.copper.write_options(writer, endian, ())?;

        // 0x3618 (13848): Name (64 bytes)
        pad_to(writer, 13848)?;
        writer.write_all(&self.name)?;

        // 0x3658 (13912): Last Name Len (u32)
        pad_to(writer, 13912)?;
        32u32.write_options(writer, endian, ())?;

        // 0x365C (13916): Last Name (32 bytes)
        pad_to(writer, 13916)?;
        writer.write_all(&self.last_name)?;

        // 0x36B8 (14008): Zone ID (u16)
        pad_to(writer, 14008)?;
        self.zone_id.write_options(writer, endian, ())?;

        // 0x36BA (14010): Zone Instance (u16)
        self.zone_instance.write_options(writer, endian, ())?;

        // 0x36BC (14012): Y, X, Z, Heading (Floats)
        pad_to(writer, 14012)?;
        self.y.write_options(writer, endian, ())?;
        self.x.write_options(writer, endian, ())?;
        self.z.write_options(writer, endian, ())?;
        self.heading.write_options(writer, endian, ())?;

        // Final Padding to match RoF2 size
        pad_to(writer, TOTAL_SIZE)?;
        
        Ok(())
    }
}

#[derive(Debug, BinWrite)]
#[bw(little)]
pub struct CharSpells {
    // Empty for now (spawn packet)
    pub unknown: u32, 
}

#[derive(Debug, BinRead, BinWrite)]
#[br(little)]
pub struct ClientUpdate {
    pub spawn_id: u16,          // 0x00
    pub sequence: u16,          // 0x02
    pub unknown0004: u32,       // 0x04
    pub x_pos: f32,             // 0x08
    pub y_pos: f32,             // 0x0C
    pub flags1: u32,            // 0x10 (delta_heading:10, animation:10, padding:12)
    pub delta_x: f32,           // 0x14
    pub delta_y: f32,           // 0x18
    pub z_pos: f32,             // 0x1C
    pub delta_z: f32,           // 0x20
    pub flags2: u32,            // 0x24 (animation:10, heading:12, padding:10)
}

impl ClientUpdate {
    pub fn heading(&self) -> f32 {
        // flags2 bits 10-21 are heading
        let h_raw = (self.flags2 >> 10) & 0xFFF;
        (h_raw as f32) / 4096.0 * 360.0
    }
}


#[derive(Debug, BinWrite)]
#[bw(little)]
pub struct ItemData {
    // Empty for now
    pub unknown: u32,
}

#[derive(Debug, BinWrite)]
#[bw(little)]
pub struct SpawnAppearance {
    pub spawn_id: u16,
    pub type_: u16,
    pub parameter: u32,
}

pub struct Spawn {
    pub name: String,
    pub last_name: String,
    pub spawn_id: u32,
    pub level: u8,
    pub race: u32,
    pub class: u8,
    pub gender: u8,
    pub cur_hp: u8,
    pub max_hp: u8,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub heading: f32,
    pub npc: u8, // 0=player, 1=npc, 2=pc corpse, 3=npc corpse
}

impl binrw::BinWrite for Spawn {
    type Args<'a> = ();
    fn write_options<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        endian: binrw::Endian,
        _: Self::Args<'_>,
    ) -> binrw::BinResult<()> {
        let start = writer.stream_position()?;
        let pad_to = |w: &mut W, offset: u64| -> binrw::BinResult<()> {
            let current = w.stream_position()?;
            let target = start + offset;
            if current < target {
                let diff = (target - current) as usize;
                 w.write_all(&vec![0u8; diff])?;
            }
            Ok(())
        };

        // 0x0000: Name (64 bytes fixed for simplicity in this stub)
        let mut name_buf = [0u8; 64];
        let n_len = self.name.len().min(63);
        name_buf[..n_len].copy_from_slice(&self.name.as_bytes()[..n_len]);
        writer.write_all(&name_buf)?;
        
        // 0x0040 (64): spawnId (u32)
        self.spawn_id.write_options(writer, endian, ())?;
        
        // 0x0044 (68): level (u8)
        self.level.write_options(writer, endian, ())?;
        
        // 0x0045 (69): bounding_radius (float)
        0.1f32.write_options(writer, endian, ())?;
        
        // 0x0049 (73): NPC (u8)
        self.npc.write_options(writer, endian, ())?;
        
        // 0x004A (74): Bitfields (5 bytes)
        writer.write_all(&[self.gender & 0x03, 0, 0, 0, 0])?;
        
        // 0x004F (79): otherData (u8)
        0u8.write_options(writer, endian, ())?;
        
        // 0x0050 (80): sizes/speeds
        pad_to(writer, 88)?;
        1.0f32.write_options(writer, endian, ())?; // size
        
        pad_to(writer, 93)?;
        1.0f32.write_options(writer, endian, ())?; // walkspeed
        1.0f32.write_options(writer, endian, ())?; // runspeed
        
        // 0x0065 (101): race (u32)
        pad_to(writer, 101)?;
        self.race.write_options(writer, endian, ())?;
        
        // 0x006C (108): curHp (u8)
        pad_to(writer, 108)?;
        self.cur_hp.write_options(writer, endian, ())?;
        
        // Appearances
        pad_to(writer, 109)?;
        0u8.write_options(writer, endian, ())?; // hair
        0u8.write_options(writer, endian, ())?; // beard
        0u8.write_options(writer, endian, ())?; // eye1
        0u8.write_options(writer, endian, ())?; // eye2
        1u8.write_options(writer, endian, ())?; // hairstyle
        0u8.write_options(writer, endian, ())?; // beard
        
        // 0x011E (286): class_ (u8)
        pad_to(writer, 286)?;
        self.class.write_options(writer, endian, ())?;
        
        // 0x0120 (288): StandState (u8)
        100u8.write_options(writer, endian, ())?;
        
        // 0x0122 (290): lastName (64 bytes fixed for simplicity)
        pad_to(writer, 290)?;
        let mut last_name_buf = [0u8; 64];
        let ln_len = self.last_name.len().min(63);
        last_name_buf[..ln_len].copy_from_slice(&self.last_name.as_bytes()[..ln_len]);
        writer.write_all(&last_name_buf)?;

        // Position at variable offset? Let's use 489 as a baseline
        pad_to(writer, 489)?;
        // Position bitpacking is hard, let's just write raw floats at some offset 
        // and hope the client has a fallback or we send a ClientUpdate immediately.
        self.y.write_options(writer, endian, ())?;
        self.x.write_options(writer, endian, ())?;
        self.z.write_options(writer, endian, ())?;
        self.heading.write_options(writer, endian, ())?;

        Ok(())
    }
}
