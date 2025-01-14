use crate::*;

impl Contract {
    // Run agent to interrupt the swap transaction and request the agent to provide the output amount
    pub fn run_agent_market_maker(
        &mut self,
        sender_id: AccountId,
        token_in: AccountId,
        token_out: AccountId,
        amount_in: Balance,
        min_amount_out: Balance,
    ) {
        let swap_request_data = json!({
            "sender_id": sender_id.clone(),
            "token_in": token_in.clone(),
            "token_out": token_out.clone(),
            "amount_in": U128::from(amount_in),
            "min_amount_out": U128::from(min_amount_out)
        });

        // Create a promise to resume the swap transaction after the agent responds
        let promise_idx = env::promise_yield_create(
            "on_agent_market_maker_response",
            &swap_request_data.to_string().into_bytes(),
            MIN_RESPONSE_GAS,
            GasWeight::default(),
            DATA_ID_REGISTER,
        );

        // Get the data_id of the register with promises
        let data_id: CryptoHash = env::read_register(DATA_ID_REGISTER)
            .expect("Register is empty")
            .try_into()
            .expect("Wrong register length");

        // emit the agent event with the swap request data
        events::emit::run_agent(&self.agent, &swap_request_data.to_string(), Some(data_id));

        // Return the promise index to the caller
        env::promise_return(promise_idx);
    }
}

#[near_bindgen]
impl Contract {
    // Agent to response to the swap transaction with the output amount
    pub fn agent_response(&mut self, data_id: CryptoHash, amount_out: U128) {
        log!("Agent resolved the swap. Amount_out: {}", amount_out.0);

        assert_eq!(env::predecessor_account_id(), self.agent_account_id, "Illegal agent account_id");

        // resume the initial swap transaction with the amount_out from agent
        if !env::promise_yield_resume(&data_id, &serde_json::to_vec(&amount_out).unwrap()) {
            env::panic_str("Unable to resume promise")
        }
    }

    #[private]
    // Callback function to handle the agent response
    pub fn on_agent_market_maker_response(
        &mut self,
        sender_id: AccountId,
        token_in: AccountId,
        token_out: AccountId,
        amount_in: U128,
        min_amount_out: U128,
        #[callback_result] amount_out: Result<U128, PromiseError>,
    ) -> Option<Balance> {
        if let Ok(response) = amount_out.as_ref() {
            let amount_out = response.0;

            assert!(
                amount_out >= min_amount_out.0,
                "Output amount {} is less than minimum {}",
                amount_out,
                min_amount_out.0
            );

            // update pool balances
            let amount_out = self.internal_swap(&token_in, &token_out, amount_in.0, amount_out);

            // send the output token to the sender
            ext_ft::ext(token_out.clone())
                .with_static_gas(GAS_FOR_FT_TRANSFER)
                .with_attached_deposit(NearToken::from_yoctonear(1))
                .ft_transfer(
                    sender_id,
                    U128(amount_out),
                    Some("Swap completed".to_string()),
                );

            Some(amount_out)
        } else {
            log!("Response error");
            None
        }
    }
}


