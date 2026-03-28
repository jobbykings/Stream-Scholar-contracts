# Stream Scholar Contracts

A comprehensive Soroban smart contract system for educational scholarships and academic credentialing on the Stellar network.

## Features

### Core Scholarship System
- **Dynamic Pricing**: Time-based access pricing with discounts for engaged students
- **Subscription Tiers**: Bulk course access with monthly subscriptions
- **Heartbeat System**: Active learning verification through periodic check-ins
- **Minimum Deposits**: Configurable minimum funding requirements

### Issue #88: Multi-Token Book Stipend Voucher
- **Restricted Asset Drip**: Book tokens that can only be redeemed at verified bookstores
- **Donor Control**: Donors create vouchers with specific educational purposes
- **Spending Transparency**: Ensures educational credits are used as intended
- **Expiry System**: Time-limited vouchers to encourage timely usage

### Issue #89: Zero-Knowledge GPA Verification Proof
- **Privacy-Preserving Verification**: Students prove GPA > 3.5 without revealing exact grades
- **ZK-Proof Integration**: Oracle uploads verification proofs instead of raw GPA data
- **Financial Assurance**: Grantors receive confirmation of academic performance
- **Academic Privacy**: Protects sensitive student information

### Issue #90: Soulbound Scholarship Credential Minter
- **On-Chain Diplomas**: Permanent, non-transferable credential NFTs
- **Rich Metadata**: Includes total hours funded, major, and donor organization
- **Social Capital**: Verifiable credentials for employers and other DAOs
- **Graduation Triggers**: Automated credential minting upon program completion

### Issue #91: Inter-Protocol Reputation Sync
- **Learning Velocity Score**: Cross-contract academic reputation metrics
- **Ecosystem Synergy**: Stream-Scholar reputation benefits in Grant-Stream
- **Junior Grant Prioritization**: High-scoring students get preferential treatment
- **Learning-to-Earning Pipeline**: Academic achievement leads to professional opportunities

## Project Structure

```text
.
├── contracts
│   └── scholar_contracts
│       ├── src
│       │   ├── lib.rs
│       │   └── test.rs
│       └── Cargo.toml
├── Cargo.toml
└── README.md
```

## Key Data Structures

### BookStipendVoucher
- `voucher_id`: Unique identifier
- `donor`/`student`: Participants
- `amount`: Book token quantity
- `verified_bookstores`: Approved redemption locations

### ZKGPAProof
- `proof_hash`: Cryptographic proof
- `verification_level`: Minimum GPA threshold (35 = 3.5)
- `verified_at`: Proof submission timestamp

### SoulboundCredential
- `credential_id`: Unique identifier
- `total_hours_funded`: Educational investment
- `major`/`donor_organization`: Academic context

### LearningVelocityScore
- `score`: Calculated reputation metric
- `courses_completed`/`avg_completion_time`: Performance data

## Testing

Run the comprehensive test suite:

```bash
cargo test
```

Tests include:
- Core scholarship functionality
- Book stipend voucher flows
- ZK GPA verification
- Soulbound credential minting
- Cross-contract reputation queries

## Deployed Contract
- **Network:** Stellar Testnet
- **Contract ID:** CB7OZPTIUENDWJWNHRGDPZLIEIS6TXMFRYT4WCGHIZVYLCTXEONC6VHY
