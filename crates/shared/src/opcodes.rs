use num_derive::FromPrimitive;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, Serialize, Deserialize)]
#[repr(u16)]
pub enum OpCode {
    // Login Opcodes
    SessionReady = 0x0001,
    Login = 0x0002,
    Login2 = 0x0003,
    LoginApproval = 0x0017,
    ServerListRequest = 0x0004, // Corrected from 0x0005 to match login_server expectation or variant?
    ServerListResponse = 0x0018, // Fixed alignment
    PlayEverquestRequest = 0x000d,
    PlayEverquestResponse = 0x0021,

    // World Opcodes
    SendLoginInfo = 0x7a09,
    ApproveWorld = 0x7499,
    LogServer = 0x7ceb,
    SendCharInfo = 0x00d2,
    CharSelectRequest = 0x00d1,
    ExpansionInfo = 0x590d,
    GuildsList = 0x507a,
    EnterWorld = 0x57c3,
    SendMaxCharacters = 0x5475,
    SendMembership = 0x7acc,
    SendMembershipDetails = 0x057b,
    CharacterCreate = 0x6bbf,
    CharacterCreateRequest = 0x6773,
    DeleteCharacter = 0x1808,
    ApproveName = 0x56a2,
    Motd = 0x0c22,
    SendZonePoints = 0x3234,
    TributeInfo = 0x4254,
    TimeOfDay = 0x5070,
    MercenaryData = 0x3e98,
    Weather = 0x661e,
    ZoneServerInfo = 0x4c44,

    // Zone Opcodes (RoF2)
    ZoneEntry = 0x5089,
    ZoneEntry2 = 0x1900,
    ZoneEntry3 = 0x3747,
    ClientUpdate = 0x7DFC,
    ClientReady = 0x345d,
    ReqClientSpawn = 0x35FA,
    SendExpZonein = 0x5f8e,
    SendAAStats = 0x43c8,
    SendTributes = 0x729b,
    LevelUpdate = 0x1eec,
    Stamina = 0x2a79,
    CharInventory = 0x5ca6,
    PlayerProfile = 0x6506,
    Spawn = 0x6968,
    DeleteSpawn = 0x6a0c,
    TargetMouse = 0x184d,
    Damage = 0x1f0e,
    Attack = 0x759c,
    ChannelMessage = 0x00d9,
    ShopRequest = 0x7422,
    ShopEnd = 0x2213,
    ShopBuy = 0x3b1c,
    ShopSell = 0x6582,
    ShopList = 0x794a,
    MoneyUpdate = 0x4859,

    // RoF2 Specifics / Observed
    RoF2ClientReady = 0x1100,
    RoF2Unknown1500 = 0x1500,

    // Client Echoes / CRCs
    WorldClientCrc1 = 0x0f13,
    WorldClientCrc2 = 0x4b8d,
    WorldClientCrc3 = 0x298d,

    Unknown = 0xFFFF,
}

impl Default for OpCode {
    fn default() -> Self {
        OpCode::Unknown
    }
}
