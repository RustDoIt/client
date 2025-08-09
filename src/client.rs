use std::collections::HashMap;
use common::{types::{NodeCommand, NodeEvent}, routing_handler::RoutingHandler};
use crossbeam_channel::{select_biased, Receiver, Sender};
use wg_internal::{network::NodeId, packet::{Nack, NackType, NodeType, Packet, PacketType}};
use crate::errors::ClientError;


#[derive(Debug)]
pub enum ClientType {
    ChatClient,
    WebBrowser
}

/// Events from client to simulation controller
#[derive(Debug)]
pub enum ClientEvent {
    PacketSent(Packet),
    ClientRegistered(String),
    MessageSent(NodeId, String), // (to, message)
}


pub(crate) struct Client {
    id: NodeId,
    packet_recv: Receiver<Packet>,
    controller_recv: Receiver<NodeCommand>,
    routing_handler: RoutingHandler
}


impl Client {
    pub(crate) fn new(
        id: NodeId,
        neighbors: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        controller_recv: Receiver<NodeCommand>,
        controller_send: Sender<NodeEvent>,
    ) -> Self {
        Self {
            id,
            packet_recv,
            controller_recv,
            routing_handler: RoutingHandler::new(id, NodeType::Client, neighbors, controller_send)
        }
    }

    pub(crate) fn start(&mut self) -> Result<(), ClientError>{
        // start first discovery
        self.routing_handler.start_flood().map_err(|_e| ClientError::NetworkError(("Cannot Start flood").to_string()))?;

        loop {
            select_biased! {
                recv(self.controller_recv) -> command => {
                    if let Ok(command) = command {
                        // handle command
                        match command {
                            NodeCommand::AddSender(node_id, sender) => {
                                let _ = self.routing_handler.add_neighbor(node_id, sender);
                            },
                            NodeCommand::RemoveSender(node_id) => {
                                let _ = self.routing_handler.remove_neighbor(node_id);
                            },
                            NodeCommand::Shutdown => break,
                        }
                    }
                },

                recv(self.packet_recv) -> packet => {
                    if let Ok(packet) = packet {
                        // handle packet
                        match packet.pack_type {
                            PacketType::MsgFragment(fragment) => todo!(),
                            PacketType::Ack(ack) => todo!(),
                            PacketType::Nack(nack) => self.handle_nack(nack),
                            PacketType::FloodRequest(flood_request) => {
                                let _ = self.routing_handler.handle_flood_request(flood_request, packet.session_id);
                            },
                            PacketType::FloodResponse(flood_response) => {
                                let _ = self.routing_handler.handle_flood_response(flood_response);
                            },
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_nack(&mut self, nack: Nack) {
        match nack.nack_type {
            NackType::ErrorInRouting(_node_id) => {
                // Drone crashed --> forward to routing handler
                self.routing_handler.start_flood();
            },
            NackType::DestinationIsDrone => todo!(),
            NackType::Dropped => todo!(),
            NackType::UnexpectedRecipient(_) => todo!(),
        }
    }
}


pub struct ChatClient {
    client: Client,
    received_messages: Vec<(String, String)>,
    communication_server: Vec<NodeId>
}


impl ChatClient {
    pub fn new(
        id: NodeId,
        neighbors: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        controller_recv: Receiver<NodeCommand>,
        controller_send: Sender<NodeEvent>,
    ) -> Self {
        Self {
            client: Client::new(id, neighbors, packet_recv, controller_recv, controller_send),
            received_messages: vec![],
            communication_server: vec![]
        }
    }
}
