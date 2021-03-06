// Temporary fix due to:
// https://github.com/bodil/im-rs/issues/118
use std::collections::HashMap as ImHashMap;

use im::hashset::HashSet as ImHashSet;
use im::vector::Vector as ImVec;

use common::ser_utils::{ser_b64, ser_map_b64_any, ser_option_b64, ser_string};
use signature::canonical::CanonicalSerialize;

use proto::crypto::{HashedLock, InvoiceId, PaymentId, PlainLock, PublicKey, Uid};

use proto::app_server::messages::NamedRelayAddress;
use proto::funder::messages::{AddFriend, Currency, Receipt, ResponseSendFundsOp};

use crate::friend::{FriendMutation, FriendState};

#[derive(Arbitrary, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct FunderState<B: Clone> {
    /// Public key of this node
    #[serde(with = "ser_b64")]
    pub local_public_key: PublicKey,
    /// Addresses of relays we are going to connect to.
    pub relays: ImVec<NamedRelayAddress<B>>,
    /// All configured friends and their state
    #[serde(with = "ser_map_b64_any")]
    #[serde(bound(
        serialize = "B: serde::Serialize",
        deserialize = "B: serde::de::Deserialize<'de>"
    ))]
    pub friends: ImHashMap<PublicKey, FriendState<B>>,
    /// Locally issued invoices in progress (For which this node is the seller)
    #[serde(with = "ser_map_b64_any")]
    pub open_invoices: ImHashMap<InvoiceId, OpenInvoice>,
    /// Locally created transaction in progress. (For which this node is the buyer).
    #[serde(with = "ser_map_b64_any")]
    pub open_transactions: ImHashMap<Uid, OpenTransaction>,
    /// Ongoing payments (For which this node is the buyer):
    #[serde(with = "ser_map_b64_any")]
    pub payments: ImHashMap<PaymentId, Payment>,
}

/// A state of a Payment where new transactions may still be added.
#[derive(Arbitrary, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct NewTransactions {
    pub num_transactions: u64,
    /// We have one src_plain_lock that we are going to use for every Transaction we create through
    /// this payment.
    #[serde(with = "ser_b64")]
    pub invoice_id: InvoiceId,
    pub currency: Currency,
    #[serde(with = "ser_string")]
    pub total_dest_payment: u128,
    #[serde(with = "ser_b64")]
    pub dest_public_key: PublicKey,
}

#[allow(clippy::large_enum_variant)]
#[derive(Arbitrary, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum PaymentStage {
    /// User can add new transactions
    // TODO: Think about a better name for this?
    NewTransactions(NewTransactions),
    /// User can no longer add new transactions (user sent a RequestClosePayment)
    InProgress(u64), // num_transactions
    /// A receipt was received:
    Success(u64, Receipt, #[serde(with = "ser_b64")] Uid), // (num_transactions, Receipt, ack_uid)
    /// The payment will not complete, because all transactions were canceled:
    #[serde(with = "ser_b64")]
    Canceled(Uid), // ack_uid
    /// User already acked, We now wait for the remaining transactions to finish.
    AfterSuccessAck(u64), // num_transactions
}

#[derive(Arbitrary, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Payment {
    #[serde(with = "ser_b64")]
    pub src_plain_lock: PlainLock,
    pub stage: PaymentStage,
}

/*
#[derive(Arbitrary, Clone, Serialize, Deserialize, Debug)]
pub struct IncomingTransaction {
    pub request_id: Uid,
}
*/

/// A local invoice in progress
#[derive(Arbitrary, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct OpenInvoice {
    /// Currency in use for this invoice
    pub currency: Currency,
    /// Total payment required to fulfill this invoice:
    #[serde(with = "ser_string")]
    pub total_dest_payment: u128,
    /// The lock we used on our ResponseSendFundsOp message.
    /// We have to keep it, otherwise we will not be able to send a valid CollectSendFundsOp later.
    #[serde(with = "ser_b64")]
    pub dest_plain_lock: PlainLock,
    /// Lock created by the originator of the transactions used to fulfill this invoice.
    /// We expect all transactions to have the same lock. This allows the buyer to unlock all the
    /// transactions at once by sending a commit message.
    #[serde(with = "ser_option_b64")]
    pub opt_src_hashed_lock: Option<HashedLock>,
    /// Multiple transactions are possible for a single invoice in case of a multi-route payment.
    // TODO: Add serde hint
    pub incoming_transactions: ImHashSet<Uid>,
}

impl OpenInvoice {
    pub fn new(currency: Currency, total_dest_payment: u128, dest_plain_lock: PlainLock) -> Self {
        OpenInvoice {
            currency,
            total_dest_payment,
            dest_plain_lock,
            opt_src_hashed_lock: None,
            incoming_transactions: ImHashSet::new(),
        }
    }
}

/// A local request (Originated from this node) in progress
#[derive(Arbitrary, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct OpenTransaction {
    #[serde(with = "ser_b64")]
    pub payment_id: PaymentId,
    /// A response (if we got one):
    pub opt_response: Option<ResponseSendFundsOp>,
}

#[allow(clippy::large_enum_variant)]
#[derive(Arbitrary, Debug, Clone)]
pub enum FunderMutation<B: Clone> {
    FriendMutation((PublicKey, FriendMutation<B>)),
    AddRelay(NamedRelayAddress<B>),
    RemoveRelay(PublicKey),
    AddFriend(AddFriend<B>),
    RemoveFriend(PublicKey),
    AddInvoice((InvoiceId, Currency, u128, PlainLock)), // (invoice_id, currency, total_dest_payment, dest_plain_lock)
    AddIncomingTransaction((InvoiceId, Uid)),           // (invoice_id, request_id)
    SetInvoiceSrcHashedLock((InvoiceId, HashedLock)),   // (invoice_id, src_hashed_lock)
    RemoveInvoice(InvoiceId),
    AddTransaction((Uid, PaymentId)), // (request_id, payment_id)
    SetTransactionResponse(ResponseSendFundsOp), // (request_id, response_send_funds)
    RemoveTransaction(Uid),           // request_id
    UpdatePayment((PaymentId, Payment)),
    RemovePayment(PaymentId),
}

impl<B> FunderState<B>
where
    B: Clone + CanonicalSerialize,
{
    pub fn new(local_public_key: PublicKey, relays: Vec<NamedRelayAddress<B>>) -> Self {
        // Convert relays into a map:
        let relays = relays.into_iter().collect();

        FunderState {
            local_public_key,
            relays,
            friends: ImHashMap::new(),
            open_invoices: ImHashMap::new(),
            open_transactions: ImHashMap::new(),
            payments: ImHashMap::new(),
        }
    }

    // TODO: Use MutableState trait instead:
    pub fn mutate(&mut self, funder_mutation: &FunderMutation<B>) {
        match funder_mutation {
            FunderMutation::FriendMutation((public_key, friend_mutation)) => {
                let friend = self.friends.get_mut(&public_key).unwrap();
                friend.mutate(friend_mutation);
            }
            FunderMutation::AddRelay(named_relay_address) => {
                // Check for duplicates:
                self.relays.retain(|cur_named_relay_address| {
                    cur_named_relay_address.public_key != named_relay_address.public_key
                });
                self.relays.push_back(named_relay_address.clone());
                // TODO: Should check here if we have more than a constant amount of relays
            }
            FunderMutation::RemoveRelay(public_key) => {
                self.relays.retain(|cur_named_relay_address| {
                    &cur_named_relay_address.public_key != public_key
                });
            }
            FunderMutation::AddFriend(add_friend) => {
                let friend = FriendState::new(
                    &self.local_public_key,
                    &add_friend.friend_public_key,
                    add_friend.relays.clone(),
                    add_friend.name.clone(),
                );
                // Insert friend, but also make sure that we didn't override an existing friend
                // with the same public key:
                let res = self
                    .friends
                    .insert(add_friend.friend_public_key.clone(), friend);
                assert!(res.is_none());
            }
            FunderMutation::RemoveFriend(public_key) => {
                let _ = self.friends.remove(&public_key);
            }
            FunderMutation::AddInvoice((
                invoice_id,
                currency,
                total_dest_payment,
                dest_plain_lock,
            )) => {
                self.open_invoices.insert(
                    invoice_id.clone(),
                    OpenInvoice::new(
                        currency.clone(),
                        *total_dest_payment,
                        dest_plain_lock.clone(),
                    ),
                );
            }
            FunderMutation::AddIncomingTransaction((invoice_id, request_id)) => {
                let open_invoice = self.open_invoices.get_mut(invoice_id).unwrap();
                open_invoice
                    .incoming_transactions
                    .insert(request_id.clone());
            }
            FunderMutation::SetInvoiceSrcHashedLock((invoice_id, src_hashed_lock)) => {
                let open_invoice = self.open_invoices.get_mut(invoice_id).unwrap();
                assert!(open_invoice.opt_src_hashed_lock.is_none());
                open_invoice.opt_src_hashed_lock = Some(src_hashed_lock.clone());
            }
            FunderMutation::RemoveInvoice(invoice_id) => {
                let _ = self.open_invoices.remove(invoice_id);
            }
            FunderMutation::AddTransaction((request_id, payment_id)) => {
                let open_transaction = OpenTransaction {
                    payment_id: payment_id.clone(),
                    opt_response: None,
                };
                let _ = self
                    .open_transactions
                    .insert(request_id.clone(), open_transaction);
            }
            FunderMutation::SetTransactionResponse(response_send_funds) => {
                let open_transaction = self
                    .open_transactions
                    .get_mut(&response_send_funds.request_id)
                    .unwrap();
                // We assert that no response was received so far:
                assert!(open_transaction.opt_response.take().is_none());
                open_transaction.opt_response = Some(response_send_funds.clone());
            }
            FunderMutation::RemoveTransaction(request_id) => {
                let _ = self.open_transactions.remove(request_id);
            }
            FunderMutation::UpdatePayment((payment_id, payment)) => {
                let _ = self.payments.insert(payment_id.clone(), payment.clone());
            }
            FunderMutation::RemovePayment(payment_id) => {
                let _ = self.payments.remove(payment_id);
            }
        }
    }
}
