// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::channel::mpsc;
use futures::StreamExt;
use js_sys::Uint8Array;
use nym_client_core::client::base_client::ClientOutput;
use nym_client_core::client::received_buffer::{
    ReceivedBufferMessage, ReconstructedMessagesReceiver,
};
use nym_service_providers_common::interface::Serializable;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use wasm_utils::{console_error, console_log};

pub(crate) struct ResponsePusher {
    reconstructed_receiver: ReconstructedMessagesReceiver,
    on_message: js_sys::Function,
}

impl ResponsePusher {
    pub(crate) fn new(
        client_output: ClientOutput,
        on_message: js_sys::Function,
        // on_mix_fetch_message: js_sys::Function,
    ) -> Self {
        // register our output
        let (reconstructed_sender, reconstructed_receiver) = mpsc::unbounded();

        // tell the buffer to start sending stuff to us
        client_output
            .received_buffer_request_sender
            .unbounded_send(ReceivedBufferMessage::ReceiverAnnounce(
                reconstructed_sender,
            ))
            .expect("the buffer request failed!");

        ResponsePusher {
            reconstructed_receiver,
            on_message,
        }
    }

    pub(crate) fn start(mut self) {
        spawn_local(async move {
            let this = JsValue::null();

            while let Some(reconstructed) = self.reconstructed_receiver.next().await {
                for reconstructed_msg in reconstructed {
                    let (msg, tag) = reconstructed_msg.into_inner();

                    if let Ok(socks5_response) =
                        nym_socks5_requests::Socks5Response::try_from_bytes(&msg)
                    {
                        if let Ok(mix_http_response) =
                            nym_http_requests::socks::decode_socks_response_as_http_response(
                                socks5_response,
                            )
                        {
                            console_log!("mix_fetch response {:?}", mix_http_response);
                        }
                    }

                    let msg_slice: &[u8] = &msg;
                    let array = Uint8Array::from(msg_slice);
                    let arg1 = JsValue::from(array);
                    let arg2 = JsValue::from(tag);
                    self.on_message
                        .call2(&this, &arg1, &arg2)
                        .expect("on binary message failed!");
                }
            }

            console_error!("we stopped receiving reconstructed messages!")
        })
    }
}
