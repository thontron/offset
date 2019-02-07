use capnp;
use capnp::serialize_packed;
use crypto::identity::PublicKey;
use common::int_convert::usize_to_u32;
use crate::capnp_common::{write_signature, read_signature,
                          write_custom_int128, read_custom_int128,
                          write_custom_u_int128, read_custom_u_int128,
                          write_rand_nonce, read_rand_nonce,
                          write_uid, read_uid,
                          write_invoice_id, read_invoice_id,
                          write_public_key, read_public_key,
                          write_relay_address, read_relay_address,
                          write_index_server_address, read_index_server_address,
                          write_receipt, read_receipt,
                          write_hash, read_hash};

use report_capnp;
use crate::serialize::SerializeError;
use crate::report::messages::{MoveTokenHashedReport, FriendStatusReport, RequestsStatusReport,
                            FriendLivenessReport, DirectionReport,
                            McRequestsStatusReport};


fn ser_move_token_hashed_report(move_token_hashed_report: &MoveTokenHashedReport,
                    move_token_hashed_report_builder: &mut report_capnp::move_token_hashed_report::Builder) {

    write_hash(&move_token_hashed_report.prefix_hash,
        &mut move_token_hashed_report_builder.reborrow().init_prefix_hash());

    write_public_key(&move_token_hashed_report.local_public_key,
        &mut move_token_hashed_report_builder.reborrow().init_local_public_key());

    write_public_key(&move_token_hashed_report.remote_public_key,
        &mut move_token_hashed_report_builder.reborrow().init_remote_public_key());

    move_token_hashed_report_builder.reborrow().set_inconsistency_counter(move_token_hashed_report.inconsistency_counter);

    write_custom_u_int128(move_token_hashed_report.move_token_counter,
        &mut move_token_hashed_report_builder.reborrow().init_move_token_counter());

    write_custom_int128(move_token_hashed_report.balance,
        &mut move_token_hashed_report_builder.reborrow().init_balance());

    write_custom_u_int128(move_token_hashed_report.local_pending_debt,
        &mut move_token_hashed_report_builder.reborrow().init_local_pending_debt());

    write_custom_u_int128(move_token_hashed_report.remote_pending_debt,
        &mut move_token_hashed_report_builder.reborrow().init_remote_pending_debt());

    write_rand_nonce(&move_token_hashed_report.rand_nonce,
        &mut move_token_hashed_report_builder.reborrow().init_rand_nonce());

    write_signature(&move_token_hashed_report.new_token,
        &mut move_token_hashed_report_builder.reborrow().init_new_token());
}

fn deser_move_token_hashed_report(move_token_hashed_report_reader: &report_capnp::move_token_hashed_report::Reader)
    -> Result<MoveTokenHashedReport, SerializeError> {

    Ok(MoveTokenHashedReport {
        prefix_hash: read_hash(&move_token_hashed_report_reader.get_prefix_hash()?)?,
        local_public_key: read_public_key(&move_token_hashed_report_reader.get_local_public_key()?)?,
        remote_public_key: read_public_key(&move_token_hashed_report_reader.get_remote_public_key()?)?,
        inconsistency_counter: move_token_hashed_report_reader.get_inconsistency_counter(),
        move_token_counter: read_custom_u_int128(&move_token_hashed_report_reader.get_move_token_counter()?)?,
        balance: read_custom_int128(&move_token_hashed_report_reader.get_balance()?)?,
        local_pending_debt: read_custom_u_int128(&move_token_hashed_report_reader.get_local_pending_debt()?)?,
        remote_pending_debt: read_custom_u_int128(&move_token_hashed_report_reader.get_remote_pending_debt()?)?,
        rand_nonce: read_rand_nonce(&move_token_hashed_report_reader.get_rand_nonce()?)?,
        new_token: read_signature(&move_token_hashed_report_reader.get_new_token()?)?,
    })
}

fn ser_friend_status_report(friend_status_report: &FriendStatusReport,
                    friend_status_report_builder: &mut report_capnp::friend_status_report::Builder) {

    match friend_status_report {
        FriendStatusReport::Enabled => friend_status_report_builder.set_enabled(()),
        FriendStatusReport::Disabled => friend_status_report_builder.set_disabled(()),
    }
}

fn deser_friend_status_report(friend_status_report_reader: &report_capnp::friend_status_report::Reader)
    -> Result<FriendStatusReport, SerializeError> {

    Ok(match friend_status_report_reader.which()? {
        report_capnp::friend_status_report::Disabled(()) => FriendStatusReport::Disabled,
        report_capnp::friend_status_report::Enabled(()) => FriendStatusReport::Enabled,
    })
}

fn ser_requests_status_report(requests_status_report: &RequestsStatusReport,
                    requests_status_report_builder: &mut report_capnp::requests_status_report::Builder) {

    match requests_status_report {
        RequestsStatusReport::Closed => requests_status_report_builder.set_closed(()),
        RequestsStatusReport::Open => requests_status_report_builder.set_open(()),
    }
}

fn deser_requests_status_report(requests_status_report_reader: &report_capnp::requests_status_report::Reader)
    -> Result<RequestsStatusReport, SerializeError> {

    Ok(match requests_status_report_reader.which()? {
        report_capnp::requests_status_report::Closed(()) => RequestsStatusReport::Closed,
        report_capnp::requests_status_report::Open(()) => RequestsStatusReport::Open,
    })
}

fn ser_friend_liveness_report(friend_liveness_report: &FriendLivenessReport,
                    friend_liveness_report_builder: &mut report_capnp::friend_liveness_report::Builder) {

    match friend_liveness_report {
        FriendLivenessReport::Offline => friend_liveness_report_builder.set_offline(()),
        FriendLivenessReport::Online => friend_liveness_report_builder.set_online(()),
    }
}

fn deser_friend_liveness_report(friend_liveness_report_reader: &report_capnp::friend_liveness_report::Reader)
    -> Result<FriendLivenessReport, SerializeError> {

    Ok(match friend_liveness_report_reader.which()? {
        report_capnp::friend_liveness_report::Offline(()) => FriendLivenessReport::Offline,
        report_capnp::friend_liveness_report::Online(()) => FriendLivenessReport::Online,
    })
}

fn ser_direction_report(direction_report: &DirectionReport,
                    direction_report_builder: &mut report_capnp::direction_report::Builder) {

    match direction_report {
        DirectionReport::Incoming => direction_report_builder.set_incoming(()),
        DirectionReport::Outgoing => direction_report_builder.set_outgoing(()),
    }
}

fn deser_direction_report(direction_report_reader: &report_capnp::direction_report::Reader)
    -> Result<DirectionReport, SerializeError> {

    Ok(match direction_report_reader.which()? {
        report_capnp::direction_report::Incoming(()) => DirectionReport::Incoming,
        report_capnp::direction_report::Outgoing(()) => DirectionReport::Outgoing,
    })
}

fn ser_mc_requests_status_report(mc_requests_status_report: &McRequestsStatusReport,
                    mc_requests_status_report_builder: &mut report_capnp::mc_requests_status_report::Builder) {

    ser_requests_status_report(&mc_requests_status_report.local, 
            &mut mc_requests_status_report_builder.reborrow().init_local());

    ser_requests_status_report(&mc_requests_status_report.remote, 
            &mut mc_requests_status_report_builder.reborrow().init_remote());

}

fn deser_mc_requests_status_report(mc_requests_status_report: &report_capnp::mc_requests_status_report::Reader)
    -> Result<McRequestsStatusReport, SerializeError> {

    Ok(McRequestsStatusReport {
        local: deser_requests_status_report(&mc_requests_status_report.get_local()?)?,
        remote: deser_requests_status_report(&mc_requests_status_report.get_remote()?)?,
    })
}