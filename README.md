# AMM, where the "A" stands for Agent

### Agent for Resolving Swaps

This project introduces a smart contract and an integrated agent mechanism to streamline and automate token swaps using the NEAR blockchain. The focus of the design is on the agent mechanism, which resolves swap transactions by calculating the output amount based on the Automated Market Maker (AMM) formula.


## Workflow Overview

- A user initiates a token swap via the smart contract.
- The contract interrupts the transaction and delegates the output calculation to the NEAR AI agent.
- The agent processes the swap details, performs the calculation, and responds to the contract with the result.
- The contract resumes the transaction, finalizes the swap, and transfers tokens to the user.
  This design leverages the agent to automate complex calculations, ensuring efficient and accurate swap execution while maintaining security and transparency.


> ⚠ Warning: This AMM implementation is intended for experimental and educational purposes only. The code is AI-generated and has not been thoroughly audited for production use. Proceed with caution and do not deploy in a production environment.


## Smart Contract: Core Functionality

The contract includes the following key components:

### Agent Invocation:

- The `run_agent_market_maker` function interrupts a swap transaction to delegate the calculation of the output amount to the agent.
- Swap request details (e.g., tokens, input amount, and minimum output) are serialized and emitted as an event for the agent to process.
- A promise is created to resume the swap transaction once the agent provides a response.

### Agent Response Handling:

- The `agent_response` function allows the agent to return the calculated output amount.

- It resumes the interrupted swap transaction using the provided output amount.


### Callback Handling:

The `on_agent_market_maker_response` function verifies the agent's response and completes the swap by transferring the output tokens to the user.


## Agent: Intelligent Swap Resolution

The agent, implemented in Python, listens for swap requests and resolves them as follows:

### Listening for Swap Events:

- The agent processes the run_agent event emitted by the smart contract.

- It extracts swap details such as tokens, input amount, and loads pool balances from the chain.

### AMM Calculation:

- Using the AMM formula (k = balance_in * balance_out), the agent calculates the output amount (amount_out) based on the input amount (amount_in).

- Ensures the calculation respects liquidity pool constraints.

### Response to the Contract:

- The agent calls the agent_response function on the smart contract with the calculated amount_out and the data identifier.
- This action resumes the original transaction and finalizes the swap.


## Features

### 1. Create Pool
- **Function**: `create_pool`
- **Description**: Creates a new liquidity pool with specified token pairs and initial amounts.
- **Parameters**:
    - `token_a`: `AccountId` - The first token in the pool.
    - `amount_a`: `U128` - The initial amount of the first token.
    - `token_b`: `AccountId` - The second token in the pool.
    - `amount_b`: `U128` - The initial amount of the second token.

### 2. Add Liquidity
- **Function**: `add_liquidity_from_deposits`
- **Description**: Adds liquidity to an existing pool from the user's deposits.
- **Parameters**:
    - `token_a`: `AccountId` - The first token in the pool.
    - `token_b`: `AccountId` - The second token in the pool.
    - `amount_a`: `U128` - The amount of the first token to add.
    - `amount_b`: `U128` - The amount of the second token to add.

### 3. Swap Tokens
- **Function**: `internal_swap`
- **Description**: Swaps tokens within a pool using a constant product formula.
- **Parameters**:
    - `token_in`: `AccountId` - The token to swap from.
    - `token_out`: `AccountId` - The token to swap to.
    - `amount_in`: `Balance` - The amount of the input token.
    - `min_amount_out`: `Balance` - The minimum amount of the output token.

### 4. Agent Mechanism
- **Function**: `run_agent_market_maker`
- **Description**: Runs an agent to interrupt the swap transaction and request the agent to provide the output amount.
- **Parameters**:
    - `sender_id`: `AccountId` - The ID of the sender.
    - `token_in`: `AccountId` - The input token.
    - `token_out`: `AccountId` - The output token.
    - `amount_in`: `Balance` - The amount of the input token.
    - `min_amount_out`: `Balance` - The minimum amount of the output token.
  

- **Function**: `agent_response`
- **Description**: Handles the agent's response to the swap transaction with the output amount.
- **Parameters**:
    - `data_id`: `CryptoHash` - The data ID of the register with promises.
    - `amount_out`: `U128` - The output amount from the agent.


- **Function**: `on_agent_market_maker_response`
- **Description**: Callback function to handle the agent's response.
- **Parameters**:
    - `sender_id`: `AccountId` - The ID of the sender.
    - `token_in`: `AccountId` - The input token.
    - `token_out`: `AccountId` - The output token.
    - `amount_in`: `U128` - The amount of the input token.
    - `min_amount_out`: `U128` - The minimum amount of the output token.
    - `amount_out`: `Result<U128, PromiseError>` - The result of the agent's response.

## Testing

The contract includes several tests to ensure its functionality:
- `test_new`: Verifies the initial state of the contract.
- `test_create_pool`: Tests the creation of a new pool.
- `test_create_pool_small_deposit`: Ensures that creating a pool with insufficient deposit panics.
- `test_get_pool_info`: Verifies the retrieval of pool information.
- `test_get_shares`: Tests the retrieval of user shares in a pool.
- `test_internal_deposit`: Verifies the internal deposit functionality.
- `test_internal_swap`: Tests the internal swap functionality using a constant product formula.

## Usage

To use this contract, deploy it to the NEAR blockchain and interact with it using the provided functions. Ensure you have sufficient token balances for creating pools and adding liquidity.

## License

This project is licensed under the MIT License.

## DEMO DEPLOYMENT:

Smart contract: `amm.ai-is-near.near`

NEAR AI AGENT: https://app.near.ai/agents/zavodil.near/amm/latest/source

Example of transaction: https://nearblocks.io/txns/6PifgzUUv7YLeWYDNAzRZdrNsd7ooraCqvhVD8DGU6Ui#execution