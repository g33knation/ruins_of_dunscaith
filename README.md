# Ruins of Dunscaith 🏰

A modernized, high-performance, and AI-native EverQuest Emulator (EQEmu) server suite implemented in Rust.

## 🎯 Goal
The primary objective of **Ruins of Dunscaith** is to modernize the classic EverQuest server infrastructure using safe, concurrent, and high-performance technologies. By leveraging Rust, we aim to provide a more stable and scalable alternative to legacy server implementations, while integrating advanced AI-driven maintenance and development workflows.

## 🏗️ Project Architecture
The project is organized into several modular components to ensure separation of concerns and ease of development:

*   **`akk-rust`**: The core library providing shared logic and engine capabilities.
*   **`login-server`**: Handles authentication and server list management.
*   **`world-server`**: Manages global game state, characters, and cross-zone communication.
*   **`zone-server`**: High-performance instance servers for individual game zones.
*   **`shared`**: Common data structures, packet definitions, and network utilities.
*   **`ZeroClaw`**: An integrated AI infrastructure that provides autonomous codebase analysis, RAG-driven context awareness, and assisted refactoring.

## 🚀 Current Progress
We are currently in active development, focusing on the following areas:

*   **Rust Port Completion**: Core networking and server logic have been successfully ported to Rust.
*   **Protocol Modernization**: Transitioning from legacy hardcoded packet handling to a robust, enum-based opcode system (`OpCode`).
*   **AI-Assisted Development**: Full integration of `ZeroClaw` with codebase-wide RAG (Retrieval-Augmented Generation) for intelligent code navigation and security auditing.
*   **Security & Stability**: Implementing strict security policies and auditing mechanisms to ensure a safe environment for developers and players.

## 🛠️ Development & Tooling
This project utilizes specialized tooling to maintain a high development velocity:

*   **Codebase RAG**: Provides immediate, indexed access to the entire project for developers and AI agents.
*   **Modern Rust Stack**: Built with the latest stable Rust features for maximum performance and memory safety.

---
> [!NOTE]
> This project is currently in the **Pre-Alpha** stage. We are focusing on core stability and protocol accuracy before general availability.
