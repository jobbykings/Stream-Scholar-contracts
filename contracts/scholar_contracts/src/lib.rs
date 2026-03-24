#![no_std]
use soroban_sdk::{contract, contracttype, contractimpl, Address, Env, token};

const EARLY_DROP_WINDOW_SECONDS: u64 = 300; // 5 minutes

#[contracttype]
#[derive(Clone)]
pub struct Access {
    pub student: Address,
    pub course_id: u64,
    pub expiry_time: u64,
    pub token: Address,
    pub last_purchase_time: u64,
}

#[contracttype]
pub enum DataKey {
    Access(Address, u64),
    Price,
}

#[contract]
pub struct ScholarContract;

#[contractimpl]
impl ScholarContract {
    pub fn init(env: Env, rate: i128) {
        env.storage().instance().set(&DataKey::Price, &rate);
    }

    pub fn buy_access(env: Env, student: Address, course_id: u64, amount: i128, token: Address) {
        student.require_auth();

        let client = token::Client::new(&env, &token);
        client.transfer(&student, &env.current_contract_address(), &amount);

        let rate: i128 = env.storage().instance().get(&DataKey::Price).unwrap_or(1); 
        let seconds_bought = (amount / rate) as u64;
        let current_time = env.ledger().timestamp();

        let mut access = env.storage().instance().get(&DataKey::Access(student.clone(), course_id))
            .unwrap_or(Access {
                student: student.clone(),
                course_id,
                expiry_time: current_time,
                token,
                last_purchase_time: current_time,
            });

        if access.expiry_time > current_time {
            access.expiry_time += seconds_bought;
        } else {
            access.expiry_time = current_time + seconds_bought;
        }
        
        // Update last purchase time to current time
        access.last_purchase_time = current_time;

        env.storage().instance().set(&DataKey::Access(student, course_id), &access);
    }

    /// Allows a student to get a 100% refund if they drop within the first 5 minutes of purchase.
    /// Returns the refunded amount.
    pub fn pro_rated_refund(env: Env, student: Address, course_id: u64) -> i128 {
        student.require_auth();
        
        let current_time = env.ledger().timestamp();
        
        // Get the access record
        let access: Access = env.storage().instance().get(&DataKey::Access(student.clone(), course_id))
            .unwrap_or(Access {
                student: student.clone(),
                course_id,
                expiry_time: 0,
                token: student.clone(),
                last_purchase_time: 0,
            });
        
        // Check if within early drop window (5 minutes = 300 seconds)
        let time_since_purchase = current_time.saturating_sub(access.last_purchase_time);
        
        // Only allow refund if within the early drop window
        if time_since_purchase > EARLY_DROP_WINDOW_SECONDS {
            panic!("Refund only available within 5 minutes of purchase");
        }
        
        // Calculate refund amount based on remaining time
        // Get the rate to calculate how much was paid for the remaining time
        let rate: i128 = env.storage().instance().get(&DataKey::Price).unwrap_or(1);
        
        // Calculate remaining seconds
        let remaining_time = if access.expiry_time > current_time {
            access.expiry_time - current_time
        } else {
            0
        };
        
        let refund_amount = (remaining_time as i128) * rate;
        
        // Transfer the refund to the student
        let client = token::Client::new(&env, &access.token);
        client.transfer(&env.current_contract_address(), &student, &refund_amount);
        
        // Remove the access record
        env.storage().instance().remove(&DataKey::Access(student, course_id));
        
        refund_amount
    }

    pub fn has_access(env: Env, student: Address, course_id: u64) -> bool {
        let access: Access = env.storage().instance().get(&DataKey::Access(student.clone(), course_id))
            .unwrap_or(Access {
                student: student.clone(),
                course_id,
                expiry_time: 0,
                token: student,
                last_purchase_time: 0,
            });
            
        env.ledger().timestamp() < access.expiry_time
    }
}

mod test;
