use dotenv::dotenv;
use ethers::prelude::{Provider, Http, SignerMiddleware, LocalWallet, abigen, Middleware};
use ethers::signers::Signer;
use eyre::Result;
use std::{str::FromStr, sync::Arc};
use ethers::types::{Address, U256};

// Generate the contract bindings
abigen!(
    SwordCollection,
    r#"[
        function getSwordCount(uint256 color) external view returns (uint256)
        function incrementSword(uint256 color) external
    ]"#
);

#[derive(Clone)]
pub struct BlockchainClient {
    pub contract_client: Option<Arc<SignerMiddleware<Provider<Http>, LocalWallet>>>,
    pub contract_address: Option<Address>,
}

impl BlockchainClient {
    pub async fn new() -> Result<Self> {
        dotenv().ok();

        println!("RPC_URL: {}", std::env::var("RPC_URL").unwrap());
        println!("STYLUS_CONTRACT_ADDRESS: {}", std::env::var("STYLUS_CONTRACT_ADDRESS").unwrap());
        println!("PRIVATE_KEY: {}", std::env::var("PRIVATE_KEY").unwrap());
        
        let mut client = BlockchainClient {
            contract_client: None,
            contract_address: None,
        };

        if let (Ok(rpc_url), Ok(contract_addr), Ok(privkey)) = (
            std::env::var("RPC_URL"),
            std::env::var("STYLUS_CONTRACT_ADDRESS"),
            std::env::var("PRIVATE_KEY"),
        ) {
            let provider = Provider::<Http>::try_from(rpc_url)?;
            let wallet = LocalWallet::from_str(&privkey)?;
            let chain_id = provider.get_chainid().await?.as_u64();
            let client_arc = Arc::new(SignerMiddleware::new(
                provider,
                wallet.with_chain_id(chain_id),
            ));

            let contract_address: Address = contract_addr.parse()?;
            let contract = SwordCollection::new(contract_address, client_arc.clone());

            // Load existing swords
            for color in 0u8..3u8 {
                println!("Loading sword count for color: {}", color);
                let count: U256 = contract.get_sword_count(U256::from(color)).call().await?;
                println!("counting: {}", count);
                println!("fin");
            }

            client.contract_client = Some(client_arc);
            client.contract_address = Some(contract_address);
        }

        Ok(client)
    }

    pub async fn load_existing_swords(&self) -> Result<Vec<u8>> {
        let mut swords_collected = Vec::new();
        
        if let (Some(client), Some(address)) = (&self.contract_client, self.contract_address) {
            let contract = SwordCollection::new(address.clone(), client.clone());
            
            for color in 0u8..3u8 {
                let count: U256 = contract.get_sword_count(U256::from(color)).call().await?;
                for _ in 0..count.as_u64() {
                    swords_collected.push(color);
                }
            }
        }
        
        Ok(swords_collected)
    }

    pub async fn save_sword(&self, color: u8) -> Result<()> {
        if let (Some(client), Some(address)) = (&self.contract_client, self.contract_address) {
            let contract = SwordCollection::new(address.clone(), client.clone());
            if let Err(e) = contract.increment_sword(U256::from(color)).send().await {
                eprintln!("Failed to save sword to contract: {}", e);
            }
        }
        Ok(())
    }


}
