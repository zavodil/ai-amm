use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, ext_contract, log, near_bindgen, AccountId, BorshStorageKey, Gas,
    NearToken, PanicOnDefault, PromiseOrValue,
};
use schemars::JsonSchema;
use std::convert::TryInto;

use near_sdk::{GasWeight, PromiseError};
use serde_json::json;

mod agent;
mod events;

type Balance = u128;
pub type CryptoHash = [u8; 32];
const TGAS: u64 = 1_000_000_000_000;
const GAS_FOR_FT_TRANSFER: Gas = Gas::from_gas(10 * TGAS);
const INIT_SHARES_SUPPLY: u128 = 1_000_000_000_000_000;
pub const MIN_RESPONSE_GAS: Gas = Gas::from_tgas(30);
pub const DATA_ID_REGISTER: u64 = 37;

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "near_sdk::borsh")]
enum StorageKey {
    Pools,
    Deposits,
    TokenDeposits { account_id: AccountId },
}

#[derive(Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum Action {
    Deposit {},
    Swap {
        token_out: AccountId,
        min_amount_out: U128,
    },
    AddLiquidity {
        token_other: AccountId,
        amount_other: U128,
    },
}

#[derive(Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum TokenReceiverMessage {
    Execute {
        actions: Vec<Action>
    },
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct SwapRequest {
    #[schemars(with = "String")]
    pub sender_id: AccountId,
    #[schemars(with = "String")]
    pub token_in: AccountId,
    #[schemars(with = "String")]
    pub token_out: AccountId,
    #[schemars(with = "String")]
    pub amount_in: U128,
    #[schemars(with = "String")]
    pub min_amount_out: U128,
}

#[ext_contract(ext_ft)]
pub trait FungibleToken {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128>;
}

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Pool {
    token_a: AccountId,
    token_b: AccountId,
    token_a_balance: Balance,
    token_b_balance: Balance,
    total_shares: Balance,
    shares: UnorderedMap<AccountId, Balance>,
}

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
//#[near(serializers=[borsh])]
pub struct AccountDeposits {
    tokens: UnorderedMap<AccountId, Balance>,
}

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
#[near_bindgen]
pub struct Contract {
    agent: String,
    agent_account_id: AccountId,
    pools: UnorderedMap<String, Pool>,
    deposits: UnorderedMap<AccountId, AccountDeposits>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(agent: String, agent_account_id: AccountId) -> Self {
        Self {
            agent,
            agent_account_id,
            pools: UnorderedMap::new(StorageKey::Pools),
            deposits: UnorderedMap::new(StorageKey::Deposits),
        }
    }

    #[private]
    pub fn set_agent(&mut self, agent: String) {
        self.agent = agent;
    }

    pub fn create_pool(
        &mut self,
        token_a: AccountId,
        token_a_amount: U128,
        token_b: AccountId,
        token_b_amount: U128,
    ) {
        let sender_id = env::predecessor_account_id();
        let deposits = self.get_deposits(&sender_id);
        let balance_token_a = deposits.tokens.get(&token_a).unwrap_or(0);
        assert!(
            balance_token_a >= token_a_amount.0,
            "Need to deposit tokens A"
        );
        let balance_token_b = deposits.tokens.get(&token_b).unwrap_or(0);
        assert!(
            balance_token_b >= token_b_amount.0,
            "Need to deposit tokens B"
        );

        let pool_key = get_pool_key(&token_a, &token_b);
        assert!(self.pools.get(&pool_key).is_none(), "Pool already exists");

        let mut shares_map = UnorderedMap::new(format!("s:{}", pool_key).as_bytes());

        let initial_shares = INIT_SHARES_SUPPLY;
        shares_map.insert(&env::predecessor_account_id(), &initial_shares);

        let pool = Pool {
            token_a,
            token_b,
            token_a_balance: token_a_amount.0,
            token_b_balance: token_b_amount.0,
            total_shares: initial_shares,
            shares: shares_map,
        };

        self.pools.insert(&pool_key, &pool);
    }

    pub fn get_pool_info(
        &self,
        token_a: AccountId,
        token_b: AccountId,
    ) -> Option<(Balance, Balance, Balance)> {
        let pool_key = get_pool_key(&token_a, &token_b);
        self.pools.get(&pool_key).map(|pool| {
            (
                pool.token_a_balance,
                pool.token_b_balance,
                pool.total_shares,
            )
        })
    }

    pub fn get_shares(
        &self,
        token_a: AccountId,
        token_b: AccountId,
        account_id: AccountId,
    ) -> Balance {
        let pool_key = get_pool_key(&token_a, &token_b);
        let pool = self.pools.get(&pool_key).expect("Pool not found");
        pool.shares.get(&account_id).unwrap_or(0)
    }

    pub fn get_user_deposits(&self, account_id: AccountId) -> Vec<(AccountId, U128)> {
        self.deposits
            .get(&account_id)
            .unwrap()
            .tokens
            .iter()
            .map(|(k, v)| (k.clone(), U128(v)))
            .collect()
    }

    fn get_deposits(&self, account_id: &AccountId) -> AccountDeposits {
        if let Some(deposits) = self.deposits.get(account_id) {
            deposits
        } else {
            AccountDeposits {
                tokens: UnorderedMap::new(StorageKey::TokenDeposits {
                    account_id: account_id.clone(),
                }),
            }
        }
    }

    fn internal_deposit(&mut self, account_id: &AccountId, token_id: &AccountId, amount: Balance) {
        let mut deposits = self.get_deposits(account_id);
        let balance = deposits.tokens.get(token_id).unwrap_or(0);
        deposits.tokens.insert(token_id, &(balance + amount));
        self.deposits.insert(account_id, &deposits);
    }

    fn internal_process_token_transfer(
        &mut self,
        sender_id: AccountId,
        token_in: AccountId,
        amount_in: Balance,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let message: TokenReceiverMessage =
            serde_json::from_str(&msg).expect("Failed to parse message");

        match message {
            TokenReceiverMessage::Execute {
                actions
            } => {
                for action in actions {
                    match action {
                        Action::Swap {
                            token_out,
                            min_amount_out,
                        } => {
                            // Deposit the amount_in sent by user in this transaction to the contract
                            self.internal_deposit(&sender_id, &token_in, amount_in);

                            // Run agent to interrupt the swap transaction
                            self.run_agent_market_maker(
                                sender_id.clone(),
                                token_in.clone(),
                                token_out,
                                amount_in,
                                min_amount_out.0,
                            );
                        }
                        Action::Deposit {} => {
                            self.internal_deposit(&sender_id, &token_in, amount_in);
                            log!("Deposit successfull")
                        }
                        Action::AddLiquidity {
                            token_other,
                            amount_other,
                        } => {
                            let deposits = self.get_deposits(&sender_id);
                            let balance_other = deposits.tokens.get(&token_other).unwrap_or(0);

                            assert!(
                                balance_other >= amount_other.0,
                                "Insufficient balance of token_other for add_liquidity"
                            );

                            self.internal_add_liquidity(
                                &token_in,
                                &token_other,
                                amount_in,
                                amount_other.0,
                                &sender_id,
                            );
                        }
                    }
                }
            }
        }

        PromiseOrValue::Value(U128(0))
    }

    pub fn get_swap_balances(&self, token_in: AccountId, token_out: AccountId) -> (U128, U128) {
        let pool_key = get_pool_key(&token_in, &token_out);
        let pool = self.pools.get(&pool_key).expect("Pool not found");
        let (balance_in, balance_out) = if token_in == pool.token_a {
            (pool.token_a_balance, pool.token_b_balance)
        } else {
            (pool.token_b_balance, pool.token_a_balance)
        };

        (U128::from(balance_in), U128::from(balance_out))
    }

    fn internal_swap(
        &mut self,
        token_in: &AccountId,
        token_out: &AccountId,
        amount_in: Balance,
        amount_out: Balance,
    ) -> Balance {
        let pool_key = get_pool_key(token_in, token_out);
        let mut pool = self.pools.get(&pool_key).expect("Pool not found");

        let (balance_in, balance_out) = if token_in == &pool.token_a {
            (pool.token_a_balance, pool.token_b_balance)
        } else {
            (pool.token_b_balance, pool.token_a_balance)
        };

        let new_balance_in = balance_in + amount_in;
        let new_balance_out = balance_out - amount_out;

        if token_in == &pool.token_a {
            pool.token_a_balance = new_balance_in;
            pool.token_b_balance = new_balance_out;
        } else {
            pool.token_b_balance = new_balance_in;
            pool.token_a_balance = new_balance_out;
        }

        self.pools.insert(&pool_key, &pool);
        amount_out
    }

    fn internal_add_liquidity(
        &mut self,
        token_in: &AccountId,
        token_other: &AccountId,
        amount_in: Balance,
        amount_other: Balance,
        sender_id: &AccountId,
    ) {
        let pool_key = get_pool_key(token_in, token_other);
        let mut pool = self.pools.get(&pool_key).expect("Pool not found");

        let (token_a_amount, token_b_amount) = if token_in == &pool.token_a {
            (amount_in, amount_other)
        } else {
            (amount_other, amount_in)
        };

        let share = std::cmp::min(
            token_a_amount * pool.total_shares / pool.token_a_balance,
            token_b_amount * pool.total_shares / pool.token_b_balance,
        );

        pool.token_a_balance += token_a_amount;
        pool.token_b_balance += token_b_amount;
        pool.total_shares += share;

        let user_shares = pool.shares.get(sender_id).unwrap_or(0);
        pool.shares.insert(sender_id, &(user_shares + share));

        self.pools.insert(&pool_key, &pool);
    }

    pub fn add_liquidity_from_deposits(
        &mut self,
        token_a: AccountId,
        token_b: AccountId,
        amount_a: U128,
        amount_b: U128,
    ) -> Balance {
        let sender_id = env::predecessor_account_id();
        let deposits = self.get_deposits(&sender_id);

        let balance_a = deposits.tokens.get(&token_a).unwrap_or(0);
        let balance_b = deposits.tokens.get(&token_b).unwrap_or(0);

        assert!(balance_a >= amount_a.0, "Insufficient balance of token_a");
        assert!(balance_b >= amount_b.0, "Insufficient balance of token_b");

        self.internal_add_liquidity(&token_a, &token_b, amount_a.0, amount_b.0, &sender_id);

        let pool_key = get_pool_key(&token_a, &token_b);
        let pool = self.pools.get(&pool_key).unwrap();
        pool.shares.get(&sender_id).unwrap_or(0)
    }
}

fn get_pool_key(token_a: &AccountId, token_b: &AccountId) -> String {
    let mut tokens = [token_a.to_string(), token_b.to_string()];
    tokens.sort();
    format!("{}:{}", tokens[0], tokens[1])
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let token_in = env::predecessor_account_id();

        self.internal_process_token_transfer(sender_id, token_in, amount.0, msg);

        PromiseOrValue::Value(U128(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    // Helper function to set up test context
    fn get_contract() -> (Contract, AccountId, AccountId, AccountId) {
        let pools = UnorderedMap::new(StorageKey::Pools);
        let mut deposits: UnorderedMap<AccountId, AccountDeposits> =
            UnorderedMap::new(StorageKey::Deposits);

        let account_a: AccountId = AccountId::from_str("bob.near").unwrap();
        let account_b: AccountId = AccountId::from_str("token_in.near").unwrap();
        let account_c: AccountId = AccountId::from_str("token_out.near").unwrap();

        let mut account_a_tokens: UnorderedMap<AccountId, Balance> =
            UnorderedMap::new(StorageKey::TokenDeposits {
                account_id: account_a.clone(),
            });
        account_a_tokens.insert(&account_b, &1_000_000);
        account_a_tokens.insert(&account_c, &1_000_000);
        deposits.insert(
            &account_a,
            &AccountDeposits {
                tokens: account_a_tokens,
            },
        );

        let contract = Contract {
            agent: "test-agent".to_string(),
            agent_account_id: AccountId::from_str("agent.near").unwrap(),
            pools,
            deposits,
        };

        (contract, account_a, account_b, account_c)
    }

    #[test]
    fn test_new() {
        let (contract, _, _, _) = get_contract();
        assert!(contract.pools.is_empty());
        assert_eq!(contract.deposits.len(), 1);
    }

    #[test]
    fn test_create_pool() {
        let (mut contract, _, account_b, account_c) = get_contract();
        contract.create_pool(
            account_b.clone(),
            U128(1_000_000),
            account_c.clone(),
            U128(1_000_000),
        );

        let pool_info = contract.get_pool_info(account_b, account_c);
        assert!(pool_info.is_some());
        let (token_a_balance, token_b_balance, total_shares) = pool_info.unwrap();
        assert_eq!(token_a_balance, 1_000_000);
        assert_eq!(token_b_balance, 1_000_000);
        assert_eq!(total_shares, INIT_SHARES_SUPPLY);
    }

    #[test]
    #[should_panic(expected = "Need to deposit tokens A")]
    fn test_create_pool_small_deposit() {
        let (mut contract, _, account_b, account_c) = get_contract();
        contract.create_pool(account_b, U128(2_000_000), account_c, U128(1_000_000));
    }

    #[test]
    fn test_get_pool_info() {
        let (mut contract, _, account_b, account_c) = get_contract();
        contract.create_pool(
            account_b.clone(),
            U128(1_000_000),
            account_c.clone(),
            U128(1_000_000),
        );

        let pool_info = contract.get_pool_info(account_b.clone(), account_c.clone());
        assert!(pool_info.is_some());

        let pool_info_reverse = contract.get_pool_info(account_c, account_b);
        assert_eq!(pool_info, pool_info_reverse);
    }

    #[test]
    fn test_get_shares() {
        let (mut contract, account_a, account_b, account_c) = get_contract();

        contract.create_pool(
            account_b.clone(),
            U128(1_000_000),
            account_c.clone(),
            U128(1_000_000),
        );

        let shares = contract.get_shares(account_b, account_c, account_a);
        assert_eq!(shares, INIT_SHARES_SUPPLY);
    }

    #[test]
    fn test_internal_deposit() {
        let (mut contract, account_a, account_b, _) = get_contract();
        let account = account_a;
        let token = account_b;

        contract.internal_deposit(&account, &token, 1_000_000);
        let deposits = contract.get_deposits(&account);
        assert_eq!(deposits.tokens.get(&token).unwrap(), 2_000_000);
    }

    #[test]
    fn test_internal_swap() {
        let (mut contract, _, account_b, account_c) = get_contract();
        // Create pool with 1:1 ratio
        contract.create_pool(
            account_b.clone(),
            U128(1_000_000),
            account_c.clone(),
            U128(1_000_000),
        );

        // Test swap
        let amount_out = contract.internal_swap(&account_b, &account_c, 500, 500);

        // Verify the swap result
        // Using constant product formula: (x + dx)(y - dy) = xy
        // Where x = 1_000_000, dx = 500_000
        assert!(amount_out > 0);

        let pool_info = contract.get_pool_info(account_b, account_c).unwrap();
        assert_eq!(pool_info.0, 1000_500); // token_a_balance
        assert_eq!(pool_info.1, 999_500); // token_b_balance
    }
}
