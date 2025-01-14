use crate::*;

pub mod emit {
    use super::*;
    use near_sdk::serde_json::json;
    use near_sdk::{env, log, AccountId};

    #[derive(Serialize)]
    #[serde(crate = "near_sdk::serde")]
    struct AgentData<'a> {
        pub message: &'a String,
        pub agent: &'a String,
        pub request_id: Option<CryptoHash>,
        pub max_iterations: &'a Option<u8>,
        pub thread_id: &'a Option<String>,
        pub env_vars: &'a Option<String>,

        signer_id: &'a AccountId,
        referral_id: &'a Option<AccountId>,
        #[serde(with = "option_u128_dec_format")]
        pub amount: Option<u128>,
    }

    fn log_event<T: Serialize>(event: &str, data: T) {
        let event = json!({
            "standard": "nearai",
            "version": "0.1.0",
            "event": event,
            "data": [data]
        });

        log!("EVENT_JSON:{}", event.to_string());
    }

    pub fn run_agent(agent: &str, message: &String, request_id: Option<CryptoHash>) {
        log_event(
            "run_agent",
            AgentData {
                message,
                agent: &agent.to_string(),
                request_id,
                max_iterations: &None,
                thread_id: &None,
                env_vars: &None,
                signer_id: &env::predecessor_account_id(),
                referral_id: &None,
                amount: None,
            },
        );
    }
}

pub mod option_u128_dec_format {
    use near_sdk::serde::Serializer;

    pub fn serialize<S>(num: &Option<u128>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&num.unwrap_or_default().to_string())
    }
}
