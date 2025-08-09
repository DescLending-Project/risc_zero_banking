# Zero-Knowledge Based Credit Scoring System

A privacy-preserving hybrid credit scoring system built with RISC Zero that combines traditional financial data (FICO-like scores, fetched with TLSNotary) with on-chain DeFi activity to asses creditworthiness without revealing sensitive financial data.

## Architecture

![Architecture Diagram](./workflow.png)

The system consists of four main components:

- **Inner Proofs**: Individual proofs for different credit factors (fetched TradFi score, DeFi activity assesment and fetched stateroot, for data integrity verification)
- **Outer Proof**: Aggregated proof that verifies all inner proofs and calculates the final credit score
- **Smart Contracts**: On-chain verification and score publishing infrastructure
- **Custom Libraries**: Shared utilities 

## Directory Structure
'''
├── lib/                   # Custom libraries and utilities
├── risc0_proofs/          # Standalone RISC Zero proof implementation
├── score_publisher/       # Integrated scoring system with nested proof verification and on-chain publishing
└── solidity/              # Smart contracts for verification and credit score management
'''
