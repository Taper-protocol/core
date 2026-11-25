# Taper Protocol (`$TAPR`) — Core

Autonomous deflationary primitive built on Solana Token-2022 and Anchor.

## Overview

Taper Protocol enforces a hardcoded **0.05% (5 bps) transfer fee** at the Token-2022 mint level and automates fee collection via an immutable, permissionless Anchor smart contract:

- **Fixed Initial Supply:** `21,000,000 TAPR`
- **Transfer Fee:** `0.05%` (`5 bps`) on transfers
  - **0.04% (80% of fee):** Permanently burned, reducing circulating supply
  - **0.01% (20% of fee):** Paid as a permissionless reward to any account that triggers fee harvesting and burning
- **Immutability Target:** Mint, Freeze, Transfer Fee Config, Metadata Update, and Program Upgrade authorities are permanently revoked upon Mainnet finalization.

## Repository Structure

```text
core/
├── programs/        # Anchor smart contract (Rust)
├── tests/           # Integration and negative security tests
├── scripts/         # Token-2022 initialization and harvesting scripts
├── Anchor.toml
├── Cargo.toml
└── Cargo.lock       # Committed for deterministic solana-verify builds
