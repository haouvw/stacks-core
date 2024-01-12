// Copyright (C) 2013-2020 Blockstack PBC, a public benefit corporation
// Copyright (C) 2020-2023 Stacks Open Internet Foundation
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

use blockstack_lib::chainstate::nakamoto::NakamotoBlock;
use blockstack_lib::chainstate::stacks::boot::MINERS_NAME;
use blockstack_lib::net::api::postblock_proposal::BlockValidateResponse;
use blockstack_lib::net::api::poststackerdbchunk::StackerDBChunksEvent;
use blockstack_lib::util_lib::boot::boot_code_id;
use clarity::vm::types::QualifiedContractIdentifier;
use serde::{Deserialize, Serialize};
use stacks_common::codec::{
    read_next, read_next_at_most, write_next, Error as CodecError, StacksMessageCodec,
};
use tiny_http::{
    Method as HttpMethod, Request as HttpRequest, Response as HttpResponse, Server as HttpServer,
};
use wsts::net::{Message, Packet};

use crate::http::{decode_http_body, decode_http_request};
use crate::EventError;

/// Event enum for newly-arrived signer subscribed events
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SignerEvent {
    /// A new stackerDB chunk was received
    StackerDB(StackerDBChunksEvent),
    /// A new block proposal was received
    BlockProposal(BlockValidateResponse),
}

/// Trait to implement a stop-signaler for the event receiver thread.
/// The caller calls `send()` and the event receiver loop (which lives in a separate thread) will
/// terminate.
pub trait EventStopSignaler {
    /// Send the stop signal
    fn send(&mut self);
}

/// Trait to implement to handle StackerDB and BlockProposal events sent by the Stacks node
pub trait EventReceiver {
    /// The implementation of ST will ensure that a call to ST::send() will cause
    /// the call to `is_stopped()` below to return true.
    type ST: EventStopSignaler + Send + Sync;

    /// Open a server socket to the given socket address.
    fn bind(&mut self, listener: SocketAddr) -> Result<SocketAddr, EventError>;
    /// Return the next event
    fn next_event(&mut self) -> Result<SignerEvent, EventError>;
    /// Add a downstream event consumer
    fn add_consumer(&mut self, event_out: Sender<SignerEvent>);
    /// Forward the event to downstream consumers
    fn forward_event(&mut self, ev: SignerEvent) -> bool;
    /// Determine if the receiver should hang up
    fn is_stopped(&self) -> bool;
    /// Get a stop signal instance that, when sent, will cause this receiver to stop accepting new
    /// events.  Called after `bind()`.
    fn get_stop_signaler(&mut self) -> Result<Self::ST, EventError>;

    /// Main loop for the receiver.
    /// Typically, this is started in a separate thread.
    fn main_loop(&mut self) {
        loop {
            if self.is_stopped() {
                info!("Event receiver stopped");
                break;
            }
            let next_event = match self.next_event() {
                Ok(event) => event,
                Err(EventError::UnrecognizedEvent(..)) => {
                    // got an event that we don't care about (not a problem)
                    continue;
                }
                Err(EventError::Terminated) => {
                    // we're done
                    info!("Caught termination signal");
                    break;
                }
                Err(e) => {
                    warn!("Failed to receive next event: {:?}", &e);
                    continue;
                }
            };
            if !self.forward_event(next_event) {
                info!("Failed to forward event");
                break;
            }
        }
        info!("Event receiver main loop exit");
    }
}

/// Event receiver for Signer events
pub struct SignerEventReceiver {
    /// stacker db contracts we're listening for
    pub stackerdb_contract_ids: Vec<QualifiedContractIdentifier>,
    /// Address we bind to
    local_addr: Option<SocketAddr>,
    /// server socket that listens for HTTP POSTs from the node
    http_server: Option<HttpServer>,
    /// channel into which to write newly-discovered data
    out_channels: Vec<Sender<SignerEvent>>,
    /// inter-thread stop variable -- if set to true, then the `main_loop` will exit
    stop_signal: Arc<AtomicBool>,
}

impl SignerEventReceiver {
    /// Make a new Signer event receiver, and return both the receiver and the read end of a
    /// channel into which node-received data can be obtained.
    pub fn new(contract_ids: Vec<QualifiedContractIdentifier>) -> SignerEventReceiver {
        SignerEventReceiver {
            stackerdb_contract_ids: contract_ids,
            http_server: None,
            local_addr: None,
            out_channels: vec![],
            stop_signal: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Do something with the socket
    pub fn with_server<F, R>(&mut self, todo: F) -> Result<R, EventError>
    where
        F: FnOnce(&SignerEventReceiver, &mut HttpServer, &[QualifiedContractIdentifier]) -> R,
    {
        let mut server = if let Some(s) = self.http_server.take() {
            s
        } else {
            return Err(EventError::NotBound);
        };

        let res = todo(self, &mut server, &self.stackerdb_contract_ids);

        self.http_server = Some(server);
        Ok(res)
    }
}

/// Stop signaler implementation
pub struct SignerStopSignaler {
    stop_signal: Arc<AtomicBool>,
    local_addr: SocketAddr,
}

impl SignerStopSignaler {
    /// Make a new stop signaler
    pub fn new(sig: Arc<AtomicBool>, local_addr: SocketAddr) -> SignerStopSignaler {
        SignerStopSignaler {
            stop_signal: sig,
            local_addr,
        }
    }
}

impl EventStopSignaler for SignerStopSignaler {
    fn send(&mut self) {
        self.stop_signal.store(true, Ordering::SeqCst);
        // wake up the thread so the atomicbool can be checked
        // This makes me sad...but for now...it works.
        if let Ok(mut stream) = TcpStream::connect(self.local_addr) {
            // We need to send actual data to trigger the event receiver
            let body = "Yo. Shut this shit down!".to_string();
            let req = format!(
                "POST /shutdown HTTP/1.0\r\nContent-Length: {}\r\n\r\n{}",
                &body.len(),
                body
            );
            stream.write_all(req.as_bytes()).unwrap();
        }
    }
}

impl EventReceiver for SignerEventReceiver {
    type ST = SignerStopSignaler;

    /// Start listening on the given socket address.
    /// Returns the address that was bound.
    /// Errors out if bind(2) fails
    fn bind(&mut self, listener: SocketAddr) -> Result<SocketAddr, EventError> {
        self.http_server = Some(HttpServer::http(listener).expect("failed to start HttpServer"));
        self.local_addr = Some(listener);
        Ok(listener)
    }

    /// Wait for the node to post something, and then return it.
    /// Errors are recoverable -- the caller should call this method again even if it returns an
    /// error.
    fn next_event(&mut self) -> Result<SignerEvent, EventError> {
        self.with_server(|event_receiver, http_server, contract_ids| {
            let mut request = http_server.recv()?;

            // were we asked to terminate?
            if event_receiver.is_stopped() {
                return Err(EventError::Terminated);
            }

            if request.method() != &HttpMethod::Post {
                return Err(EventError::MalformedRequest(format!(
                    "Unrecognized method '{}'",
                    &request.method(),
                )));
            }
            if request.url() == "/stackerdb_chunks" {
                debug!("Got stackerdb_chunks event");
                let mut body = String::new();
                if let Err(e) = request
                    .as_reader()
                    .read_to_string(&mut body) {
                    error!("Failed to read body: {:?}", &e);

                    request
                        .respond(HttpResponse::empty(200u16))
                        .expect("response failed");
                    return Err(EventError::MalformedRequest(format!(
                        "Failed to read body: {:?}",
                        &e
                    )));
                    }

                let event: StackerDBChunksEvent =
                    serde_json::from_slice(body.as_bytes()).map_err(|e| {
                        EventError::Deserialize(format!("Could not decode body to JSON: {:?}", &e))
                    })?;

                if !contract_ids.contains(&event.contract_id) {
                    info!(
                        "[{:?}] next_event got event from an unexpected contract id {}, return OK so other side doesn't keep sending this",
                        event_receiver.local_addr,
                        event.contract_id
                    );
                    request
                        .respond(HttpResponse::empty(200u16))
                        .expect("response failed");
                    return Err(EventError::UnrecognizedStackerDBContract(event.contract_id));
                }

                request
                    .respond(HttpResponse::empty(200u16))
                    .expect("response failed");

                Ok(SignerEvent::StackerDB(event))
            } else if request.url() == "/proposal_response" {
                debug!("Got proposal_response event");
                let mut body = String::new();
                if let Err(e) = request
                    .as_reader()
                    .read_to_string(&mut body) {
                    error!("Failed to read body: {:?}", &e);

                    request
                        .respond(HttpResponse::empty(200u16))
                        .expect("response failed");
                    return Err(EventError::MalformedRequest(format!(
                        "Failed to read body: {:?}",
                        &e
                    )));
                    }

                let event: BlockValidateResponse =
                    serde_json::from_slice(body.as_bytes()).map_err(|e| {
                        EventError::Deserialize(format!("Could not decode body to JSON: {:?}", &e))
                    })?;

                request
                    .respond(HttpResponse::empty(200u16))
                    .expect("response failed");

                Ok(SignerEvent::BlockProposal(event))
            } else {
                let url = request.url().to_string();

                info!(
                    "[{:?}] next_event got request with unexpected url {}, return OK so other side doesn't keep sending this",
                    event_receiver.local_addr,
                    request.url()
                );

                request
                    .respond(HttpResponse::empty(200u16))
                    .expect("response failed");
                Err(EventError::UnrecognizedEvent(url))
            }
        })?
    }

    /// Determine if the receiver is hung up
    fn is_stopped(&self) -> bool {
        self.stop_signal.load(Ordering::SeqCst)
    }

    /// Forward an event
    /// Return true on success; false on error.
    /// Returning false terminates the event receiver.
    fn forward_event(&mut self, ev: SignerEvent) -> bool {
        if self.out_channels.is_empty() {
            // nothing to do
            error!("No channels connected to event receiver");
            false
        } else if self.out_channels.len() == 1 {
            // avoid a clone
            if let Err(e) = self.out_channels[0].send(ev) {
                error!("Failed to send to signer runloop: {:?}", &e);
                return false;
            }
            true
        } else {
            for (i, out_channel) in self.out_channels.iter().enumerate() {
                if let Err(e) = out_channel.send(ev.clone()) {
                    error!("Failed to send to signer runloop #{}: {:?}", i, &e);
                    return false;
                }
            }
            true
        }
    }

    /// Add an event consumer.  A received event will be forwarded to this Sender.
    fn add_consumer(&mut self, out_channel: Sender<SignerEvent>) {
        self.out_channels.push(out_channel);
    }

    /// Get a stopped signaler.  The caller can then use it to terminate the event receiver loop,
    /// even if it's in a different thread.
    fn get_stop_signaler(&mut self) -> Result<SignerStopSignaler, EventError> {
        if let Some(local_addr) = self.local_addr {
            Ok(SignerStopSignaler::new(
                self.stop_signal.clone(),
                local_addr,
            ))
        } else {
            Err(EventError::NotBound)
        }
    }
}
