#![doc = include_str!("../README.md")]

#[macro_use]
extern crate pbc_contract_codegen;

use pbc_contract_common::address::{Address, AddressType};
use pbc_contract_common::context::{CallbackContext, ContractContext};
use pbc_contract_common::events::EventGroup;

use defi_common::interact_mpc20::MPC20Contract;

/// Initial state after contract creation.
const STATE_CREATED: u8 = 0;
/// State after tokens have been transferred to the contract.
/// The contract now awaits approval from the approver.
const STATE_AWAITING_APPROVAL: u8 = 1;
/// State after the approver has signalled fulfilment of the condition
const STATE_APPROVED: u8 = 2;

/// The contract state.
///
/// ### Fields:
///
///   * `sender`: The sender of the tokens
///
///   * `receiver`: The receiver of tokens following approval of the condition.
///
///   * `approver`: The approver that can signal fulfilment of the condition.
///
///   * `token_type`: The address of the token used in the contract.
///
///   * `balance`: The amount of tokens currently in the contract.
///
///   * `start_time_millis`: The start time of the contract milliseconds.
///
///   * `end_time_millis`: The dead line of the contract in milliseconds.
///
///   * `status`: The current status of the contract.
///
#[state]
pub struct ContractState {
    sender: Address,
    receiver: Address,
    approver: Address,
    token_type: Address,
    balance: u128,
    start_time_millis: i64,
    end_time_millis: i64,
    status: u8,
}

/// Initial function to bootstrap the contract's state.
///
/// ### Parameters
///
///   * `context`: The contract context containing sender and chain information.
///
///   * `receiver`: The receiver of tokens following approval of the condition.
///
///   * `approver`: The approver that can signal fulfilment of the condition.
///
///   * `token_type`: The address of the token used in the contract.
///
///   * `hours_until_deadline`: The number of hours until the deadline gets passed.
///
/// ### Returns
///
/// The new state object with the initial state being `STATE_CREATED`.
///
#[init]
pub fn initialize(
    context: ContractContext,
    sender: Address,
    receiver: Address,
    approver: Address,
    token_type: Address,
    hours_until_deadline: u32,
) -> ContractState {
    if token_type.address_type != AddressType::PublicContract {
        panic!("Tried to create a contract selling a non publicContract token");
    }
    let millis_until_deadline = i64::from(hours_until_deadline) * 60 * 60 * 1000;
    let end_time_millis = context.block_production_time + millis_until_deadline;
    ContractState {
        sender,
        receiver,
        approver,
        token_type,
        balance: 0,
        start_time_millis: context.block_production_time,
        end_time_millis,
        status: STATE_CREATED,
    }
}

/// Action for the sender to deposit tokens into the contract.
/// Throws an error if not called by the `sender` or if
/// the status is not `STATE_CREATED`.
/// The function creates a transfer event of tokens from the `sender` to the contract, and
/// a callback to `deposit_callback`.
///
/// ### Parameters:
///
/// * `context`: The context for the action call.
///
/// * `state`: The current state of the contract.
///
/// * `amount`: The amount of tokens to deposit
///
/// ### Returns
///
/// The unchanged state object and the event group containing the
/// transfer event and the callback event.
///
#[action(shortname = 0x01)]
pub fn deposit(
    context: ContractContext,
    state: ContractState,
    amount: u128,
) -> (ContractState, Vec<EventGroup>) {
    if context.sender != state.sender {
        panic!("Deposit can only be called by the sender");
    }
    if state.status == STATE_APPROVED {
        panic!("Cannot deposit tokens after the condition has been fulfilled");
    }
    if context.block_production_time > state.end_time_millis {
        panic!("Cannot deposit tokens after deadline is passed");
    }
    // Create transfer event of tokens from the sender to the contract
    // transfer should callback to deposit_callback
    let mut e = EventGroup::builder();
    MPC20Contract::at_address(state.token_type).transfer_from(
        &mut e,
        &context.sender,
        &context.contract_address,
        amount,
    );
    e.with_callback(SHORTNAME_DEPOSIT_CALLBACK)
        .argument(amount)
        .done();
    let event_group: EventGroup = e.build();

    (state, vec![event_group])
}

/// Callback for depositing tokens. If the transfer was successful the status of the contract
/// is updated to `STATE_AWAITING_APPROVAL`. Otherwise, the callback panics.
///
/// ### Parameters:
///
/// * `ctx`: The contractContext for the callback.
///
/// * `callback_ctx`: The callbackContext.
///
/// * `state`: The current state of the contract.
///
/// ### Returns
///
/// The new state object.
///
#[callback(shortname = 0x02)]
pub fn deposit_callback(
    _ctx: ContractContext,
    callback_ctx: CallbackContext,
    state: ContractState,
    amount: u128,
) -> (ContractState, Vec<EventGroup>) {
    if !callback_ctx.success {
        panic!("Transfer event did not succeed for deposit");
    }
    let mut new_state = state;
    new_state.balance += amount;
    new_state.status = STATE_AWAITING_APPROVAL;
    (new_state, vec![])
}

/// Action for signalling fulfilment of the condition. Panics if the deadline of the
/// contract has been passed, if the caller is not the correct `approver` or if the contract is
/// not in state `STATE_AWAITING_APPROVAL`. Otherwise, updates the status of the contract to `STATE_APPROVED`.
///
/// ### Parameters:
///
/// * `context`: The contractContext for the action.
///
/// * `state`: The current state of the contract.
///
/// ### Returns
///
/// The new state object.
///
#[action(shortname = 0x03)]
pub fn approve(context: ContractContext, state: ContractState) -> (ContractState, Vec<EventGroup>) {
    if context.sender != state.approver {
        panic!("Only the designated approver can approve")
    }
    if context.block_production_time > state.end_time_millis {
        panic!("Condition was fulfilled after deadline was passed");
    }
    if state.status != STATE_AWAITING_APPROVAL {
        panic!("Tried to approve when status was not STATE_AWAITING_APPROVAL")
    }

    let mut new_state = state;
    new_state.status = STATE_APPROVED;
    (new_state, vec![])
}

/// Action for claiming tokens.
/// The `receiver` is allowed to claim the tokens if the status is `STATE_APPROVED`.
/// The `sender` is allowed to claim the tokens if the status is `AWAITING_APPROVAL`
/// and the deadline has been passed.
/// No other addresses can claim tokens
/// If the tokens are claimed a corresponding transfer event is created and the status is
/// updated to `TOKENS_CLAIMED`.
///
/// ### Parameters:
///
/// * `context`: The context for the action call.
///
/// * `state`: The current state of the contract.
///
/// ### Returns
///
/// The new state object and an event group possibly containing a
/// transfer event.
///
#[action(shortname = 0x04)]
pub fn claim(context: ContractContext, state: ContractState) -> (ContractState, Vec<EventGroup>) {
    let can_claim = context.sender == state.receiver || context.sender == state.sender;
    if !can_claim {
        panic!("Only the sender and the receiver in the escrow transfer can claim tokens");
    }
    if state.status == STATE_CREATED {
        panic!("Cannot claim tokens when no tokens have been deposited");
    }
    if state.balance == 0 {
        panic!("Cannot claim tokens when balance is zero");
    }
    if context.sender == state.receiver && state.status != STATE_APPROVED {
        panic!("The receiver cannot claim unless transfer condition has been fulfilled");
    }
    if context.sender == state.sender {
        if state.status == STATE_APPROVED {
            panic!("The sender cannot claim tokens since the condition has been fulfilled");
        }
        if context.block_production_time < state.end_time_millis {
            panic!("The sender cannot claim tokens before the deadline is passed");
        }
    }

    let mut e = EventGroup::builder();
    MPC20Contract::at_address(state.token_type).transfer(&mut e, &context.sender, state.balance);
    let event_group = e.build();

    let mut new_state = state;
    new_state.balance = 0;

    (new_state, vec![event_group])
}
