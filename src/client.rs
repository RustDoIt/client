use anyhow::Result;
use common::network::NetworkError;
use common::{
    types::{NodeCommand, NodeEvent},
    FragmentAssembler, RoutingHandler,
};
use crossbeam_channel::{select_biased, Receiver, Sender};
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::Mutex,
    task::JoinHandle,
};
use wg_internal::{
    network::NodeId,
    packet::{NodeType, Packet, PacketType},
};

#[derive(Debug, Clone, PartialEq)]
pub enum ClientType {
    ChatClient,
    WebBrowser,
}

/// Events from client to simulation controller
#[derive(Debug)]
pub enum ClientEvent {
    PacketSent(Packet),
    ClientRegistered(String),
    MessageSent(NodeId, String), // (to, message)
}

pub type Data = Vec<u8>;

pub trait GenericClient {
    fn get_available_requests() -> Vec<String>;
    fn start(&mut self) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn send_request();
}

pub(crate) struct Client {
    id: NodeId,
    packet_recv: Option<Receiver<Packet>>,
    controller_recv: Option<Receiver<NodeCommand>>,
    pub routing_handler: Arc<Mutex<RoutingHandler>>,
    pending_messages: Arc<Mutex<Vec<Data>>>,
    pending_requests: Arc<Mutex<Vec<(NodeId, Data)>>>,
    received_message: Option<Data>,
}

impl Client {
    pub(crate) fn new(
        id: NodeId,
        neighbors: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        controller_recv: Receiver<NodeCommand>,
        controller_send: Sender<NodeEvent>,
        pending_messages: Arc<Mutex<Vec<Data>>>,
        pending_requests: Arc<Mutex<Vec<(NodeId, Data)>>>,
    ) -> Self {
        Self {
            id,
            packet_recv: Some(packet_recv),
            controller_recv: Some(controller_recv),
            routing_handler: Arc::new(Mutex::new(RoutingHandler::new(
                id,
                NodeType::Client,
                neighbors,
                controller_send,
            ))),
            pending_messages,
            pending_requests,
            received_message: None,
        }
    }

    pub(crate) fn run(mut self) -> Option<JoinHandle<Result<()>>> {
        let router = self.routing_handler.clone();
        let packet_recv = self.packet_recv.take()?;
        let controller_recv = self.controller_recv.take()?;
        let pending_messages = self.pending_messages.clone();

        Some(tokio::task::spawn(Client::process_packet(
            router,
            packet_recv,
            controller_recv,
            pending_messages,
        )))
    }

    async fn process_packet(
        routing_handler: Arc<Mutex<RoutingHandler>>,
        packet_recv: Receiver<Packet>,
        controller_recv: Receiver<NodeCommand>,
        pending_messages: Arc<Mutex<Vec<Data>>>,
    ) -> Result<()> {
        {
            routing_handler.lock().await.start_flood()?;
        }

        let mut assembler = FragmentAssembler::new();
        let mut received_message = None;

        loop {
            {
                let mut router = routing_handler.lock().await;
                select_biased! {
                    recv(controller_recv) -> cmd => {
                        if let Ok(cmd) = cmd {
                            match cmd {
                                NodeCommand::AddSender(node_id, sender) => router.add_neighbor(node_id, sender),
                                NodeCommand::RemoveSender(node_id) => router.remove_neighbor(node_id),
                                NodeCommand::Shutdown => return Ok(()),
                            }
                        }
                    },

                    recv(packet_recv) -> pkt => {

                        if let Ok(pkt) = pkt {
                            match pkt.pack_type {
                                PacketType::MsgFragment(fragment) => {
                                    received_message = assembler.add_fragment(
                                        fragment,
                                        pkt.session_id,
                                        pkt.routing_header.hops[0]
                                    );
                                },
                                PacketType::Ack(ack) => router.handle_ack(ack, pkt.session_id, pkt.routing_header.hops[0]),
                                PacketType::Nack(nack) => {
                                    if let Err(e) = router.handle_nack(nack, pkt.routing_header.hops[0]) {
                                        match e {
                                            NetworkError::ControllerDisconnected => return Err(anyhow::anyhow!("Controller disconnected")),
                                            NetworkError::PathNotFound(_) => router.start_flood()?,
                                            _ => {}
                                        }
                                    }
                                },
                                PacketType::FloodResponse(flood_response) => router.handle_flood_response(flood_response),
                                PacketType::FloodRequest(flood_request) => {
                                    if let Err(e) = router.handle_flood_request(flood_request, pkt.session_id) {
                                        match e {
                                            NetworkError::ControllerDisconnected => return Err(anyhow::anyhow!("Controller disconnected")),
                                            NetworkError::PathNotFound(_) => router.start_flood()?,
                                            _ => {}
                                        }
                                    }
                                },
                            }
                        }
                    }
                }
            }
            if let Some(message) = received_message.take() {
                pending_messages.lock().await.push(message);
            }
        }
    }
}
