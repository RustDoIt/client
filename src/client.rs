use wg_internal::{
    network::NodeId,
    packet::Packet,
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

