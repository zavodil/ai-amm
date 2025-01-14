import asyncio
import json

from nearai.agents.environment import Environment
from py_near.account import Account

master_account_id = globals()['env'].env_vars.get("master_account_id", None)
master_private_key = globals()['env'].env_vars.get("master_private_key", None)
contract_id = "amm.ai-is-near.near"


async def agent_response(env: Environment, data_id, amount_out):
    # Create an account instance with master account credentials
    acc = Account(master_account_id, master_private_key)

    # Prepare arguments for the function call
    args = {
        "data_id": data_id,
        "amount_out": amount_out,
    }

    # Call the smart contract function 'agent_response' with the prepared arguments
    tr = await acc.function_call(contract_id, 'agent_response', args, 200000000000000, 0)

    # Add a reply to the agent environment with the transaction hash
    env.add_reply(
        f"Transaction created: [{tr.transaction.hash}](https://nearblocks.io/txns/{tr.transaction.hash})")


async def main(env: Environment):
    message = env.get_last_message()

    message_data = json.loads(message["content"])

    event = message_data.get("event")
    request_id = message_data.get("request_id")
    user_message = message_data.get("message", None)

    if event == "run_agent" and user_message is not None and env.signer_account_id == "ai-is-near.near":
        request = json.loads(user_message)
        acc = Account(master_account_id, master_private_key)

        agent_data = await acc.view_function(
            contract_id, "get_swap_balances",
            {"token_in": request.get("token_in"), "token_out": request.get("token_out")})

        print("agent_data.result", agent_data.result)
        balance_in = int(agent_data.result[0])
        balance_out = int(agent_data.result[1])
        amount_in = int(request.get("amount_in"))

        # AMM formula, calculate amount_out
        k = balance_in * balance_out
        new_balance_in = balance_in + amount_in
        if amount_in > 0 and new_balance_in > 0:
            new_balance_out = int(k / new_balance_in)
            amount_out = balance_out - new_balance_out
            await agent_response(env, request_id, str(amount_out))
        else:
            env.add_reply("Illegal amount")
    else:
        env.add_reply("Illegal request")


if not (master_account_id and master_private_key):
    env.add_reply("Agent wasn't initialized yet.")
else:
    asyncio.run(main(env))

env.mark_done()
