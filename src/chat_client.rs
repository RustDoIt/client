use std::{collections::HashMap, sync::Arc};
use anyhow::Result;
use common::RoutingHandler;
use common::types::{NodeCommand, NodeEvent};
use crossbeam_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex, task::JoinHandle};
use wg_internal::{network::NodeId, packet::Packet};

use crate::client::{Client, Data, GenericClient};

// Chat Protocol Messages according to AP-protocol.md
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "message_type")]
pub enum ChatRequest {
    #[serde(rename = "server_type?")]
    ServerTypeQuery,

    #[serde(rename = "registration_to_chat")]
    RegistrationToChat { client_id: NodeId },

    #[serde(rename = "client_list?")]
    ClientListQuery,

    #[serde(rename = "message_for?")]
    MessageFor { client_id: NodeId, message: String },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "response_type")]
pub enum ChatResponse {
    #[serde(rename = "server_type!")]
    ServerTypeResponse { server_type: String },

    #[serde(rename = "client_list!")]
    ClientListResponse { list_of_client_ids: Vec<NodeId> },

    #[serde(rename = "message_from!")]
    MessageFrom { client_id: NodeId, message: String },

    #[serde(rename = "error_wrong_client_id!")]
    ErrorWrongClientId,

    // Custom response for successful registration
    #[serde(rename = "registration_success")]
    RegistrationSuccess,
}

pub struct ChatClient {
    client: Option<Client>,
    routing_handler: Arc<Mutex<RoutingHandler>>,
    received_messages: Vec<(String, String)>,
    communication_server: Vec<NodeId>,
    packet_processor: Option<JoinHandle<Result<()>>>
}

impl ChatClient {
    pub fn new(
        id: NodeId,
        neighbors: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        controller_recv: Receiver<NodeCommand>,
        controller_send: Sender<NodeEvent>,
    ) -> Self {
        let pending_messages = Arc::new(Mutex::new(Vec::<Data>::new()));
        let pending_requests = Arc::new(Mutex::new(Vec::<(NodeId, Data)>::new()));
        let client= Client::new(
            id,
            neighbors,
            packet_recv,
            controller_recv,
            controller_send,
            pending_messages,
            pending_requests,
        );
        let routing_handler = client.routing_handler.clone();

        Self {
            client: Some(client),
            routing_handler,
            received_messages: vec![],
            communication_server: vec![],
            packet_processor: None

        }
    }
}
impl GenericClient for ChatClient {
    fn get_available_requests() -> Vec<String> {
        todo!()
    }

    fn start(&mut self) -> Result<()> {
        if let Some(client) = self.client.take() {
            if let Some(processor) = client.run() {
                self.packet_processor = Some(processor);
            } else {
                return Err(anyhow::anyhow!("Failed to start client"));
            }
        }
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        if let Some(processor) = self.packet_processor.take() {
            processor.abort();
        }
        Ok(())
    }

    fn send_request() {
        todo!()
    }
}
