use std::{
    any::Any,
    collections::{
        hash_map::Entry::Vacant,
        HashMap,
    },
};

use common::{
    types::{MediaFile, ServerType, TextFile, WebRequest, WebResponse},
    FragmentAssembler, Processor, RoutingHandler,
};
use crossbeam_channel::{Receiver, Sender};
use wg_internal::{
    network::NodeId,
    packet::{NodeType, Packet},
};

type Cache = HashMap<TextFile, Option<Vec<MediaFile>>>;

#[derive(Debug)]
pub struct WebBrowser {
    id: NodeId,
    routing_handler: RoutingHandler,
    controller_recv: Receiver<Box<dyn Any>>,
    controller_send: Sender<Box<dyn Any>>,
    packet_recv: Receiver<Packet>,
    assembler: FragmentAssembler,
    text_servers: HashMap<NodeId, Vec<String>>,
    cached_files: Cache,
}

impl WebBrowser {
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
            text_servers: HashMap::new(),
            cached_files: HashMap::new(),
        }
    }

    fn get_text_servers(&self) -> Vec<NodeId> {
        self.text_servers.keys().copied().collect()
    }

    fn get_list_files_by_id(&self, id: NodeId) -> Option<&Vec<String>> {
        self.text_servers.get(&id)
    }

    fn set_files_list(&mut self, server_id: NodeId, list: Vec<String>) {
        if let Vacant(e) = self.text_servers.entry(server_id) {
            e.insert(list);
        }
    }
}

impl Processor for WebBrowser {
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

    fn handle_msg(&mut self, msg: Vec<u8>, from: NodeId, session_id: u64) {
        if let Ok(msg) = serde_json::from_slice::<WebResponse>(&msg) {
            match msg {
                WebResponse::ServerType { server_type } => {
                    if matches!(server_type, ServerType::TextServer) {
                        self.text_servers.insert(from, vec![]);
                    }
                }
                WebResponse::TextFilesList { files } => {
                    self.set_files_list(from, files);
                }
                WebResponse::TextFile { file_data } => {
                    if let Ok(file) = serde_json::from_slice::<TextFile>(&file_data) {
                        for r in &file.get_refs() {
                            if let Ok(req) = serde_json::to_vec(&WebRequest::MediaQuery {
                                media_id: r.id.to_string(),
                            }) {
                                let _ = self.routing_handler.send_message(
                                    &req,
                                    r.get_location(),
                                    Some(session_id),
                                );
                            }
                        }
                    }
                }
                WebResponse::MediaFile { media_data } => todo!(),
                WebResponse::ErrorNotFound => todo!(),
                WebResponse::ErrorUnsupportedRequest => todo!(),
            }
        }
    }
}
