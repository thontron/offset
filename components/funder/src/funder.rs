use futures::channel::mpsc;
use futures::{future, stream, SinkExt, StreamExt};

use crypto::crypto_rand::CryptoRandom;
use identity::IdentityClient;

// use crate::database::{AtomicDb, DbRunner, DbRunnerError};
use database::DatabaseClient;

use proto::funder::messages::{FunderIncomingControl, 
    FunderOutgoingControl};
use proto::funder::scheme::FunderScheme;

use crate::ephemeral::Ephemeral;
use crate::handler::{funder_handle_message};
use crate::types::{FunderIncoming, FunderOutgoingComm, FunderIncomingComm};
use crate::state::{FunderState, FunderMutation};


#[derive(Debug)]
pub enum FunderError {
    IncomingControlClosed,
    IncomingCommClosed,
    IncomingMessagesError,
    DbError,
    SendControlError,
    SendCommError,
}


#[derive(Debug, Clone)]
pub enum FunderEvent<FS: FunderScheme> {
    FunderIncoming(FunderIncoming<FS>),
    IncomingControlClosed,
    IncomingCommClosed,
}

pub async fn inner_funder_loop<FS,R>(
    mut identity_client: IdentityClient,
    rng: R,
    incoming_control: mpsc::Receiver<FunderIncomingControl<FS>>,
    incoming_comm: mpsc::Receiver<FunderIncomingComm<FS>>,
    control_sender: mpsc::Sender<FunderOutgoingControl<FS>>,
    comm_sender: mpsc::Sender<FunderOutgoingComm<FS>>,
    mut funder_state: FunderState<FS>,
    mut db_client: DatabaseClient<FunderMutation<FS>>,
    max_operations_in_batch: usize,
    max_pending_user_requests: usize,
    mut opt_event_sender: Option<mpsc::Sender<FunderEvent<FS>>>) -> Result<(), FunderError> 

where
    FS: FunderScheme,
    R: CryptoRandom + 'static,
{

    // Transform error type:
    let mut comm_sender = comm_sender.sink_map_err(|_| ());
    let mut control_sender = control_sender.sink_map_err(|_| ());

    // let mut db_runner = DbRunner::new(atomic_db);
    let mut ephemeral = Ephemeral::new();

    // Select over all possible events:
    let incoming_control = incoming_control
        .map(|incoming_control_msg| FunderEvent::FunderIncoming(FunderIncoming::Control(incoming_control_msg)))
        .chain(stream::once(future::ready(FunderEvent::IncomingControlClosed)));
    let incoming_comm = incoming_comm
        .map(|incoming_comm_msg| FunderEvent::FunderIncoming(FunderIncoming::Comm(incoming_comm_msg)))
        .chain(stream::once(future::ready(FunderEvent::IncomingCommClosed)));
    // Chain the Init message first:
    let mut incoming_messages = stream::once(future::ready(FunderEvent::FunderIncoming(FunderIncoming::Init)))
        .chain(incoming_control.select(incoming_comm));

    while let Some(funder_event) = await!(incoming_messages.next()) {
        // For testing:
        // Read one message from incoming messages:
        let funder_incoming = match funder_event.clone() {
            FunderEvent::IncomingControlClosed => return Err(FunderError::IncomingControlClosed),
            FunderEvent::IncomingCommClosed => return Err(FunderError::IncomingCommClosed),
            FunderEvent::FunderIncoming(funder_incoming) => funder_incoming,
        }; 

        let res = await!(funder_handle_message(&mut identity_client,
                              &rng,
                              funder_state.clone(),
                              ephemeral.clone(),
                              max_operations_in_batch,
                              max_pending_user_requests,
                              funder_incoming));


        let handler_output = match res {
            Ok(handler_output) => handler_output,
            Err(handler_error) => {
                // Reporting a recoverable error:
                error!("Funder handler error: {:?}", handler_error);
                continue;
            },
        };

        if !handler_output.funder_mutations.is_empty() {
            // Mutate our funder_state in memory:
            for mutation in &handler_output.funder_mutations {
                funder_state.mutate(mutation);
            }
            // If there are any mutations, send them to the database:
            await!(db_client.mutate(handler_output.funder_mutations))
                .map_err(|_| FunderError::DbError)?;
        }
        
        // Apply ephemeral mutations to our ephemeral:
        for mutation in &handler_output.ephemeral_mutations {
            ephemeral.mutate(mutation);
        }

        // Send outgoing communication messages:
        let mut comm_stream = stream::iter::<_>(handler_output.outgoing_comms);
        await!(comm_sender.send_all(&mut comm_stream))
            .map_err(|_| FunderError::SendCommError)?;

        // Send outgoing control messages:
        let mut control_stream = stream::iter::<_>(handler_output.outgoing_control);
        await!(control_sender.send_all(&mut control_stream))
            .map_err(|_| FunderError::SendControlError)?;


        if let Some(ref mut event_sender) = opt_event_sender {
            await!(event_sender.send(funder_event)).unwrap();
        }

    }
    // TODO: Do we ever really get here?
    Ok(())
}

pub async fn funder_loop<FS,R>(
    identity_client: IdentityClient,
    rng: R,
    incoming_control: mpsc::Receiver<FunderIncomingControl<FS>>,
    incoming_comm: mpsc::Receiver<FunderIncomingComm<FS>>,
    control_sender: mpsc::Sender<FunderOutgoingControl<FS>>,
    comm_sender: mpsc::Sender<FunderOutgoingComm<FS>>,
    max_operations_in_batch: usize,
    max_pending_user_requests: usize,
    funder_state: FunderState<FS>,
    db_client: DatabaseClient<FunderMutation<FS>>) -> Result<(), FunderError> 
where
    FS: FunderScheme,
    R: CryptoRandom + 'static,
{

    await!(inner_funder_loop(identity_client,
           rng,
           incoming_control,
           incoming_comm,
           control_sender,
           comm_sender,
           funder_state,
           db_client,
           max_operations_in_batch,
           max_pending_user_requests,
           None))

}


