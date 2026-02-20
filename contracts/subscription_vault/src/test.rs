use crate::{
    FundsDepositedEvent, MerchantWithdrawalEvent, Subscription, SubscriptionCancelledEvent,
    SubscriptionChargedEvent, SubscriptionCreatedEvent, SubscriptionPausedEvent,
    SubscriptionResumedEvent, SubscriptionStatus, SubscriptionVault, SubscriptionVaultClient,
};
use soroban_sdk::testutils::{Address as _, Events};
use soroban_sdk::{symbol_short, Address, Env, IntoVal, TryFromVal, Val};

// ---------------------------------------------------------------------------
// Helper: decode the event data payload (3rd element of event tuple)
// ---------------------------------------------------------------------------
fn last_event_data<T: TryFromVal<Env, Val>>(env: &Env) -> T {
    let events = env.events().all();
    let last = events.last().unwrap();
    T::try_from_val(env, &last.2).unwrap()
}

// ===========================================================================
// Basic struct / init tests
// ===========================================================================

#[test]
fn test_init_and_struct() {
    let env = Env::default();
    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    client.init(&token, &admin);
}

#[test]
fn test_subscription_struct() {
    let env = Env::default();
    let sub = Subscription {
        subscriber: Address::generate(&env),
        merchant: Address::generate(&env),
        amount: 10_000_0000,
        interval_seconds: 30 * 24 * 60 * 60,
        last_payment_timestamp: 0,
        status: SubscriptionStatus::Active,
        prepaid_balance: 50_000_0000,
        usage_enabled: false,
    };
    assert_eq!(sub.status, SubscriptionStatus::Active);
}

// ===========================================================================
// Gap 1 — Payload-level assertions for each lifecycle action
// ===========================================================================

#[test]
fn test_create_subscription_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);
    let amount = 10_000_0000i128;
    let interval = 2_592_000u64;

    let sub_id = client.create_subscription(&subscriber, &merchant, &amount, &interval, &false);

    // Topic check
    let events = env.events().all();
    let last_event = events.last().unwrap();
    assert_eq!(last_event.0, contract_id);
    assert_eq!(
        last_event.1,
        (symbol_short!("sub_new"),).into_val(&env)
    );

    // Payload check
    let data: SubscriptionCreatedEvent = last_event_data(&env);
    assert_eq!(data.subscription_id, sub_id);
    assert_eq!(data.subscriber, subscriber);
    assert_eq!(data.merchant, merchant);
    assert_eq!(data.amount, amount);
    assert_eq!(data.interval_seconds, interval);
}

#[test]
fn test_deposit_funds_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);

    let sub_id =
        client.create_subscription(&subscriber, &merchant, &10_000_0000, &2_592_000, &false);

    client.deposit_funds(&sub_id, &subscriber, &50_000_0000);

    // Topic check
    let events = env.events().all();
    let last_event = events.last().unwrap();
    assert_eq!(last_event.0, contract_id);
    assert_eq!(
        last_event.1,
        (symbol_short!("deposit"),).into_val(&env)
    );

    // Payload check
    let data: FundsDepositedEvent = last_event_data(&env);
    assert_eq!(data.subscription_id, sub_id);
    assert_eq!(data.subscriber, subscriber);
    assert_eq!(data.amount, 50_000_0000);
    assert_eq!(data.new_balance, 50_000_0000);
}

#[test]
fn test_charge_subscription_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);
    let amount = 10_000_0000i128;

    let sub_id =
        client.create_subscription(&subscriber, &merchant, &amount, &2_592_000, &false);
    client.deposit_funds(&sub_id, &subscriber, &50_000_0000);

    client.charge_subscription(&sub_id);

    // Topic check
    let events = env.events().all();
    let last_event = events.last().unwrap();
    assert_eq!(last_event.0, contract_id);
    assert_eq!(
        last_event.1,
        (symbol_short!("charged"),).into_val(&env)
    );

    // Payload check
    let data: SubscriptionChargedEvent = last_event_data(&env);
    assert_eq!(data.subscription_id, sub_id);
    assert_eq!(data.merchant, merchant);
    assert_eq!(data.amount, amount);
    assert_eq!(data.remaining_balance, 40_000_0000); // 50 - 10
}

#[test]
fn test_pause_subscription_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);

    let sub_id =
        client.create_subscription(&subscriber, &merchant, &10_000_0000, &2_592_000, &false);

    client.pause_subscription(&sub_id, &subscriber);

    // Topic check
    let events = env.events().all();
    let last_event = events.last().unwrap();
    assert_eq!(last_event.0, contract_id);
    assert_eq!(
        last_event.1,
        (symbol_short!("paused"),).into_val(&env)
    );

    // Payload check
    let data: SubscriptionPausedEvent = last_event_data(&env);
    assert_eq!(data.subscription_id, sub_id);
    assert_eq!(data.authorizer, subscriber);
}

#[test]
fn test_resume_subscription_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);

    let sub_id =
        client.create_subscription(&subscriber, &merchant, &10_000_0000, &2_592_000, &false);
    client.pause_subscription(&sub_id, &subscriber);
    client.resume_subscription(&sub_id, &subscriber);

    // Topic check
    let events = env.events().all();
    let last_event = events.last().unwrap();
    assert_eq!(last_event.0, contract_id);
    assert_eq!(
        last_event.1,
        (symbol_short!("resumed"),).into_val(&env)
    );

    // Payload check
    let data: SubscriptionResumedEvent = last_event_data(&env);
    assert_eq!(data.subscription_id, sub_id);
    assert_eq!(data.authorizer, subscriber);
}

#[test]
fn test_cancel_subscription_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);

    let sub_id =
        client.create_subscription(&subscriber, &merchant, &10_000_0000, &2_592_000, &false);
    client.deposit_funds(&sub_id, &subscriber, &50_000_0000);

    client.cancel_subscription(&sub_id, &subscriber);

    // Topic check
    let events = env.events().all();
    let last_event = events.last().unwrap();
    assert_eq!(last_event.0, contract_id);
    assert_eq!(
        last_event.1,
        (symbol_short!("cancelled"),).into_val(&env)
    );

    // Payload check
    let data: SubscriptionCancelledEvent = last_event_data(&env);
    assert_eq!(data.subscription_id, sub_id);
    assert_eq!(data.authorizer, subscriber);
    assert_eq!(data.refund_amount, 50_000_0000);
}

#[test]
fn test_withdraw_merchant_funds_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);
    let charge_amount = 10_000_0000i128;

    // Create, deposit, and charge so the merchant has a balance
    let sub_id =
        client.create_subscription(&subscriber, &merchant, &charge_amount, &2_592_000, &false);
    client.deposit_funds(&sub_id, &subscriber, &50_000_0000);
    client.charge_subscription(&sub_id);

    // Withdraw
    let withdraw_amount = 5_000_0000i128;
    client.withdraw_merchant_funds(&merchant, &withdraw_amount);

    // Topic check
    let events = env.events().all();
    let last_event = events.last().unwrap();
    assert_eq!(last_event.0, contract_id);
    assert_eq!(
        last_event.1,
        (symbol_short!("withdraw"),).into_val(&env)
    );

    // Payload check
    let data: MerchantWithdrawalEvent = last_event_data(&env);
    assert_eq!(data.merchant, merchant);
    assert_eq!(data.amount, withdraw_amount);
    assert_eq!(data.remaining_balance, charge_amount - withdraw_amount); // 10 - 5 = 5
}

// ===========================================================================
// Gap 2 — Negative / edge-case tests
// ===========================================================================

#[test]
#[should_panic(expected = "Error(Contract, #404)")]
fn test_deposit_nonexistent_subscription_no_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    let subscriber = Address::generate(&env);
    // Subscription 999 doesn't exist — must error, no event emitted
    client.deposit_funds(&999, &subscriber, &50_000_0000);
}

#[test]
#[should_panic(expected = "Error(Contract, #404)")]
fn test_charge_nonexistent_subscription_no_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    // Subscription 999 doesn't exist — must error, no event emitted
    client.charge_subscription(&999);
}

#[test]
#[should_panic(expected = "Error(Contract, #402)")]
fn test_charge_insufficient_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);

    // Create with amount=10, deposit only 5 → charge should fail
    let sub_id =
        client.create_subscription(&subscriber, &merchant, &10_000_0000, &2_592_000, &false);
    client.deposit_funds(&sub_id, &subscriber, &5_000_0000);

    client.charge_subscription(&sub_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #402)")]
fn test_withdraw_exceeds_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);

    // Merchant has 10 from one charge, tries to withdraw 20
    let sub_id =
        client.create_subscription(&subscriber, &merchant, &10_000_0000, &2_592_000, &false);
    client.deposit_funds(&sub_id, &subscriber, &50_000_0000);
    client.charge_subscription(&sub_id);

    client.withdraw_merchant_funds(&merchant, &20_000_0000);
}

#[test]
#[should_panic(expected = "Error(Contract, #402)")]
fn test_withdraw_no_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    // Merchant has no accumulated balance at all
    client.withdraw_merchant_funds(&merchant, &1);
}

#[test]
fn test_event_count_single_create() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);

    client.create_subscription(&subscriber, &merchant, &10_000_0000, &2_592_000, &false);

    // Exactly 1 event should be emitted from the contract
    let contract_events: u32 = env
        .events()
        .all()
        .iter()
        .filter(|e| e.0 == contract_id)
        .count() as u32;
    assert_eq!(contract_events, 1);
}

#[test]
fn test_cancel_already_cancelled() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);

    let sub_id =
        client.create_subscription(&subscriber, &merchant, &10_000_0000, &2_592_000, &false);
    client.deposit_funds(&sub_id, &subscriber, &50_000_0000);

    // Cancel once — should emit with refund_amount == 50
    client.cancel_subscription(&sub_id, &subscriber);
    let data1: SubscriptionCancelledEvent = last_event_data(&env);
    assert_eq!(data1.refund_amount, 50_000_0000);

    // Cancel again — should still succeed but refund_amount stays the same (balance unchanged)
    client.cancel_subscription(&sub_id, &subscriber);
    let data2: SubscriptionCancelledEvent = last_event_data(&env);
    assert_eq!(data2.refund_amount, 50_000_0000);
}

// ===========================================================================
// Gap 5 — Full lifecycle test including merchant withdrawal
// ===========================================================================

#[test]
fn test_full_lifecycle_events() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);
    let charge_amount = 10_000_0000i128;

    // 1. Create
    let sub_id =
        client.create_subscription(&subscriber, &merchant, &charge_amount, &2_592_000, &false);
    assert_eq!(
        env.events().all().last().unwrap().1,
        (symbol_short!("sub_new"),).into_val(&env)
    );

    // 2. Deposit
    client.deposit_funds(&sub_id, &subscriber, &50_000_0000);
    assert_eq!(
        env.events().all().last().unwrap().1,
        (symbol_short!("deposit"),).into_val(&env)
    );

    // 3. Charge
    client.charge_subscription(&sub_id);
    assert_eq!(
        env.events().all().last().unwrap().1,
        (symbol_short!("charged"),).into_val(&env)
    );

    // 4. Merchant withdrawal (now possible because charge accumulated balance)
    client.withdraw_merchant_funds(&merchant, &charge_amount);
    assert_eq!(
        env.events().all().last().unwrap().1,
        (symbol_short!("withdraw"),).into_val(&env)
    );
    let wd: MerchantWithdrawalEvent = last_event_data(&env);
    assert_eq!(wd.remaining_balance, 0);

    // 5. Pause
    client.pause_subscription(&sub_id, &subscriber);
    assert_eq!(
        env.events().all().last().unwrap().1,
        (symbol_short!("paused"),).into_val(&env)
    );

    // 6. Resume
    client.resume_subscription(&sub_id, &subscriber);
    assert_eq!(
        env.events().all().last().unwrap().1,
        (symbol_short!("resumed"),).into_val(&env)
    );

    // 7. Cancel
    client.cancel_subscription(&sub_id, &subscriber);
    assert_eq!(
        env.events().all().last().unwrap().1,
        (symbol_short!("cancelled"),).into_val(&env)
    );
}
