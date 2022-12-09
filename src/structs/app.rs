use Babilonia::{hash_to_binary_representation, DIFFICULTY_PREFIX, calculate_hash};
use chrono::Utc;
use log::{error, warn};
use serde::{Serialize, Deserialize};
use super::block::Block;

#[derive(Serialize, Deserialize, Debug)]
pub struct App{
    pub blocks: Vec<Block>,
}

impl App {
    fn new() -> Self{
        Self { blocks: vec![] }
    }

    fn genesis(&mut self) {
        let genesis_block = Block {
            id: 0,
            timestamp: Utc::now().timestamp(),
            previous_hash: String::from("genesis"),
            data: String::from("genesis!"),
            nonce: 2836,
            hash: "0000f816a87f806bb0073dcf026a64fb40c946b5abee2573702828694d5b4c43".to_string(),
        };
        self.blocks.push(genesis_block);
    }

    fn try_add_block(&mut self, block: Block){
        let latest_block = self.blocks.last().expect("There is at least one block");
        if self.is_block_valid(&block, latest_block){
            self.blocks.push(block);
        } else {
            error!("Could not add block - invalid");
        }
    }

    fn is_block_valid(&self, block: &Block, previous_block: &Block) -> bool {
        if block.previous_hash != previous_block.hash {
            warn!("block with id: {} has wrong previous_hash", block.id);
            return false;
        } else if !hash_to_binary_representation(
            &hex::decode(&block.hash).expect("Can decode from hex"),
            ).starts_with(DIFFICULTY_PREFIX){
                warn!("block with id: {} hsh invalid difficulty", block.id);
                return false;
        } else if hex::encode(calculate_hash(
            block.id,
            block.timestamp,
            &block.previous_hash,
            &block.data,
            block.nonce,
        )) != block.hash {
            warn!("Block with id: {} has invalid hash", block.id);
            return false;
        }
        true
    }

    fn is_chain_valid(&self, chain: &[Block]) -> bool {
        for i in 0..chain.len()  {
            if i == 0{
                continue;
            }
            let first = chain.get(i-1).expect("has to exist");
            let second = chain.get(i).expect("has to exist");
            if !self.is_block_valid(second, first){
                return false;
            }
        }  
        true
    }

    pub fn choose_chain(&mut self, local: Vec<Block>, remote: Vec<Block>) -> Vec<Block>{
        let is_local_valid = self.is_chain_valid(&local);
        let is_remote_valid = self.is_chain_valid(&remote);

        if is_local_valid && is_remote_valid {
            if local.len() >= remote.len() {
                local
            } else { remote }
        } else if is_remote_valid && !is_local_valid{
            remote
        } else if !is_remote_valid && is_local_valid{
            local
        } else{
            panic!("local and remote chains are both invalid");
        }
    }
}