# Pull Request: Study Group Collateral Lock & GPA-Flow Math Verification

## Summary
This PR implements two critical security and logic features for the Stream-Scholar platform:

1. **Study-Group Collateral Lock for Joint Grants** - Addresses Issue #97
2. **Formal Verification of GPA-Flow Math Invariants** - Addresses Issue #99

## Features Implemented

### Study Group Collateral Lock (#97)

#### Problem Solved
Team projects often receive shared grants, but ensuring all members contribute fairly is challenging. This implementation provides on-chain accountability for group-based scholarships.

#### Core Features
- **3-Member Study Groups**: Exactly 3 students share one streaming grant
- **Collateral Locking**: Each member locks XLM as collateral (configurable amount)
- **Democratic Voting System**: 2/3 vote required to pause or slash members
- **Two-Stage Accountability**:
  1. **Pause**: Member's share is paused if voted out (no withdrawals)
  2. **Slash**: Collateral is slashed to fund replacement member
- **Automatic Member Replacement**: Slashed members are replaced with new members
- **Equal Share Distribution**: Grant stream split equally (1/3 each) among active members

#### New Functions
- `create_study_group()`: Create 3-member group with collateral requirements
- `lock_collateral()`: Members lock their XLM collateral
- `vote_to_pause_member()`: Vote to pause non-contributing member
- `vote_to_slash_collateral()`: Vote to slash collateral and replace member
- `withdraw_from_group_stream()`: Withdraw member's share (if not paused)
- `get_member_status()`: Check member's collateral status
- `get_study_group_info()`: Retrieve group details

#### Security Benefits
- **On-Chain Accountability**: Financial consequences for non-participation
- **Democratic Governance**: Prevents single-member abuse
- **Collateral Protection**: Funds secured until replacement is found
- **Fair Distribution**: Ensures active members receive their full share

### GPA-Flow Math Verification (#99)

#### Problem Solved
When flow rates change based on academic performance, complex mathematical calculations can lead to overspending. This implementation provides formal mathematical guarantees.

#### Core Features
- **GPA-Based Flow Rates**: Academic performance directly impacts funding rate
- **Linear Bonus System**: 0% bonus at 2.0 GPA, 20% bonus at 4.0 GPA
- **Minimum GPA Threshold**: 2.0 GPA required for any funding
- **Academic Oracle System**: Authorized institutions verify GPA records
- **Budget Invariant Verification**: Mathematical proof that total spending never exceeds grant amount
- **Dynamic Flow Adjustment**: Real-time rate updates based on new GPA data

#### Mathematical Guarantee
The core invariant: **Sum(FlowRate × DeltaTime) ≤ Max_Grant_Amount**

This ensures:
- No overspending due to calculation errors
- Protection against rounding errors
- Donor confidence in budget compliance
- Formal verification of financial safety

#### New Functions
- `set_academic_oracle()`: Admin sets authorized academic institutions
- `verify_gpa()`: Students submit GPA for verification
- `calculate_gpa_adjusted_flow_rate()`: Calculate rate based on GPA
- `create_gpa_adjusted_stream()`: Create stream with GPA-based rates
- `verify_budget_invariant()`: Core mathematical verification
- `withdraw_with_math_verification()`: Safe withdrawal with invariant check
- `update_gpa_and_flow()`: Dynamic rate updates
- `get_budget_status()`: Monitor budget compliance
- `get_gpa_info()`: Retrieve verified GPA records

#### Academic Incentives
- **Performance-Based Funding**: Higher GPA = Higher flow rate
- **Academic Motivation**: Direct financial rewards for excellence
- **Institutional Integration**: Schools can verify student achievement
- **Merit-Based Distribution**: Funds flow based on academic merit

## Technical Implementation

### New Data Structures

#### Study Group Structures
```rust
StudyGroup {
    group_id: u64,
    members: Vec<Address>, // Exactly 3 members
    grant_stream: Stream,
    collateral_per_member: i128,
    is_active: bool,
    created_at: u64,
}

MemberCollateral {
    member: Address,
    group_id: u64,
    collateral_amount: i128,
    is_locked: bool,
    is_slashed: bool,
    is_paused: bool,
    locked_at: u64,
}

VoteRecord {
    voter: Address,
    target_member: Address,
    group_id: u64,
    vote_type: Symbol, // "pause" or "slash"
    voted_at: u64,
}
```

#### GPA & Math Verification Structures
```rust
GpaRecord {
    student: Address,
    gpa_scaled: u64, // GPA × 100 (e.g., 3.5 = 350)
    verified_at: u64,
    academic_period: Symbol,
    verifier_address: Address,
}

FlowRateAdjustment {
    student: Address,
    base_rate: i128,
    adjusted_rate: i128,
    gpa_scaled: u64,
    adjustment_timestamp: u64,
    max_grant_amount: i128,
    total_distributed: i128,
}

BudgetTracker {
    student: Address,
    max_grant_amount: i128,
    total_distributed: i128,
    current_flow_rate: i128,
    last_accumulation_time: u64,
    accumulated_amount: i128,
}
```

### Constants & Configuration
```rust
// Study Group Constants
const STUDY_GROUP_SIZE: u64 = 3;
const VOTE_THRESHOLD: u64 = 2; // 2/3 vote required

// GPA-Based Flow Constants
const GPA_SCALE: u64 = 400; // 4.0 × 100
const MAX_GPA_BONUS: i128 = 200; // 20% bonus at 4.0 GPA
const MIN_GPA_THRESHOLD: u64 = 200; // 2.0 minimum GPA
const FLOW_RATE_PRECISION: u64 = 1000;
```

### Events
- `STUDY_GROUP_CREATED`: New study group formed
- `COLLATERAL_LOCKED`: Member locks collateral
- `MEMBER_PAUSED`: Member paused by vote
- `COLLATERAL_SLASHED`: Member collateral slashed
- `GPA_VERIFIED`: GPA verified by institution
- `GPA_FLOW_ADJUSTED`: Flow rate adjusted based on GPA
- `GPA_FLOW_UPDATED`: Dynamic GPA update

## Security & Logic Benefits

### For Study Groups
- **Accountability**: Financial consequences ensure participation
- **Fairness**: Democratic voting prevents abuse
- **Continuity**: Collateral funds replacement members
- **Transparency**: All actions recorded on-chain

### For GPA-Based Funding
- **Mathematical Safety**: Proven budget invariants prevent overspending
- **Academic Incentives**: Direct rewards for performance
- **Institutional Trust**: Verified GPA from authorized sources
- **Dynamic Adaptation**: Real-time rate adjustments

## Usage Examples

### Study Group Setup
```rust
// Create study group
let group_id = client.create_study_group(
    &funder,
    &members, // Vec<Address> of exactly 3
    &100, // 100 XLM collateral per member
    &1000, // Base flow rate
    &token_address
);

// Members lock collateral
for member in members {
    client.lock_collateral(&member, &group_id, &token_address);
}
```

### GPA-Based Stream
```rust
// Set academic oracle
client.set_academic_oracle(&admin, &university_address);

// Verify student GPA
client.verify_gpa(
    &student,
    &350, // 3.5 GPA
    &Symbol::new(&env, "fall2024"),
    &university_address
);

// Create GPA-adjusted stream
client.create_gpa_adjusted_stream(
    &funder,
    &student,
    &1000, // Base rate
    &50000, // Maximum grant amount
    &token_address
);
```

## Impact

### Study Group Impact
1. **Team Accountability**: Ensures all members contribute to shared projects
2. **Financial Protection**: Collateral safeguards against free-riding
3. **Democratic Governance**: Fair voting system for group management
4. **Project Continuity**: Replacement system maintains project momentum

### GPA-Flow Impact
1. **Academic Excellence**: Direct financial incentives for high performance
2. **Budget Safety**: Mathematical guarantees prevent overspending
3. **Donor Confidence**: Proven financial safety increases trust
4. **Institutional Integration**: Schools participate in funding verification

## Testing
Comprehensive test suite added covering:
- Study group creation and collateral locking
- Voting mechanisms (pause and slash)
- GPA verification and flow rate calculations
- Budget invariant verification
- Dynamic GPA updates
- Edge cases and error conditions

## Breaking Changes
No breaking changes - all new functionality is additive and maintains backward compatibility.

## Security Considerations
- **Collateral Security**: Funds held in contract until conditions met
- **Voting Security**: One vote per member, prevents double voting
- **GPA Verification**: Only authorized academic institutions can verify
- **Math Verification**: Invariant checks prevent overspending
- **Access Control**: Proper authorization for all sensitive operations
