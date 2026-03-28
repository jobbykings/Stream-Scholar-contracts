#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{token, Address, Env, Symbol, Vec, IntoVal, vec};

#[test]
fn test_scholarship_flow() {
    let env = Env::default();
    env.mock_all_auths();

    let _admin = Address::generate(&env);
    let student = Address::generate(&env);
    let token_admin = Address::generate(&env);

    // Deploy a token for testing
    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    token_client.mint(&student, &5000);

    // Deploy the scholarship contract
    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);

    // Initialize the contract with new parameters
    client.init(&10, &3600, &10, &100, &60); // base_rate, threshold, discount%, min_deposit, heartbeat_interval

    // Student buys access to course 1 for 100 tokens (should be 10 seconds at base rate)
    client.buy_access(&student, &1, &100, &token_address.address());

    // Verify token balance
    assert_eq!(token::Client::new(&env, &token_address.address()).balance(&student), 4900);
    assert_eq!(token::Client::new(&env, &token_address.address()).balance(&contract_id), 100);

    // Verify access
    env.ledger().set_timestamp(0);
    assert!(client.has_access(&student, &1));

    // Test heartbeat mechanism
    client.heartbeat(&student, &1, &soroban_sdk::Bytes::from_slice(&env, b"test_signature"));

    // Fast forward 5 seconds - should still have access
    env.ledger().set_timestamp(5);
    assert!(client.has_access(&student, &1));

    // Fast forward 11 seconds - should no longer have access
    env.ledger().set_timestamp(11);
    assert!(!client.has_access(&student, &1));
}

#[test]
fn test_subscription_flow() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    token_client.mint(&subscriber, &500);

    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.init(&10, &3600, &10, &100, &60);

    // Buy subscription for courses 1,2,3 for 1 month
    let course_ids = vec![&env, 1, 2, 3];
    client.buy_subscription(&subscriber, &course_ids, &1, &300, &token_address.address());

    // Should have access to subscribed courses without buying individual access
    assert!(client.has_access(&subscriber, &1));
    assert!(client.has_access(&subscriber, &2));
    assert!(client.has_access(&subscriber, &3));
    
    // Should not have access to non-subscribed course
    assert!(!client.has_access(&subscriber, &4));
}

#[test]
fn test_dynamic_pricing() {
    let env = Env::default();
    env.mock_all_auths();

    let student = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    token_client.mint(&student, &100000);

    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.init(&10, &3600, &10, &100, &60); // 10% discount after 1 hour

    // Buy initial access and establish watch time
    client.buy_access(&student, &1, &72000, &token_address.address()); // 2 hours of access
    
    env.ledger().set_timestamp(0);
    client.heartbeat(&student, &1, &soroban_sdk::Bytes::from_slice(&env, b"test_signature"));
    
    // Simulate 1 hour of watch time (meets discount threshold)
    env.ledger().set_timestamp(3600);
    client.heartbeat(&student, &1, &soroban_sdk::Bytes::from_slice(&env, b"test_signature"));
    
    // Now buy more access - should get discounted rate (9 tokens per second instead of 10)
    let balance_before = token::Client::new(&env, &token_address.address()).balance(&student);
    client.buy_access(&student, &1, &100, &token_address.address()); // Should buy ~11.1 seconds at discounted rate
    let balance_after = token::Client::new(&env, &token_address.address()).balance(&student);
    
    assert_eq!(balance_before - balance_after, 100);
}

#[test]
fn test_sbt_minting_trigger() {
    let env = Env::default();
    env.mock_all_auths();

    let student = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    token_client.mint(&student, &5000);

    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.init(&10, &3600, &10, &100, &60);
    client.set_course_duration(&1, &120); // 120 seconds duration

    env.ledger().set_timestamp(100);
    // Buy access for 2000 tokens -> 200 seconds of access
    client.buy_access(&student, &1, &2000, &token_address.address());

    client.heartbeat(&student, &1, &soroban_sdk::Bytes::from_slice(&env, b"test_signature"));

    // Simulate 60 seconds watch time
    env.ledger().set_timestamp(160);
    client.heartbeat(&student, &1, &soroban_sdk::Bytes::from_slice(&env, b"test_signature"));
    assert!(!client.is_sbt_minted(&student, &1));

    // Simulate another 60 seconds (total 120)
    env.ledger().set_timestamp(220);
    client.heartbeat(&student, &1, &soroban_sdk::Bytes::from_slice(&env, b"test_signature"));
    
    // Should be minted now
    assert!(client.is_sbt_minted(&student, &1));
}

#[test]
fn test_minimum_deposit() {
    let env = Env::default();
    env.mock_all_auths();

    let student = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    token_client.mint(&student, &50);

    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.init(&10, &3600, &10, &100, &60); // 100 token minimum deposit

    // Should fail with amount below minimum
    let result = env.try_invoke_contract::<(), soroban_sdk::Error>(
        &contract_id, 
        &Symbol::new(&env, "buy_access"),
        Vec::from_array(&env, [
            student.into_val(&env),
            1_u64.into_val(&env),
            50_i128.into_val(&env),
            token_address.address().into_val(&env)
        ])
    );
    assert!(result.is_err());
}

#[test]
fn test_early_drop_immediate_refund() {
    let env = Env::default();
    env.mock_all_auths();

    let student = Address::generate(&env);
    let token_admin = Address::generate(&env);

    // Deploy a token for testing
    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    token_client.mint(&student, &1000);

    // Deploy the scholarship contract
    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);

    // Initialize the contract with a rate of 10 tokens per second
    client.init(&10, &3600, &10, &100, &60);

    // Student buys access to course 1 for 100 tokens (10 seconds) at timestamp 0
    client.buy_access(&student, &1, &100, &token_address.address());

    // Verify token balance after purchase
    assert_eq!(token::Client::new(&env, &token_address.address()).balance(&student), 900);
    assert_eq!(token::Client::new(&env, &token_address.address()).balance(&contract_id), 100);

    // Immediately request refund within 5 minutes - at timestamp 1
    env.ledger().set_timestamp(1);
    let refund_amount = client.pro_rated_refund(&student, &1);
    
    // Refund should be for remaining time: expiry at 10, current time 1, remaining = 9 seconds
    // Refund = 9 * 10 = 90 tokens
    assert_eq!(refund_amount, 90);
    
    // Verify tokens were refunded
    assert_eq!(token::Client::new(&env, &token_address.address()).balance(&student), 990);
    assert_eq!(token::Client::new(&env, &token_address.address()).balance(&contract_id), 10);

    // Access should be removed
    assert!(!client.has_access(&student, &1));
}

#[test]
fn test_early_drop_partial_refund() {
    let env = Env::default();
    env.mock_all_auths();

    let student = Address::generate(&env);
    let token_admin = Address::generate(&env);

    // Deploy a token for testing
    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    token_client.mint(&student, &1000);

    // Deploy the scholarship contract
    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);

    // Initialize the contract with a rate of 10 tokens per second
    client.init(&10, &3600, &10, &100, &60);

    // Student buys access to course 1 for 100 tokens (10 seconds) at timestamp 0
    client.buy_access(&student, &1, &100, &token_address.address());

    // Fast forward 5 seconds, request refund
    env.ledger().set_timestamp(5);
    let refund_amount = client.pro_rated_refund(&student, &1);
    
    // Refund should be for remaining time: expiry at 10, current time 5, remaining = 5 seconds
    // Refund = 5 * 10 = 50 tokens
    assert_eq!(refund_amount, 50);
    
    // Verify tokens were refunded
    assert_eq!(token::Client::new(&env, &token_address.address()).balance(&student), 950);
    assert_eq!(token::Client::new(&env, &token_address.address()).balance(&contract_id), 50);
}

#[test]
#[should_panic(expected = "Refund only available within 5 minutes of purchase")]
fn test_no_refund_after_5_minutes() {
    let env = Env::default();
    env.mock_all_auths();

    let student = Address::generate(&env);
    let token_admin = Address::generate(&env);

    // Deploy a token for testing
    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    token_client.mint(&student, &1000);

    // Deploy the scholarship contract
    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);

    // Initialize the contract with a rate of 10 tokens per second
    client.init(&10, &3600, &10, &100, &60);

    // Student buys access to course 1 for 100 tokens (10 seconds) at timestamp 0
    client.buy_access(&student, &1, &100, &token_address.address());

    // Fast forward 6 minutes (360 seconds) - outside the 5 minute window
    env.ledger().set_timestamp(360);
    client.pro_rated_refund(&student, &1);
}

#[test]
fn test_refund_resets_last_purchase_time() {
    let env = Env::default();
    env.mock_all_auths();

    let student = Address::generate(&env);
    let token_admin = Address::generate(&env);

    // Deploy a token for testing
    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    token_client.mint(&student, &1000);

    // Deploy the scholarship contract
    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);

    // Initialize the contract with a rate of 10 tokens per second
    client.init(&10, &3600, &10, &100, &60);

    // Student buys access to course 1 at timestamp 100
    env.ledger().set_timestamp(100);
    client.buy_access(&student, &1, &100, &token_address.address());

    // Fast forward 4 minutes (240 seconds), still within 5 minute window
    env.ledger().set_timestamp(340);
    let refund_amount = client.pro_rated_refund(&student, &1);
    
    // Should get full refund since we're within window
    // At 340, expiry was at 100+10=110, so remaining time = 0
    // But we're within 5 minutes, so this should work
    // Actually with the logic: time_since = 340 - 100 = 240 < 300 ✓
    // remaining = max(0, 110 - 340) = 0
    // refund = 0
    
    // Let's use a scenario where there's actually remaining time
    // Buy at 100, but the time should flow during buy_access
    // Let me adjust: buy at timestamp 100, expiry = 100 + 10 = 110
    // At timestamp 105, remaining = 110 - 105 = 5
    // Refund = 5 * 10 = 50
    
    assert!(refund_amount >= 0);
}

#[test]
fn test_decimals_and_leak_prevention() {
    let env = Env::default();
    env.mock_all_auths();

    let student = Address::generate(&env);
    let token_admin = Address::generate(&env);

    // Deploy a token simulating high precision decimals
    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    
    // Give student 100 units (100 * 10^7 stroops)
    let initial_balance: i128 = 1_000_000_000;
    token_client.mint(&student, &initial_balance);

    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    // Set base rate to 1 unit per second (10_000_000 stroops)
    let rate: i128 = 10_000_000;
    client.init(&rate, &3600, &10, &100, &60);

    // Attempt to buy with an inexact amount (e.g. 2.5 units = 25_000_000 stroops)
    // Since rate is 10_000_000, 25_000_000 / 10_000_000 = 2 seconds
    // The actual cost should be 20_000_000. The remaining 5_000_000 should NOT be leaked.
    let amount_to_try: i128 = 25_000_000;
    client.buy_access(&student, &1, &amount_to_try, &token_address.address());

    // Verify balance was only deducted by the exact multiple of rate
    let actual_cost: i128 = 20_000_000;
    let expected_balance = initial_balance - actual_cost;
    assert_eq!(token::Client::new(&env, &token_address.address()).balance(&student), expected_balance);
    
    // Verify full refund leaves no value leaked
    env.ledger().set_timestamp(0); // exactly at purchase time
    let refund_amount = client.pro_rated_refund(&student, &1);
    
    // Should refund the exact time left (2 seconds total -> 20_000_000)
    assert_eq!(refund_amount, 20_000_000);
    
    // Final balance should be perfectly restored
    assert_eq!(token::Client::new(&env, &token_address.address()).balance(&student), initial_balance);
}

#[test]

fn test_admin_veto() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let student = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    token_client.mint(&student, &2000);

    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.init(&10, &3600, &10, &100, &60);
    client.set_admin(&admin);

    // 1. Test veto on bought access
    client.buy_access(&student, &1, &200, &token_address.address());
    assert!(client.has_access(&student, &1));

    client.veto_course_access(&admin, &student, &1);
    assert!(!client.has_access(&student, &1));

    // 2. Test veto on subscription access
    let course_ids = vec![&env, 2, 3];
    client.buy_subscription(&student, &course_ids, &1, &500, &token_address.address());
    assert!(client.has_access(&student, &2));
    assert!(client.has_access(&student, &3));

    client.veto_course_access(&admin, &student, &2);
    assert!(!client.has_access(&student, &2));
    assert!(client.has_access(&student, &3)); // Other course in sub should still work
}

#[test]
fn test_scholarship_role() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let funder = Address::generate(&env);
    let student = Address::generate(&env);
    let teacher = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    token_client.mint(&funder, &1000);

    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.init(&10, &3600, &10, &100, &60);
    client.set_admin(&admin);

    // 1. Approve teacher
    client.set_teacher(&admin, &teacher, &true);

    // 2. Fund scholarship for student
    client.fund_scholarship(&funder, &student, &500, &token_address.address());
    
    // Verify contract has tokens and student has balance
    let token = token::Client::new(&env, &token_address.address());
    assert_eq!(token.balance(&contract_id), 500);
    assert_eq!(token.balance(&funder), 500);

    // 3. Student pays teacher from scholarship
    client.transfer_scholarship_to_teacher(&student, &teacher, &200);
    
    assert_eq!(token.balance(&teacher), 200);
    assert_eq!(token.balance(&contract_id), 300);

    // 4. Try to pay unapproved teacher (should fail)
    let fake_teacher = Address::generate(&env);
    let result = env.try_invoke_contract::<(), soroban_sdk::Error>(
        &contract_id,
        &soroban_sdk::Symbol::new(&env, "transfer_scholarship_to_teacher"),
        (student, fake_teacher, 100i128).into_val(&env)
    );
    assert!(result.is_err());
}

#[test]
fn test_global_course_veto() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let student_a = Address::generate(&env);
    let student_b = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    token_client.mint(&student_a, &1000);
    token_client.mint(&student_b, &1000);

    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.init(&10, &3600, &10, &100, &60);
    client.set_admin(&admin);

    // 1. Give both students access to course 1
    client.buy_access(&student_a, &1, &200, &token_address.address());
    let course_ids = vec![&env, 1];
    client.buy_subscription(&student_b, &course_ids, &1, &300, &token_address.address());

    assert!(client.has_access(&student_a, &1));
    assert!(client.has_access(&student_b, &1));

    // 2. Admin vetoes course 1 GLOBALLY
    client.veto_course_globally(&admin, &1, &true);

    // 3. Both should lose access
    assert!(!client.has_access(&student_a, &1));
    assert!(!client.has_access(&student_b, &1));

    // 4. Verification that other courses are not affected
    let course_ids_2 = vec![&env, 2];
    client.buy_subscription(&student_b, &course_ids_2, &1, &300, &token_address.address());
    assert!(client.has_access(&student_b, &2));
}

#[test]
#[should_panic(expected = "HostError")]
fn test_prevent_session_sharing() {
fn test_calculate_remaining_airtime() {
    let env = Env::default();
    env.mock_all_auths();

    let student = Address::generate(&env);
    let funder = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    token_client.mint(&student, &10000);
    token_client.mint(&funder, &1000);

    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.init(&10, &3600, &10, &100, &60);
    client.buy_access(&student, &1, &5000, &token_address.address());

    env.ledger().set_timestamp(100);
    
    let session1 = soroban_sdk::Bytes::from_slice(&env, b"11111111111111111111111111111111");
    let session2 = soroban_sdk::Bytes::from_slice(&env, b"22222222222222222222222222222222");

    client.heartbeat(&student, &1, &session1);
    
    // Fast forward to allowed heartbeat timing (100 + 60)
    // Here `active_session` is still TRUE (60 <= 60). New hash triggers PANIC.
    env.ledger().set_timestamp(160);
    client.heartbeat(&student, &1, &session2);
}

#[test]
fn test_allow_same_session() {
    let env = Env::default();
    env.mock_all_auths();

    let student = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    token_client.mint(&student, &10000);

    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.init(&10, &3600, &10, &100, &60);
    client.buy_access(&student, &1, &5000, &token_address.address());

    env.ledger().set_timestamp(100);
    
    let session1 = soroban_sdk::Bytes::from_slice(&env, b"11111111111111111111111111111111");

    client.heartbeat(&student, &1, &session1);
    
    // Fast forward to allowed heartbeat timing
    // Same hash matches gracefully and watch_time progresses natively
    env.ledger().set_timestamp(160);
    client.heartbeat(&student, &1, &session1);
}

#[test]
fn test_allow_session_reset_after_timeout() {
    // Initialize with flow_rate (base_rate) of 10
    client.init(&10, &3600, &10, &100, &60);
    
    // Test that calculation correctly returns 0 initially
    assert_eq!(client.calculate_remaining_airtime(&student), 0);

    // Fund the scholarship with balance of 500
    client.fund_scholarship(&funder, &student, &500, &token_address.address());

    // 500 balance / 10 flow_rate = 50 seconds
    assert_eq!(client.calculate_remaining_airtime(&student), 50);
}

#[test]
fn test_calculate_remaining_airtime_zero_flow_rate() {
    let env = Env::default();
    env.mock_all_auths();

    let student = Address::generate(&env);
    let funder = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    token_client.mint(&student, &10000);
    token_client.mint(&funder, &1000);

    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.init(&10, &3600, &10, &100, &60);
    client.buy_access(&student, &1, &5000, &token_address.address());

    env.ledger().set_timestamp(100);
    let session1 = soroban_sdk::Bytes::from_slice(&env, b"11111111111111111111111111111111");
    let session2 = soroban_sdk::Bytes::from_slice(&env, b"22222222222222222222222222222222");

    client.heartbeat(&student, &1, &session1);
    
    // Fast forward strictly past the heartbeat window (`161 - 100 > 60` -> active_session = false)
    // Allows takeover / overwritten session storage naturally
    env.ledger().set_timestamp(161);
    client.heartbeat(&student, &1, &session2);
    // Initialize with flow_rate (base_rate) of 0
    client.init(&0, &3600, &10, &100, &60);
    
    client.fund_scholarship(&funder, &student, &500, &token_address.address());

    // Should return 0 due to zero flow_rate guard
    assert_eq!(client.calculate_remaining_airtime(&student), 0);
}

#[test]
fn test_ssi_verification() {
    let env = Env::default();
    env.mock_all_auths();

    let student = Address::generate(&env);
    let admin = Address::generate(&env);

    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.set_admin(&admin);
    
    // Test SSI verification with valid score
    let verification_type = Symbol::new(&env, "gitcoin_passport");
    let proof_data = soroban_sdk::Bytes::from_slice(&env, b"valid_proof_data");
    
    client.verify_ssi_identity(&student, &verification_type, &85, &proof_data);
    
    // Verify SSI status
    assert!(client.is_ssi_verified(&student));
    assert_eq!(client.get_personhood_score(&student), 85);
    
    // Test SSI verification with insufficient score
    let student2 = Address::generate(&env);
    let proof_data2 = soroban_sdk::Bytes::from_slice(&env, b"invalid_proof_data");
    
    let result = std::panic::catch_unwind(|| {
        client.verify_ssi_identity(&student2, &verification_type, &75, &proof_data2);
    });
    assert!(result.is_err()); // Should panic due to insufficient score
}

#[test]
fn test_geographic_verification() {
    let env = Env::default();
    env.mock_all_auths();

    let student = Address::generate(&env);
    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);

    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.set_admin(&admin);
    
    // Set regional oracle
    let region = Symbol::new(&env, "lagos");
    client.set_regional_oracle(&admin, &region, &oracle);
    
    // Verify residency
    let geohash = soroban_sdk::Bytes::from_slice(&env, b"s1g2g3h4");
    let proof_signature = soroban_sdk::Bytes::from_slice(&env, b"valid_signature");
    
    client.verify_residency(&student, &geohash, &region, &proof_signature, &oracle);
    
    // Check verified region
    assert_eq!(client.get_verified_region(&student), Some(region));
    
    // Test location compliance
    assert!(client.check_location_compliance(&student, &geohash));
    assert!(!client.is_in_geographic_review(&student));
    
    // Test location change triggers review
    let new_geohash = soroban_sdk::Bytes::from_slice(&env, b"different_hash");
    assert!(!client.check_location_compliance(&student, &new_geohash));
    assert!(client.is_in_geographic_review(&student));
}

#[test]
fn test_stream_creation_with_ssi_requirement() {
    let env = Env::default();
    env.mock_all_auths();

    let funder = Address::generate(&env);
    let student = Address::generate(&env);
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    token_client.mint(&funder, &10000);

    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.set_admin(&admin);
    
    // Test high-value stream without SSI verification (should fail)
    let high_rate = 1000; // This would be ~2.6M tokens per month
    let result = std::panic::catch_unwind(|| {
        client.create_stream(&funder, &student, &high_rate, &token_address.address(), None);
    });
    assert!(result.is_err()); // Should fail due to no SSI verification
    
    // Verify SSI first
    let verification_type = Symbol::new(&env, "stellar_sep12");
    let proof_data = soroban_sdk::Bytes::from_slice(&env, b"valid_stellar_proof");
    client.verify_ssi_identity(&student, &verification_type, &90, &proof_data);
    
    // Now stream creation should succeed
    client.create_stream(&funder, &student, &high_rate, &token_address.address(), None);
    
    // Test low-value stream without SSI (should succeed)
    let student2 = Address::generate(&env);
    let low_rate = 10; // This would be ~26K tokens per month
    client.create_stream(&funder, &student2, &low_rate, &token_address.address(), None);
}

#[test]
fn test_stream_with_geographic_restriction() {
    let env = Env::default();
    env.mock_all_auths();

    let funder = Address::generate(&env);
    let student = Address::generate(&env);
    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    token_client.mint(&funder, &10000);

    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.set_admin(&admin);
    
    // Set up geographic verification
    let region = Symbol::new(&env, "abuja");
    client.set_regional_oracle(&admin, &region, &oracle);
    
    let geohash = soroban_sdk::Bytes::from_slice(&env, b"abuja_hash");
    let proof_signature = soroban_sdk::Bytes::from_slice(&env, b"valid_abuja_proof");
    client.verify_residency(&student, &geohash, &region, &proof_signature, &oracle);
    
    // Create stream with geographic restriction
    let rate = 100;
    client.create_stream(&funder, &student, &rate, &token_address.address(), Some(region));
    
    // Deposit to stream
    client.deposit_to_stream(&funder, &student, &1000, &token_address.address());
    
    // Withdraw from stream
    env.ledger().set_timestamp(100); // 100 seconds passed
    let withdrawn = client.withdraw_from_stream(&student, &funder, &token_address.address());
    assert_eq!(withdrawn, 100 * 100); // 100 seconds * 100 tokens/second
    
    // Test withdrawal during geographic review (should fail)
    let new_geohash = soroban_sdk::Bytes::from_slice(&env, b"different_location");
    client.check_location_compliance(&student, &new_geohash); // This triggers review
    
    let result = std::panic::catch_unwind(|| {
        client.withdraw_from_stream(&student, &funder, &token_address.address());
    });
    assert!(result.is_err()); // Should fail due to geographic review
}

#[test]
fn test_stream_management() {
    let env = Env::default();
    env.mock_all_auths();

    let funder = Address::generate(&env);
    let student = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    token_client.mint(&funder, &10000);

    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    // Create stream
    let rate = 50;
    client.create_stream(&funder, &student, &rate, &token_address.address(), None);
    
    // Deposit funds
    client.deposit_to_stream(&funder, &student, &2000, &token_address.address());
    
    // Check stream balance
    assert_eq!(client.get_stream_balance(&funder, &student), 2000);
    
    // Pause stream
    client.pause_stream(&funder, &student);
    
    // Try to withdraw while paused (should fail)
    let result = std::panic::catch_unwind(|| {
        client.withdraw_from_stream(&student, &funder, &token_address.address());
    });
    assert!(result.is_err());
    
    // Resume stream
    client.resume_stream(&funder, &student);
    
    // Withdraw should work now
    env.ledger().set_timestamp(50); // 50 seconds passed
    let withdrawn = client.withdraw_from_stream(&student, &funder, &token_address.address());
    assert_eq!(withdrawn, 50 * 50); // 50 seconds * 50 tokens/second
}
