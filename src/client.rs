use wg_internal::network:: NodeId ;
use std::sync::Arc;
use wg_internal::packet::{ NackType, NodeType, Packet, PacketType};
use crossbeam_channel::{ Sender, Receiver };
use std::collections::HashMap;
use common::network::{Network, Node};
use common::types::{PendingQueue, SendingMap, NodeEvent, NodeCommand};
use tokio::sync::{Mutex, RwLock};
use crossbeam_channel::select_biased;


pub struct Client {
    neighbors: SendingMap,
    id: NodeId,
    network_view: Arc<RwLock<Network>>,
    last_discovery: u64,
    flood_id: u64,
    session_id: u64,
    pending: PendingQueue,
    controller_send: Sender<NodeEvent>,
    controller_recv: Receiver<NodeCommand>,
    packet_recv: Receiver<Packet>,
}


impl Client {
    pub fn new(id: NodeId, controller_send: Sender<NodeEvent>, controller_recv: Receiver<NodeCommand>, packet_recv: Receiver<Packet>) -> Self {
        let pending = Arc::new(Mutex::new(HashMap::new()));
        Self {
            id,
            neighbors: Arc::new(RwLock::new(HashMap::new())),
            network_view: Arc::new(RwLock::new(Network::new(Node::new(id, NodeType::Client, vec![])))),
            session_id: 0,
            last_discovery: 0,
            flood_id: 0,
            pending,
            controller_send,
            controller_recv,
            packet_recv
        }
    }

    pub async fn run(&mut self) {
        loop {
            select_biased! {
                recv(self.controller_recv) -> command => {
                    if let Ok(command) = command {
                        if matches!(command, NodeCommand::AddSender(_, _)) {
                            let (node_id, sender) = command.as_add_sender().unwrap();
                            self.neighbors.write().await.insert(node_id, sender);
                        }
                        Network::discover(
                            self.network_view.clone(),
                            self.id,
                            self.neighbors.clone(),
                            self.session_id,
                            self.flood_id,
                            self.pending.clone()
                        );
                        self.last_discovery = self.session_id;
                        self.flood_id += 1;
                        self.session_id += 1;
                    }
                },

                recv(self.packet_recv) -> packet => {
                    if let Ok(packet) = packet {
                        match packet.pack_type {
                            PacketType::Ack(_) => {},
                            PacketType::Nack(nack) => {
                                match nack.nack_type {
                                    NackType::ErrorInRouting(_) => todo!(),
                                    NackType::DestinationIsDrone => todo!(),
                                    NackType::Dropped => todo!(),
                                    NackType::UnexpectedRecipient(node_id) => {
                                        let _ = self.network_view.write().await.remove_node(node_id);
                                        let _ = self.controller_send.send(NodeEvent::NodeRemoved(node_id));
                                    },
                                }
                            },
                            PacketType::MsgFragment(f) => {}
                            PacketType::FloodRequest(flood_request) => {},
                            PacketType::FloodResponse(flood_response) => {
                                let mut lock = self.pending.lock().await;
                                if let Some(tx) = lock.remove(&self.last_discovery) {
                                    let _ = tx.send(PacketType::FloodResponse(flood_response));
                                }

                            }

                        }
                    }
                }

            }
        }
    }
}

