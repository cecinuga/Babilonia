use std::time::Duration;

use libp2p::{identity::Keypair, PeerId, gossipsub::{IdentTopic as Topic, Gossipsub, MessageAuthenticity, GossipsubConfigBuilder, ValidationMode, GossipsubEvent, GossipsubMessage}, NetworkBehaviour, futures::channel::mpsc, mdns::{Mdns, MdnsEvent}, swarm::NetworkBehaviourEventProcess };
use log::info;
use once_cell::sync::Lazy;
use serde::{Serialize, Deserialize};

use crate::structs::{block::Block, app::App};

pub static KEYS: Lazy<Keypair> = Lazy::new(Keypair::generate_ed25519);
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));
pub static PATH: Lazy<String> = Lazy::new(|| String::from("./assets/music/"));

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainResponse {
    pub blocks: Vec<Block>,
    pub receiver: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalChainRequest{
    pub from_peer_id: String,
}

pub enum EventType {
    LocalChainResponse(ChainResponse),
    Input(String),
    Init,
}

#[derive(NetworkBehaviour)]
pub struct AppBehaviour{
    pub gossipsub: Gossipsub,
    pub mdns: Mdns,
    #[behaviour(ignore)]
    pub response_sender: mpsc::UnboundedSender<GossipsubEvent>,
    #[behaviour(ignore)]
    pub init_sender: mpsc::UnboundedSender<GossipsubEvent>,
    #[behaviour(ignore)]
    pub app: App,
}

impl AppBehaviour {
    pub async fn new(
        app: App,
        response_sender: mpsc::UnboundedSender<GossipsubEvent>,
        init_sender: mpsc::UnboundedSender<GossipsubEvent>,
    ) -> Self {
        let gossipsub_config = GossipsubConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(ValidationMode::Strict)
            .build()
            .expect("Valid config");

        let mut gossipsub = Gossipsub::new(MessageAuthenticity::Signed(KEYS.clone()), gossipsub_config).unwrap();
        gossipsub.subscribe(&CHAIN_TOPIC.clone());
        gossipsub.subscribe(&BLOCK_TOPIC.clone());
        
        let mut behaviour = Self {
            app,
            gossipsub: gossipsub,
            mdns: Mdns::new(Default::default())
                .await
                .expect("can create mdns"),
            response_sender,
            init_sender,
        };
        behaviour
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for AppBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(discovered_list) => {
                for (peer, _addr) in discovered_list {
                    self.gossipsub.add_explicit_peer(&peer);
                }
            }
            MdnsEvent::Expired(expired_list) => {
                for (peer, _addr) in expired_list {
                    if !self.mdns.has_node(&peer) {
                        self.gossipsub.remove_explicit_peer(&peer)
                    }
                }
            }
        }
    }
}

impl NetworkBehaviourEventProcess<GossipsubEvent> for AppBehaviour {
    fn inject_event(&mut self, event: GossipsubEvent){
        if let GossipsubEvent::Message{
            message, 
            propagation_source, 
            message_id
        } = event {
                if let Ok(resp) = serde_json::from_slice::<ChainResponse>(message.data.as_slice()) {
                    if resp.receiver == PEER_ID.to_string(){
                        info!("Response from {}:", message.source.expect(""));
                        resp.blocks.iter().for_each(|r| info!("{:?}", r));

                        self.app.blocks = self.app.choose_chain(self.app.blocks.clone(), resp.blocks.clone());
                    }
                }
        }
    }
}