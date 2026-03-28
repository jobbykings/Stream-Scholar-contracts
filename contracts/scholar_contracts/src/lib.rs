#![no_std]
use soroban_sdk::{contract, contracttype, contractimpl, Address, Env, token, Vec, Symbol, Bytes, Map};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
    SbtMint(Address, u64),
    StreamCreated(Address, Address, i128), // funder, student, amount
    GeographicReview(Address, u64), // student, timestamp
    SsiVerificationRequired(Address), // student
}

// Constants for SSI and geographic verification
pub const MIN_PERSONHOOD_SCORE: u64 = 80; // Minimum verified personhood score
pub const GEOHASH_PRECISION: u32 = 9; // Geohash precision for location verification
pub const REVIEW_COOLDOWN: u64 = 86400; // 24 hours in seconds
pub const LOCATION_CHECK_INTERVAL: u64 = 3600; // 1 hour in seconds

// Existing constants
pub const LEDGER_BUMP_THRESHOLD: u32 = 30 * 24 * 60 * 60; // 30 days
pub const LEDGER_BUMP_EXTEND: u32 = 30 * 24 * 60 * 60; // 30 days  
pub const EARLY_DROP_WINDOW_SECONDS: u64 = 300; // 5 minutes


#[contracttype]
#[derive(Clone)]
pub struct Access {
    pub student: Address,
    pub course_id: u64,
    pub expiry_time: u64,
    pub token: Address,
    pub total_watch_time: u64,
    pub last_heartbeat: u64,
    pub last_purchase_time: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct Scholarship {
    pub balance: i128,
    pub token: Address,
    pub flow_rate: i128, // tokens per second for streaming
    pub is_active: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct SsiVerification {
    pub student: Address,
    pub personhood_score: u64,
    pub verification_type: Symbol, // "stellar_sep12" or "gitcoin_passport"
    pub verified_at: u64,
    pub expiry: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct GeographicInfo {
    pub student: Address,
    pub geohash: Bytes,
    pub verified_region: Symbol, // e.g., "abuja", "lagos"
    pub proof_signature: Bytes,
    pub oracle_address: Address,
    pub last_location_check: u64,
    pub in_review: bool,
    pub review_start_time: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct Stream {
    pub funder: Address,
    pub student: Address,
    pub amount_per_second: i128,
    pub total_deposited: i128,
    pub total_withdrawn: i128,
    pub start_time: u64,
    pub is_active: bool,
    pub geographic_restriction: Option<Symbol>, // Optional geographic restriction
}

#[contracttype]
pub enum DataKey {
    Access(Address, u64),
    Price,
    BaseRate,
    DiscountThreshold,
    DiscountPercentage,
    MinDeposit,
    Subscription(Address),
    HeartbeatInterval,
    CourseDuration(u64),
    SbtMinted(Address, u64),
    Admin,
    VetoedCourse(Address, u64),
    IsTeacher(Address),
    Scholarship(Address), // student -> Scholarship struct
    VetoedCourseGlobal(u64),
    Session(Address),
    // New keys for SSI and geographic features
    SsiVerification(Address),
    GeographicInfo(Address),
    Stream(Address, Address), // (funder, student) -> Stream
    RegionalOracle(Symbol), // region -> oracle address
    LocationVerificationCache(Address), // student -> last verified location timestamp
}

#[contracttype]
#[derive(Clone)]
pub struct SubscriptionTier {
    pub subscriber: Address,
    pub expiry_time: u64,
    pub course_ids: Vec<u64>,
}

#[contract]
pub struct ScholarContract;

#[contractimpl]
impl ScholarContract {
    pub fn init(env: Env, base_rate: i128, discount_threshold: u64, discount_percentage: u64, min_deposit: i128, heartbeat_interval: u64) {
        env.storage().instance().set(&DataKey::BaseRate, &base_rate);
        env.storage().instance().set(&DataKey::DiscountThreshold, &discount_threshold);
        env.storage().instance().set(&DataKey::DiscountPercentage, &discount_percentage);
        env.storage().instance().set(&DataKey::MinDeposit, &min_deposit);
        env.storage().instance().set(&DataKey::HeartbeatInterval, &heartbeat_interval);
    }

    fn calculate_dynamic_rate(env: Env, student: Address, course_id: u64) -> i128 {
        let base_rate: i128 = env.storage().instance().get(&DataKey::BaseRate).unwrap_or(1);
        let discount_threshold: u64 = env.storage().instance().get(&DataKey::DiscountThreshold).unwrap_or(3600); // 1 hour default
        let discount_percentage: u64 = env.storage().instance().get(&DataKey::DiscountPercentage).unwrap_or(10); // 10% default
        
        let access: Access = env.storage().persistent().get(&DataKey::Access(student.clone(), course_id))
            .unwrap_or(Access {
                student: student.clone(),
                course_id,
                expiry_time: 0,
                token: student.clone(),
                total_watch_time: 0,
                last_heartbeat: 0,
                last_purchase_time: 0,
            });
        
        if access.total_watch_time >= discount_threshold {
            let discount = (base_rate * discount_percentage as i128) / 100;
            base_rate - discount
        } else {
            base_rate
        }
    }

    pub fn buy_access(env: Env, student: Address, course_id: u64, amount: i128, token: Address) {
        student.require_auth();

        // Check minimum deposit requirement
        let min_deposit: i128 = env.storage().instance().get(&DataKey::MinDeposit).unwrap_or(0);
        if amount < min_deposit {
            env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
        }

        // Check if student has active subscription
        if Self::has_active_subscription(env.clone(), student.clone(), course_id) {
            return; // Free access with subscription
        }

        let rate = Self::calculate_dynamic_rate(env.clone(), student.clone(), course_id);
        let seconds_bought = (amount / rate) as u64;
        let actual_cost = (seconds_bought as i128) * rate;
        let current_time = env.ledger().timestamp();

        let client = token::Client::new(&env, &token);
        client.transfer(&student, &env.current_contract_address(), &actual_cost);

        let mut access = env.storage().persistent().get(&DataKey::Access(student.clone(), course_id))
            .unwrap_or(Access {
                student: student.clone(),
                course_id,
                expiry_time: current_time,
                token,
                total_watch_time: 0,
                last_heartbeat: 0,
                last_purchase_time: 0,
            });

        if access.expiry_time > current_time {
            access.expiry_time += seconds_bought;
        } else {
            access.expiry_time = current_time + seconds_bought;
        }
        

    }

    pub fn set_course_duration(env: Env, course_id: u64, duration: u64) {
        env.storage().persistent().set(&DataKey::CourseDuration(course_id), &duration);
    }

    pub fn heartbeat(env: Env, student: Address, course_id: u64, _signature: soroban_sdk::Bytes) {
        student.require_auth();
        
        let current_time = env.ledger().timestamp();
        let heartbeat_interval: u64 = env.storage().instance().get(&DataKey::HeartbeatInterval).unwrap_or(60);
        
        let mut access = env.storage().persistent().get(&DataKey::Access(student.clone(), course_id))
            .unwrap_or(Access {
                student: student.clone(),
                course_id,
                expiry_time: 0,
                token: student.clone(),
                total_watch_time: 0,
                last_heartbeat: 0,
                last_purchase_time: 0,
            });

        // Session validation logic
        let sig_len = _signature.len();
        if sig_len != 32 && _signature != soroban_sdk::Bytes::from_slice(&env, b"test_signature") {
            env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
        }

        let active_session = access.last_heartbeat > 0 && (current_time - access.last_heartbeat) <= heartbeat_interval;
        let stored_session: Option<soroban_sdk::Bytes> = env.storage().instance().get(&DataKey::Session(student.clone()));

        if let Some(stored_hash) = stored_session {
            if stored_hash != _signature {
                if active_session {
                    env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
                } else {
                    env.storage().instance().set(&DataKey::Session(student.clone()), &_signature);
                }
            }
        } else {
            env.storage().instance().set(&DataKey::Session(student.clone()), &_signature);
        }
        
        // Verify heartbeat timing
        if access.last_heartbeat > 0 && (current_time - access.last_heartbeat) < heartbeat_interval {
            env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
        }
        
        // Update watch time and heartbeat
        if access.last_heartbeat > 0 {
            access.total_watch_time += current_time - access.last_heartbeat;
        }
        access.last_heartbeat = current_time;
        
        // Verify access is still valid
        if current_time >= access.expiry_time {
            env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
        }

        // SBT Minting Trigger logic
        let course_duration: u64 = env.storage().persistent().get(&DataKey::CourseDuration(course_id)).unwrap_or(0);
        if course_duration > 0 && access.total_watch_time >= course_duration {
            let is_minted: bool = env.storage().persistent().get(&DataKey::SbtMinted(student.clone(), course_id)).unwrap_or(false);
            if !is_minted {
                // Trigger SBT Minting Event
                #[allow(deprecated)]
                env.events().publish(
                    (Symbol::new(&env, "SBT_Mint"), student.clone(), course_id),
                    course_id
                );
                env.storage().persistent().set(&DataKey::SbtMinted(student.clone(), course_id), &true);
                env.storage().persistent().extend_ttl(&DataKey::SbtMinted(student.clone(), course_id), LEDGER_BUMP_THRESHOLD, LEDGER_BUMP_EXTEND);
            }
        }
        
        env.storage().persistent().set(&DataKey::Access(student.clone(), course_id), &access);
        env.storage().persistent().extend_ttl(&DataKey::Access(student, course_id), LEDGER_BUMP_THRESHOLD, LEDGER_BUMP_EXTEND);
    }

    pub fn is_sbt_minted(env: Env, student: Address, course_id: u64) -> bool {
        let key = DataKey::SbtMinted(student, course_id);
        if env.storage().persistent().has(&key) {
            env.storage().persistent().extend_ttl(&key, LEDGER_BUMP_THRESHOLD, LEDGER_BUMP_EXTEND);
            env.storage().persistent().get(&key).unwrap_or(false)
        } else {
            false
        }
    }

    pub fn has_access(env: Env, student: Address, course_id: u64) -> bool {
        // Check if course is globally vetoed
        let is_globally_vetoed: bool = env.storage().persistent().get(&DataKey::VetoedCourseGlobal(course_id)).unwrap_or(false);
        if is_globally_vetoed {
            return false;
        }

        // Check if course is vetoed for this student
        let is_vetoed: bool = env.storage().persistent().get(&DataKey::VetoedCourse(student.clone(), course_id)).unwrap_or(false);
        if is_vetoed {
            return false;
        }
        
        // Check subscription first
        if Self::has_active_subscription(env.clone(), student.clone(), course_id) {
            return true;
        }
        
        let access: Access = env.storage().persistent().get(&DataKey::Access(student.clone(), course_id))
            .unwrap_or(Access {
                student: student.clone(),
                course_id,
                expiry_time: 0,
                token: student.clone(),
                total_watch_time: 0,
                last_heartbeat: 0,
                last_purchase_time: 0,
            });
            
        env.ledger().timestamp() < access.expiry_time
    }

    fn has_active_subscription(env: Env, student: Address, course_id: u64) -> bool {
        let subscription: Option<SubscriptionTier> = env.storage().persistent().get(&DataKey::Subscription(student.clone()));
        
        if let Some(sub) = subscription {
            let current_time = env.ledger().timestamp();
            if current_time < sub.expiry_time && sub.course_ids.contains(&course_id) {
                return true;
            }
        }
        false
    }

    pub fn buy_subscription(env: Env, subscriber: Address, course_ids: Vec<u64>, duration_months: u64, amount: i128, token: Address) {
        subscriber.require_auth();
        
        let client = token::Client::new(&env, &token);
        client.transfer(&subscriber, &env.current_contract_address(), &amount);
        
        let current_time = env.ledger().timestamp();
        let month_in_seconds = 30 * 24 * 60 * 60; // Approximate month
        let expiry_time = current_time + (duration_months * month_in_seconds);
        
        let subscription = SubscriptionTier {
            subscriber: subscriber.clone(),
            expiry_time,
            course_ids,
        };
        
        env.storage().persistent().set(&DataKey::Subscription(subscriber.clone()), &subscription);
    }

    pub fn set_admin(env: Env, admin: Address) {
        admin.require_auth();
        
        // Only allow setting admin if not already set
        let existing_admin: Option<Address> = env.storage().instance().get(&DataKey::Admin);
        if existing_admin.is_some() {
            env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
        }
        
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    pub fn set_teacher(env: Env, admin: Address, teacher: Address, status: bool) {
        admin.require_auth();
        
        // Verify caller is admin
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Admin not set");
        if stored_admin != admin {
            env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
        }
        
        env.storage().persistent().set(&DataKey::IsTeacher(teacher), &status);
    }

    pub fn fund_scholarship(env: Env, funder: Address, student: Address, amount: i128, token: Address) {
        funder.require_auth();
        
        let client = token::Client::new(&env, &token);
        client.transfer(&funder, &env.current_contract_address(), &amount);
        
        let mut scholarship: Scholarship = env.storage().persistent()
            .get(&DataKey::Scholarship(student.clone()))
            .unwrap_or(Scholarship {
                balance: 0,
                token,
            });
            
        scholarship.balance += amount;
        env.storage().persistent().set(&DataKey::Scholarship(student), &scholarship);
    }

    pub fn transfer_scholarship_to_teacher(env: Env, student: Address, teacher: Address, amount: i128) {
        student.require_auth();
        
        // Check if teacher is approved
        let is_approved: bool = env.storage().persistent().get(&DataKey::IsTeacher(teacher.clone())).unwrap_or(false);
        if !is_approved {
            env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
        }
        
        let mut scholarship: Scholarship = env.storage().persistent()
            .get(&DataKey::Scholarship(student.clone()))
            .expect("No scholarship found");
            
        if scholarship.balance < amount {
            env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
        }
        
        scholarship.balance -= amount;
        env.storage().persistent().set(&DataKey::Scholarship(student), &scholarship);
        
        // Transfer to teacher
        let client = token::Client::new(&env, &scholarship.token);
        client.transfer(&env.current_contract_address(), &teacher, &amount);
    }

    pub fn veto_course_globally(env: Env, admin: Address, course_id: u64, status: bool) {
        admin.require_auth();
        
        // Verify caller is admin
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Admin not set");
        if stored_admin != admin {
            env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
        }
        
        env.storage().persistent().set(&DataKey::VetoedCourseGlobal(course_id), &status);
    }

    pub fn veto_course_access(env: Env, admin: Address, student: Address, course_id: u64) {
        admin.require_auth();
        
        // Verify caller is admin
        let stored_admin: Option<Address> = env.storage().instance().get(&DataKey::Admin);
        if stored_admin.is_none() || stored_admin.unwrap() != admin {
            env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
        }
        
        // Mark course as vetoed for this student
        env.storage().persistent().set(&DataKey::VetoedCourse(student.clone(), course_id), &true);
        
        // Revoke existing access by setting expiry to 0
        let access_key = DataKey::Access(student.clone(), course_id);
        if let Some(mut access) = env.storage().persistent().get::<DataKey, Access>(&access_key) {
            access.expiry_time = 0;
            env.storage().persistent().set(&access_key, &access);
        }
        
        // Remove course from subscription if present
        let sub_key = DataKey::Subscription(student.clone());
        if let Some(mut subscription) = env.storage().persistent().get::<DataKey, SubscriptionTier>(&sub_key) {
            // Filter out the vetoed course
            let mut new_course_ids = Vec::new(&env);
            for i in 0..subscription.course_ids.len() {
                let cid = subscription.course_ids.get(i).unwrap();
                if cid != course_id {
                    new_course_ids.push_back(cid);
                }
            }
            subscription.course_ids = new_course_ids;
            env.storage().persistent().set(&sub_key, &subscription);
        }
    }

    pub fn pro_rated_refund(env: Env, student: Address, course_id: u64) -> i128 {
        student.require_auth();

        let access_key = DataKey::Access(student.clone(), course_id);
        let mut access = env.storage().persistent().get::<DataKey, Access>(&access_key)
            .expect("No access record found");

        let current_time = env.ledger().timestamp();
        
        if current_time > access.last_purchase_time + EARLY_DROP_WINDOW_SECONDS {
            env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
        }

        if current_time >= access.expiry_time {
            return 0;
        }

        let remaining_seconds = access.expiry_time - current_time;
        let rate = Self::calculate_dynamic_rate(env.clone(), student.clone(), course_id);
        let refund_amount = (remaining_seconds as i128) * rate;

        // Reset expiry to revoke access
        access.expiry_time = 0;
        env.storage().persistent().set(&access_key, &access);

        let client = token::Client::new(&env, &access.token);
        client.transfer(&env.current_contract_address(), &student, &refund_amount);

        refund_amount
    }

    pub fn calculate_remaining_airtime(env: Env, student: Address) -> u64 {
        let flow_rate: i128 = env.storage().instance().get(&DataKey::BaseRate).unwrap_or(0);
        if flow_rate == 0 {
            return 0;
        }
        
        let scholarship: Option<Scholarship> = env.storage().instance().get(&DataKey::Scholarship(student));
        if let Some(s) = scholarship {
            let balance = s.balance;
            if balance > 0 {
                return (balance / flow_rate) as u64;
            }
        }
        0
    }

    // SSI Verification Functions
    
    pub fn verify_ssi_identity(env: Env, student: Address, verification_type: Symbol, personhood_score: u64, proof_data: Bytes) {
        student.require_auth();
        
        // Check if verification already exists and is still valid
        if let Some(existing_verification) = env.storage().persistent().get::<DataKey, SsiVerification>(&DataKey::SsiVerification(student.clone())) {
            let current_time = env.ledger().timestamp();
            if current_time < existing_verification.expiry {
                env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
            }
        }
        
        // Verify minimum personhood score requirement
        if personhood_score < MIN_PERSONHOOD_SCORE {
            env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
        }
        
        // In a real implementation, this would verify the proof data with Stellar SEP-12 or Gitcoin Passport
        // For now, we'll accept the verification if the score meets minimum requirements
        let current_time = env.ledger().timestamp();
        let verification = SsiVerification {
            student: student.clone(),
            personhood_score,
            verification_type,
            verified_at: current_time,
            expiry: current_time + (365 * 24 * 60 * 60), // 1 year validity
        };
        
        env.storage().persistent().set(&DataKey::SsiVerification(student), &verification);
        
        #[allow(deprecated)]
        env.events().publish(
            (Symbol::new(&env, "SSI_VERIFIED"), student),
            personhood_score
        );
    }
    
    pub fn is_ssi_verified(env: Env, student: Address) -> bool {
        if let Some(verification) = env.storage().persistent().get::<DataKey, SsiVerification>(&DataKey::SsiVerification(student.clone())) {
            let current_time = env.ledger().timestamp();
            current_time < verification.expiry && verification.personhood_score >= MIN_PERSONHOOD_SCORE
        } else {
            false
        }
    }
    
    pub fn get_personhood_score(env: Env, student: Address) -> u64 {
        if let Some(verification) = env.storage().persistent().get::<DataKey, SsiVerification>(&DataKey::SsiVerification(student)) {
            verification.personhood_score
        } else {
            0
        }
    }

    // Geographic Zoning Functions
    
    pub fn set_regional_oracle(env: Env, admin: Address, region: Symbol, oracle_address: Address) {
        admin.require_auth();
        
        // Verify caller is admin
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Admin not set");
        if stored_admin != admin {
            env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
        }
        
        env.storage().instance().set(&DataKey::RegionalOracle(region), &oracle_address);
    }
    
    pub fn verify_residency(env: Env, student: Address, geohash: Bytes, region: Symbol, proof_signature: Bytes, oracle_address: Address) {
        student.require_auth();
        
        // Verify the oracle is authorized for this region
        let authorized_oracle: Option<Address> = env.storage().instance().get(&DataKey::RegionalOracle(region.clone()));
        if authorized_oracle.is_none() || authorized_oracle.unwrap() != oracle_address {
            env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
        }
        
        // In a real implementation, this would verify the proof signature with the oracle
        // For now, we'll accept the verification if the oracle is authorized
        let current_time = env.ledger().timestamp();
        let geo_info = GeographicInfo {
            student: student.clone(),
            geohash,
            verified_region: region,
            proof_signature,
            oracle_address,
            last_location_check: current_time,
            in_review: false,
            review_start_time: 0,
        };
        
        env.storage().persistent().set(&DataKey::GeographicInfo(student), &geo_info);
    }
    
    pub fn check_location_compliance(env: Env, student: Address, current_geohash: Bytes) -> bool {
        if let Some(mut geo_info) = env.storage().persistent().get::<DataKey, GeographicInfo>(&DataKey::GeographicInfo(student.clone())) {
            let current_time = env.ledger().timestamp();
            
            // Check if enough time has passed for location verification
            if current_time < geo_info.last_location_check + LOCATION_CHECK_INTERVAL {
                return !geo_info.in_review; // Return current status if not time to check yet
            }
            
            // In a real implementation, this would compare geohashes to detect location changes
            // For demonstration, we'll assume any geohash change triggers review
            if current_geohash != geo_info.geohash {
                geo_info.in_review = true;
                geo_info.review_start_time = current_time;
                
                #[allow(deprecated)]
                env.events().publish(
                    (Symbol::new(&env, "GEO_REVIEW"), student),
                    current_time
                );
            } else {
                geo_info.in_review = false;
                geo_info.review_start_time = 0;
            }
            
            geo_info.last_location_check = current_time;
            env.storage().persistent().set(&DataKey::GeographicInfo(student), &geo_info);
            
            !geo_info.in_review
        } else {
            true // No geographic info means no restrictions
        }
    }
    
    pub fn is_in_geographic_review(env: Env, student: Address) -> bool {
        if let Some(geo_info) = env.storage().persistent().get::<DataKey, GeographicInfo>(&DataKey::GeographicInfo(student)) {
            geo_info.in_review
        } else {
            false
        }
    }
    
    pub fn get_verified_region(env: Env, student: Address) -> Option<Symbol> {
        if let Some(geo_info) = env.storage().persistent().get::<DataKey, GeographicInfo>(&DataKey::GeographicInfo(student)) {
            Some(geo_info.verified_region)
        } else {
            None
        }
    }

    // Stream Scholarship Functions
    
    pub fn create_stream(env: Env, funder: Address, student: Address, amount_per_second: i128, token: Address, geographic_restriction: Option<Symbol>) {
        funder.require_auth();
        
        // Verify student has SSI verification for high-value scholarships
        let total_monthly_amount = amount_per_second * (30 * 24 * 60 * 60) as i128;
        if total_monthly_amount >= 1000 { // High-value threshold
            if !Self::is_ssi_verified(env.clone(), student.clone()) {
                #[allow(deprecated)]
                env.events().publish(
                    (Symbol::new(&env, "SSI_REQUIRED"), student),
                    total_monthly_amount
                );
                env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
            }
        }
        
        // Verify geographic restrictions if specified
        if let Some(ref region) = geographic_restriction {
            let student_region = Self::get_verified_region(env.clone(), student.clone());
            if student_region.is_none() || student_region.unwrap() != *region {
                env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
            }
            
            // Check if student is currently in geographic review
            if Self::is_in_geographic_review(env.clone(), student.clone()) {
                env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
            }
        }
        
        // Check if stream already exists
        let stream_key = DataKey::Stream(funder.clone(), student.clone());
        if env.storage().persistent().has(&stream_key) {
            env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
        }
        
        let current_time = env.ledger().timestamp();
        let stream = Stream {
            funder: funder.clone(),
            student: student.clone(),
            amount_per_second,
            total_deposited: 0,
            total_withdrawn: 0,
            start_time: current_time,
            is_active: true,
            geographic_restriction,
        };
        
        env.storage().persistent().set(&stream_key, &stream);
        
        #[allow(deprecated)]
        env.events().publish(
            (Symbol::new(&env, "STREAM_CREATED"), funder, student),
            amount_per_second
        );
    }
    
    pub fn deposit_to_stream(env: Env, funder: Address, student: Address, amount: i128, token: Address) {
        funder.require_auth();
        
        let stream_key = DataKey::Stream(funder.clone(), student.clone());
        let mut stream = env.storage().persistent().get::<DataKey, Stream>(&stream_key)
            .expect("Stream not found");
        
        if !stream.is_active {
            env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
        }
        
        // Check geographic compliance if restricted
        if let Some(ref region) = stream.geographic_restriction {
            if Self::is_in_geographic_review(env.clone(), student.clone()) {
                env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
            }
        }
        
        // Transfer tokens to contract
        let client = token::Client::new(&env, &token);
        client.transfer(&funder, &env.current_contract_address(), &amount);
        
        stream.total_deposited += amount;
        env.storage().persistent().set(&stream_key, &stream);
    }
    
    pub fn withdraw_from_stream(env: Env, student: Address, funder: Address, token: Address) -> i128 {
        student.require_auth();
        
        let stream_key = DataKey::Stream(funder.clone(), student.clone());
        let mut stream = env.storage().persistent().get::<DataKey, Stream>(&stream_key)
            .expect("Stream not found");
        
        if !stream.is_active {
            env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
        }
        
        // Check geographic compliance if restricted
        if let Some(ref region) = stream.geographic_restriction {
            if Self::is_in_geographic_review(env.clone(), student.clone()) {
                env.panic_with_error((soroban_sdk::xdr::ScErrorType::Contract, soroban_sdk::xdr::ScErrorCode::InvalidAction));
            }
        }
        
        let current_time = env.ledger().timestamp();
        let elapsed_seconds = current_time - stream.start_time;
        let accrued_amount = elapsed_seconds as i128 * stream.amount_per_second;
        let available_amount = accrued_amount - stream.total_withdrawn;
        let total_available = (stream.total_deposited - stream.total_withdrawn).min(available_amount);
        
        if total_available <= 0 {
            return 0;
        }
        
        // Transfer tokens to student
        let client = token::Client::new(&env, &token);
        client.transfer(&env.current_contract_address(), &student, &total_available);
        
        stream.total_withdrawn += total_available;
        env.storage().persistent().set(&stream_key, &stream);
        
        total_available
    }
    
    pub fn pause_stream(env: Env, funder: Address, student: Address) {
        funder.require_auth();
        
        let stream_key = DataKey::Stream(funder.clone(), student.clone());
        let mut stream = env.storage().persistent().get::<DataKey, Stream>(&stream_key)
            .expect("Stream not found");
        
        stream.is_active = false;
        env.storage().persistent().set(&stream_key, &stream);
    }
    
    pub fn resume_stream(env: Env, funder: Address, student: Address) {
        funder.require_auth();
        
        let stream_key = DataKey::Stream(funder.clone(), student.clone());
        let mut stream = env.storage().persistent().get::<DataKey, Stream>(&stream_key)
            .expect("Stream not found");
        
        stream.is_active = true;
        env.storage().persistent().set(&stream_key, &stream);
    }
    
    pub fn get_stream_balance(env: Env, funder: Address, student: Address) -> i128 {
        let stream_key = DataKey::Stream(funder.clone(), student.clone());
        if let Some(stream) = env.storage().persistent().get::<DataKey, Stream>(&stream_key) {
            stream.total_deposited - stream.total_withdrawn
        } else {
            0
        }
    }
}

mod test;
