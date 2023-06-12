// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mix_fetch::error::MixFetchError;
use crate::mix_fetch::mix_http_requests::http_request_to_mixnet_request_to_vec_u8;
use crate::mix_fetch::request_correlator::{ActiveRequests, Response};
use futures::channel::{mpsc, oneshot};
use futures::StreamExt;
use httpcodec::Request as HttpCodecRequest;
use js_sys::Promise;
use nym_client_core::client::base_client::{ClientInput, ClientOutput};
use nym_client_core::client::inbound_messages::InputMessage;
use nym_client_core::client::received_buffer::{
    ReceivedBufferMessage, ReconstructedMessagesReceiver,
};
use nym_http_requests::socks::MixHttpResponse;
use nym_service_providers_common::interface::Serializable;
use nym_sphinx::addressing::clients::Recipient;
use nym_task::connections::TransmissionLane;
use rand::{thread_rng, RngCore};
use std::sync::Arc;
use wasm_bindgen_futures::{future_to_promise, spawn_local};
use wasm_utils::{console_error, console_log, PromisableResult};

mod client;
mod config;
pub mod error;
pub mod mix_http_requests;
pub(crate) mod request_adapter;
mod request_correlator;

pub const MIX_FETCH_CLIENT_ID_PREFIX: &str = "mix-fetch-";

#[derive(Clone)]
struct Placeholder {
    fetch_provider: Recipient,
    client_input: Arc<ClientInput>,
    requests: ActiveRequests,
}

impl Placeholder {
    pub(crate) fn new(
        fetch_provider: Recipient,
        client_input: ClientInput,
        requests: ActiveRequests,
    ) -> Self {
        Placeholder {
            fetch_provider,
            client_input: Arc::new(client_input),
            requests,
        }
    }

    pub(crate) fn fetch(
        &self,
        connection_id: u64,
        local_closed: bool,
        ordered_message_index: u64,
        req: HttpCodecRequest<Vec<u8>>,
    ) -> Promise {
        let this = self.clone();
        future_to_promise(async move {
            this.fetch_async(connection_id, local_closed, ordered_message_index, req)
                .await
                .map(|res| {
                    console_log!("raw response: {:?}", res);
                    "placeholder return value".to_string()
                })
                .into_promise_result()
        })
    }

    pub(crate) async fn fetch_async(
        &self,
        connection_id: u64,
        local_closed: bool,
        ordered_message_index: u64,
        req: HttpCodecRequest<Vec<u8>>,
    ) -> Result<Response, MixFetchError> {
        let mut rng = thread_rng();
        let request_id = rng.next_u64();

        // TODO: regenerate id in case of collisions
        // (though realistically the chance is 1 in ~ 2^63 so do we have to bother?)
        let raw_request = match http_request_to_mixnet_request_to_vec_u8(
            connection_id,
            local_closed,
            ordered_message_index,
            req,
        ) {
            Ok(ok) => ok,
            Err(_) => {
                panic!("TODO: error handling");
            }
        };
        let lane = TransmissionLane::ConnectionId(request_id);
        let input = InputMessage::new_regular(self.fetch_provider, raw_request, lane, None);

        let (tx, rx) = oneshot::channel();
        self.requests.new(request_id, tx).await;

        self.client_input
            .input_sender
            .send(input)
            .await
            .expect("TODO: error handling");

        let res = rx.await.expect("TODO: error handling for closed channel");
        Ok(res)
    }
}

struct Placeholder2 {
    reconstructed_receiver: ReconstructedMessagesReceiver,
    requests: ActiveRequests,
}

impl Placeholder2 {
    pub(crate) fn new(client_output: ClientOutput, requests: ActiveRequests) -> Self {
        // register our output
        let (reconstructed_sender, reconstructed_receiver) = mpsc::unbounded();

        // tell the buffer to start sending stuff to us
        client_output
            .received_buffer_request_sender
            .unbounded_send(ReceivedBufferMessage::ReceiverAnnounce(
                reconstructed_sender,
            ))
            .expect("the buffer request failed!");

        Placeholder2 {
            reconstructed_receiver,
            requests,
        }
    }

    pub(crate) fn start(mut self) {
        spawn_local(async move {
            while let Some(reconstructed) = self.reconstructed_receiver.next().await {
                for reconstructed_msg in reconstructed {
                    let (msg, tag) = reconstructed_msg.into_inner();
                    if tag.is_some() {
                        panic!("TODO: error handling for set tag")
                    }

                    if let Ok(socks5_response) =
                        nym_socks5_requests::Socks5Response::try_from_bytes(&msg)
                    {
                        if let Ok(mix_http_response) = MixHttpResponse::try_from(socks5_response) {
                            console_log!("mix_fetch response {:?}", mix_http_response);

                            self.requests
                                .resolve(
                                    mix_http_response.connection_id,
                                    mix_http_response.http_response,
                                )
                                .await
                        }
                    } else {
                        panic!("TODO: error handling for receiving something that's not socks5 response")
                    }
                }
            }

            console_error!("we stopped receiving reconstructed messages!")
        })
    }
}
