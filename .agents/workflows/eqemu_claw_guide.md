---
description: EQEmu Rust Codebase Architecture and Claw Agent Guidelines
---

# EQEmu Rust "Super Titan" Claw Guidelines

This document provides future LLMs (ZeroClaw agents) with the exact architectural patterns, strict rules, and context required to effectively modify the Ruins of Dunscaith EQEmu server codebase. By following this guide, you will execute tasks with the precision of a "super titan."

## 1. Project Structure & Scope
The project is a Cargo Workspace containing four main crates. You must respect the boundaries of each crate:
- **`shared`**: Contains global constants, the universal `OpCode` enum, and shared database models.
- **`login-server`**: Handles authentication (TCP for old clients, UDP for modern RoF2) and server discovery.
- **`world-server`**: Handles character selection, global routing, and cross-zone chat.
- **`zone-server`**: Handles the actual game simulation, NPC AI, combat, and entity movement.

## 2. Networking & OpCodes (CRITICAL)
- **The OpCode Enum**: All application-level OpCodes MUST use the `shared::opcodes::OpCode` enum. **DO NOT** use raw magic numbers (e.g., `0x7a09`, `0x5089`) in match arms or when sending packets. If an opcode is missing, add it to `shared/src/opcodes.rs` first.
- **Transport vs Application**: The transport layer (EQStream) processes raw `u16` opcodes and unzips payloads, then translates them into `OpCode` enums before passing them to the application layer (Actors).
- **Serialization**: Use `binrw` (`#[derive(BinRead, BinWrite)]` with `#[bw(little)]`) for parsing fixed, structured packets. For dynamic payloads (like Login tokens or variable strings), use `bytes::BufMut` (`put_u32_le`, `put_slice`, etc.) or `std::io::Cursor`.

## 3. Database Interactions (sqlx)
- All database queries use PostgreSQL via `sqlx`.
- **Compile-Time Macros vs. Dynamic Queries**: The codebase heavily uses `sqlx::query!` and `sqlx::query_as!`. However, during heavy refactoring when the PostgreSQL database is not locally accessible to the agent, you must bypass compile-time checks by either:
  1. Reverting to `sqlx::query("SELECT ...").fetch(...)`
  2. Setting `SQLX_OFFLINE=1` when running `cargo check`.
  3. Creating dummy structs with `#[derive(sqlx::FromRow)]`.

## 4. The Actor Model Architecture
- The codebase uses the Actor Pattern via `tokio::sync::mpsc` for concurrency.
- Example: `LoginSessionActor` receives network bytes, translates them into events, updates its state, and pushes responses back via a transmitter (`tx`).
- **Rule**: Do NOT wrap entire massive states in `Arc<Mutex<State>>`. Share data asynchronously through channel messages.

## 5. Typical Workflow for Protocol Handlers
When the user asks you to implement a new EQ protocol packet (e.g., OP_ApproveWorld):
1. **Locate Reference**: Look at the original C++ header (`temp_rof2_structs.h` or similar) to identify the packet layout (types and padding).
2. **Implement Struct**: Create a `binrw` struct in the appropriate crate's `net` or `packet` module. Pay strict attention to byte alignment (`u32`, `u8`, fixed arrays `[u8; 64]`).
3. **Register OpCode**: Ensure the opcode exists in `shared/src/opcodes.rs`.
4. **Implement Match Arm**: Add a match arm to the relevant Actor's `handle_application_packet` method.
5. **Verify**: Run `SQLX_OFFLINE=1 cargo check` before notifying the user.

## Note on OpCode Collisions
EqEmu uses different opcodes depending on the direction (Client->Server vs Server->Client). If 0x0004 is `ServerListRequest` (C->S) but is also `LoginAccepted` (S->C), reuse the variant `OpCode::ServerListRequest` but comment clearly `// Sent as OP_LoginAccepted (0x0004)`.
