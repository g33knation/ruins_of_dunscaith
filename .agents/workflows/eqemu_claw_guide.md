---
description: EQEmu Rust Codebase Architecture and Claw Agent Guidelines
---

# EQEmu Rust "Super Titan" Claw Guidelines

This document provides future LLMs (ZeroClaw agents) with the exact architectural patterns, strict rules, and context required to effectively modify the Ruins of Dunscaith EQEmu server codebase.

## 🚨 MANDATORY: Read the Learning Log First (MemLog)
Before performing any modification, you **MUST** read the [MemLog](file:///home/t/.gemini/antigravity/brain/b9b15725-21b6-4c47-b121-b145fa3be957/zeroclaw_learning_log.md). This file documents common failures (hallucination, context limits, path issues) and the architectural standards established to fix them. Ignoring this log will result in failure.

## 1. Project Structure & Scope
The project is a Cargo Workspace containing four main crates. You must respect the boundaries of each crate:
- **`shared`**: Contains global constants, the universal `OpCode` enum, and shared database models.
- **`login-server`**: Handles authentication (TCP for old clients, modern RoF2) and server discovery.
- **`world-server`**: Handles character selection, global routing, and cross-zone chat.
- **`zone-server`**: Handles game simulation, NPC AI, combat, and entity movement.

## 2. Networking & OpCodes (CRITICAL)
- **The OpCode Enum**: All application-level OpCodes MUST use the `shared::opcodes::OpCode` enum. **DO NOT** use raw magic numbers (e.g., `0x7a09`, `0x5089`).
- **Serialization**: Use `binrw` (`#[derive(BinRead, BinWrite)]` with `#[bw(little)]`) for fixed packets. NEVER manually build large packets with `vec.push()`.

## 3. Database Interactions (sqlx)
- All database queries use PostgreSQL via `sqlx`.
- **Repository Pattern**: Abstract DB calls into helper methods. Use `MOCK` stubs for local testing if the pool is unavailable.

## 4. The Actor Model Architecture
- The codebase uses the Actor Pattern via `tokio::sync::mpsc`. Do NOT use massive `Mutex` locks for state. Use asynchronous message passing.

## 5. Typical Workflow
When implementing a new packet:
1. **Locate Reference**: Use `temp_rof2_structs.h`.
2. **Implement Struct**: Create a `binrw` struct in `shared/src/packets.rs`.
3. **Handle OpCode**: Add a match arm in the relevant Actor.
4. **Verify**: Run `SQLX_OFFLINE=1 cargo check`.
