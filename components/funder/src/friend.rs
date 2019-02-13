use im::vector::Vector;

use crypto::identity::PublicKey;

use common::safe_arithmetic::SafeUnsignedArithmetic;

use proto::funder::messages::{RequestSendFunds,
    ResponseSendFunds, FailureSendFunds, ResetTerms,
    FriendStatus, RequestsStatus, PendingRequest};
use proto::funder::scheme::FunderScheme;

use crate::token_channel::{TcMutation, TokenChannel};
use crate::types::MoveTokenHashed;



#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum ResponseOp {
    Response(ResponseSendFunds),
    UnsignedResponse(PendingRequest),
    Failure(FailureSendFunds),
    UnsignedFailure(PendingRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SentLocalAddress<FS:FunderScheme> {
    NeverSent,
    Transition((FS::NamedAddress, FS::NamedAddress)), // (last sent, before last sent)
    LastSent(FS::NamedAddress),
}

impl<FS:FunderScheme> SentLocalAddress<FS> {
    pub fn to_vec(&self) -> Vec<FS::Address> {
        match self {
            SentLocalAddress::NeverSent => Vec::new(),
            SentLocalAddress::Transition((last_address, prev_last_address)) =>
                vec![FS::anonymize_address(last_address.clone()), 
                     FS::anonymize_address(prev_last_address.clone())],
            SentLocalAddress::LastSent(last_address) =>
                vec![FS::anonymize_address(last_address.clone())],
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FriendMutation<FS:FunderScheme> {
    TcMutation(TcMutation<FS>),
    SetInconsistent(ChannelInconsistent),
    SetConsistent(TokenChannel<FS>),
    SetWantedRemoteMaxDebt(u128),
    SetWantedLocalRequestsStatus(RequestsStatus),
    PushBackPendingRequest(RequestSendFunds),
    PopFrontPendingRequest,
    PushBackPendingResponse(ResponseOp),
    PopFrontPendingResponse,
    PushBackPendingUserRequest(RequestSendFunds),
    PopFrontPendingUserRequest,
    SetStatus(FriendStatus),
    SetRemoteAddress(FS::Address),
    SetName(String),
    SetSentLocalAddress(SentLocalAddress<FS>),
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
pub struct ChannelInconsistent {
    pub opt_last_incoming_move_token: Option<MoveTokenHashed>,
    pub local_reset_terms: ResetTerms,
    pub opt_remote_reset_terms: Option<ResetTerms>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum ChannelStatus<FS:FunderScheme> {
    Inconsistent(ChannelInconsistent),
    Consistent(TokenChannel<FS>),
}

impl<FS:FunderScheme> ChannelStatus<FS> {
    pub fn get_last_incoming_move_token_hashed(&self) -> Option<MoveTokenHashed> {
        match &self {
            ChannelStatus::Inconsistent(channel_inconsistent) => 
                channel_inconsistent.opt_last_incoming_move_token.clone(),
            ChannelStatus::Consistent(token_channel) => 
                token_channel.get_last_incoming_move_token_hashed().cloned(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FriendState<FS:FunderScheme> {
    pub local_public_key: PublicKey,
    pub remote_public_key: PublicKey,
    pub remote_address: FS::Address, 
    pub sent_local_address: SentLocalAddress<FS>,
    pub name: String,
    pub channel_status: ChannelStatus<FS>,
    pub wanted_remote_max_debt: u128,
    pub wanted_local_requests_status: RequestsStatus,
    pub pending_requests: Vector<RequestSendFunds>,
    pub pending_responses: Vector<ResponseOp>,
    // Pending operations to be sent to the token channel.
    pub status: FriendStatus,
    pub pending_user_requests: Vector<RequestSendFunds>,
    // Request that the user has sent to this neighbor, 
    // but have not been processed yet. Bounded in size.
}


impl<FS: FunderScheme> FriendState<FS> {
    pub fn new(local_public_key: &PublicKey,
               remote_public_key: &PublicKey,
               remote_address: FS::Address,
               name: String,
               balance: i128) -> Self {

        let token_channel = TokenChannel::new(local_public_key, remote_public_key, balance);

        FriendState {
            local_public_key: local_public_key.clone(),
            remote_public_key: remote_public_key.clone(),
            remote_address,
            sent_local_address: SentLocalAddress::NeverSent,
            name,
            channel_status: ChannelStatus::Consistent(token_channel),

            // The remote_max_debt we want to have. When possible, this will be sent to the remote
            // side.
            wanted_remote_max_debt: 0,
            wanted_local_requests_status: RequestsStatus::Closed,
            // The local_send_price we want to have (Or possibly close requests, by having an empty
            // send price). When possible, this will be updated with the TokenChannel.
            pending_requests: Vector::new(),
            pending_responses: Vector::new(),
            status: FriendStatus::Disabled,
            pending_user_requests: Vector::new(),
        }
    }

    // TODO: Do we use this function somewhere?
    /// Find the shared credits we have with this friend.
    /// This value is used for freeze guard calculations.
    /// This value is the capacity shared between the rest of the friends.
    ///
    /// ```text
    ///         ---B
    ///        /
    /// A--*--O-----C
    ///        \
    ///         ---D
    /// ```
    /// In the picture above, the shared credits between O and A will be shared between the nodes
    /// B, C and D.
    ///
    pub fn get_shared_credits(&self) -> u128 {
        let balance = match &self.channel_status {
            ChannelStatus::Consistent(token_channel) =>
                &token_channel.get_mutual_credit().state().balance,
            ChannelStatus::Inconsistent(_channel_inconsistent) => return 0,
        };
        balance.local_max_debt.saturating_add_signed(balance.balance)
    }

    pub fn mutate(&mut self, friend_mutation: &FriendMutation<FS>) {
        match friend_mutation {
            FriendMutation::TcMutation(tc_mutation) => {
                match &mut self.channel_status {
                    ChannelStatus::Consistent(ref mut token_channel) =>
                        token_channel.mutate(tc_mutation),
                    ChannelStatus::Inconsistent(_) => unreachable!(),
                }
            },
            FriendMutation::SetInconsistent(channel_inconsistent) => {
                self.channel_status = ChannelStatus::Inconsistent(channel_inconsistent.clone());
            },
            FriendMutation::SetConsistent(token_channel) => {
                self.channel_status = ChannelStatus::Consistent(token_channel.clone());
            },
            FriendMutation::SetWantedRemoteMaxDebt(wanted_remote_max_debt) => {
                self.wanted_remote_max_debt = *wanted_remote_max_debt;
            },
            FriendMutation::SetWantedLocalRequestsStatus(wanted_local_requests_status) => {
                self.wanted_local_requests_status = wanted_local_requests_status.clone();
            },
            FriendMutation::PushBackPendingRequest(request_send_funds) => {
                self.pending_requests.push_back(request_send_funds.clone());
            },
            FriendMutation::PopFrontPendingRequest => {
                let _ = self.pending_requests.pop_front();
            },
            FriendMutation::PushBackPendingResponse(response_op) => {
                self.pending_responses.push_back(response_op.clone());
            },
            FriendMutation::PopFrontPendingResponse => {
                let _ = self.pending_responses.pop_front();
            },
            FriendMutation::PushBackPendingUserRequest(request_send_funds) => {
                self.pending_user_requests.push_back(request_send_funds.clone());
            },
            FriendMutation::PopFrontPendingUserRequest => {
                let _ = self.pending_user_requests.pop_front();
            },
            FriendMutation::SetStatus(friend_status) => {
                self.status = friend_status.clone();
            },
            FriendMutation::SetRemoteAddress(friend_addr) => {
                self.remote_address = friend_addr.clone();
            },
            FriendMutation::SetName(friend_name) => {
                self.name = friend_name.clone();
            },
            FriendMutation::SetSentLocalAddress(sent_local_address) => {
                self.sent_local_address = sent_local_address.clone();
            },
            /*
            FriendMutation::LocalReset(reset_move_token) => {
                // Local reset was applied (We sent a reset from the control line)
                match &self.channel_status {
                    ChannelStatus::Consistent(_) => unreachable!(),
                    ChannelStatus::Inconsistent(channel_inconsistent) => {
                        let ChannelInconsistent {
                            opt_last_incoming_move_token,
                            local_reset_terms,
                            opt_remote_reset_terms,
                        } = channel_inconsistent;

                        match opt_remote_reset_terms {
                            None => unreachable!(),
                            Some(remote_reset_terms) => {
                                assert_eq!(reset_move_token.old_token, remote_reset_terms.reset_token);
                                let token_channel = TokenChannel::new_from_local_reset(
                                    &self.local_public_key,
                                    &self.remote_public_key,
                                    &reset_move_token,
                                    remote_reset_terms.balance_for_reset.checked_neg().unwrap(),
                                    opt_last_incoming_move_token.clone());
                                self.channel_status = ChannelStatus::Consistent(token_channel);
                            },
                        }
                    },
                }
            },
            FriendMutation::RemoteReset(reset_move_token) => {
                // Remote reset was applied (Remote side has given a reset command)
                match &self.channel_status {
                    ChannelStatus::Consistent(_) => unreachable!(),
                    ChannelStatus::Inconsistent(channel_inconsistent) => {
                        let token_channel = TokenChannel::new_from_remote_reset(
                            &self.local_public_key,
                            &self.remote_public_key,
                            &reset_move_token,
                            channel_inconsistent.local_reset_terms.balance_for_reset);
                        self.channel_status = ChannelStatus::Consistent(token_channel);
                    },
                }
            },
            */
        }
    }
}
