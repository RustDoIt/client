use std::{any::Any, collections::{HashMap, HashSet}};

use common::{FragmentAssembler, RoutingHandler, Processor, types::{TextFile, MediaFile}};
use crossbeam_channel::{Receiver, Sender};
use wg_internal::{network::NodeId, packet::{NodeType, Packet}};

type Cache<'a> = HashMap<TextFile<'a>, Option<Vec<MediaFile>>>;

#[derive(Debug)]
pub struct WebBrowser<'a>{
    id: NodeId,
    routing_handler: RoutingHandler,
    controller_recv: Receiver<Box<dyn Any>>,
    controller_send: Sender<Box<dyn Any>>,
    packet_recv: Receiver<Packet>,
    assembler: FragmentAssembler,
    text_servers: HashSet<NodeId>,
    cached_files: Cache<'a>
}

impl WebBrowser<'_> {
    #[must_use]
    pub fn new(
        id: NodeId,
        neighbors: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        controller_recv: Receiver<Box<dyn Any>>,
        controller_send: Sender<Box<dyn Any>>,
    ) -> Self {
        let routing_handler =
            RoutingHandler::new(id, NodeType::Client, neighbors, controller_send.clone());

        Self {
            id,
            routing_handler,
            controller_recv,
            controller_send,
            packet_recv,
            assembler: FragmentAssembler::default(),
            text_servers: HashSet::new(),
            cached_files: HashMap::new()
        }
    }
}


impl Processor for WebBrowser<'_> {
    fn controller_recv(&self) -> &Receiver<Box<dyn Any>> {
        &self.controller_recv
    }

    fn packet_recv(&self) -> &Receiver<Packet> {
        &self.packet_recv
    }

    fn assembler(&mut self) -> &mut FragmentAssembler {
        &mut self.assembler
    }

    fn routing_handler(&mut self) -> &mut RoutingHandler {
        &mut self.routing_handler
    }

    fn handle_command(&mut self, _cmd: Box<dyn Any>) -> bool {
        todo!()
    }

    fn handle_msg(&mut self, _msg: Vec<u8>, _from: NodeId, _session_id: u64) {
        todo!()
    }
}
