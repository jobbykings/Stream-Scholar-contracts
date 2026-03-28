# Pull Request: Implement SSI verification and geographic zoning for scholarships

## Summary
This PR implements two critical features for the Stream-Scholar platform:

1. **Self-Sovereign Identity (SSI) Verification** - Addresses Issue #98
2. **Geographic Zoning for Local Impact Scholarships** - Addresses Issue #96

## Features Implemented

### SSI Verification (#98)
- **Stellar SEP-12 Integration**: Support for Stellar's SEP-12 KYC/identity verification
- **Gitcoin Passport Integration**: Support for Gitcoin Passport identity verification
- **Verified Personhood Score**: Minimum score requirement (80) for high-value scholarships
- **Sybil Attack Prevention**: Students cannot create multiple wallets to farm scholarship funds
- **Identity Verification Functions**:
  - `verify_ssi_identity()`: Verify student identity with proof data
  - `is_ssi_verified()`: Check if student has valid SSI verification
  - `get_personhood_score()`: Retrieve student's personhood score

### Geographic Zoning (#96)
- **Geographic Targeting**: Donors can fund students from specific regions (Abuja, Lagos, etc.)
- **Geohash Verification**: Precise location verification using geohash technology
- **Regional Oracle System**: Verified oracles sign proof of residency documents
- **Location Monitoring**: IP-to-Location backend checks detect location changes
- **Review State Logic**: Automatic review triggers when students move out of restricted zones
- **Geographic Functions**:
  - `set_regional_oracle()`: Admin sets authorized oracles for regions
  - `verify_residency()`: Students verify residency with geohash and oracle signature
  - `check_location_compliance()`: Monitor location changes
  - `is_in_geographic_review()`: Check if student is under geographic review

### Streaming Scholarships
- **create_stream()**: Create streaming scholarships with SSI and geographic requirements
- **High-Value Protection**: Streams >1000 tokens/month require SSI verification
- **Geographic Restrictions**: Optional geographic targeting for community scholarships
- **Stream Management**: deposit, withdraw, pause, resume functions
- **Compliance Checks**: Automatic verification during stream operations

## Security & Compliance Benefits

### For Donors
- **Verified Recipients**: Funds go to real human beings with verified identities
- **Geographic Assurance**: Community scholarships stay within intended regions
- **Sybil Protection**: Prevents one person from creating multiple wallets
- **Trust Building**: Donors know their money supports legitimate students

### For Students
- **Identity Protection**: SSI gives students control over their identity data
- **Fair Access**: Geographic restrictions ensure local students get local opportunities
- **Streamlined Verification**: Single verification works across multiple scholarships

## Technical Implementation

### New Data Structures
- `SsiVerification`: Stores verification details and personhood score
- `GeographicInfo`: Stores geohash, region, and compliance status
- `Stream`: Manages streaming scholarship with restrictions

### Smart Contract Functions
- **Identity Management**: 3 new functions for SSI verification
- **Geographic Management**: 4 new functions for location verification
- **Stream Management**: 6 new functions for streaming scholarships
- **Admin Functions**: Regional oracle management

### Events
- `SSI_VERIFIED`: Published when student completes identity verification
- `GEO_REVIEW`: Published when student enters geographic review state
- `STREAM_CREATED`: Published when new stream is created

## Test Coverage
Added comprehensive tests covering:
- SSI verification with valid and invalid scores
- Geographic verification and compliance checking
- Stream creation with SSI requirements
- Geographic restrictions and review states
- Stream management operations

## Constants & Configuration
- `MIN_PERSONHOOD_SCORE`: 80 (configurable minimum score)
- `GEOHASH_PRECISION`: 9 (geohash precision level)
- `REVIEW_COOLDOWN`: 24 hours (review state duration)
- `LOCATION_CHECK_INTERVAL`: 1 hour (compliance check frequency)

## Usage Examples

### SSI Verification
```rust
// Verify student with Gitcoin Passport
client.verify_ssi_identity(
    &student, 
    &Symbol::new(&env, "gitcoin_passport"), 
    &85, 
    &proof_data
);
```

### Geographic Verification
```rust
// Set regional oracle (admin only)
client.set_regional_oracle(&admin, &Symbol::new(&env, "lagos"), &oracle);

// Student verifies residency
client.verify_residency(
    &student, 
    &geohash, 
    &Symbol::new(&env, "lagos"), 
    &proof_signature, 
    &oracle
);
```

### Create Restricted Stream
```rust
// Create stream with geographic restriction
client.create_stream(
    &funder, 
    &student, 
    &1000, // tokens per second
    &token_address, 
    Some(Symbol::new(&env, "lagos")) // restricted to Lagos
);
```

## Impact
This implementation transforms Stream-Scholar from a wallet-based system to a **human-centric platform** that:

1. **Prevents Fraud**: Sybil attacks and identity fraud are significantly reduced
2. **Ensures Impact**: Donor funds reach intended geographic communities
3. **Builds Trust**: Verified identities and geographic compliance increase donor confidence
4. **Enables Compliance**: Meets regulatory requirements for financial transactions
5. **Supports Local Development**: Geographic zoning drives community-focused educational investment

## Testing
Comprehensive test suite added with 5 new test functions covering all major functionality.

## Breaking Changes
No breaking changes - all new functionality is additive and maintains backward compatibility.
