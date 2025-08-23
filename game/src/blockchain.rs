use bevy::prelude::*;
use dotenv::dotenv;
use ethers::prelude::{Provider, Http, SignerMiddleware, LocalWallet, abigen, Middleware};
use ethers::signers::Signer;
use eyre::Result;
use std::{str::FromStr, sync::Arc};
use ethers::types::Address;

// Generate the contract bindings
abigen!(
    BlockchainContract,
    r#"[
        function getSwordCount(uint256 color) external view returns (uint256)
        function incrementSword(uint256 color) external
    ]"#
);

#[derive(Resource, Clone)]
pub struct BlockchainClient {
    pub contract_client: Option<Arc<SignerMiddleware<Provider<Http>, LocalWallet>>>,
    pub contract_address: Option<Address>,
    pub contract: Option<BlockchainContract<SignerMiddleware<Provider<Http>, LocalWallet>>>,
}

pub struct BlockchainPlugin;

impl Plugin for BlockchainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_blockchain);
    }
}

fn init_blockchain(mut commands: Commands) {
    // Initialize blockchain client in a blocking task since we can't use async in systems
    let blockchain_client = std::thread::spawn(|| {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                init_blockchain_client().await
            })
    })
    .join()
    .unwrap()
    .unwrap();

    commands.insert_resource(blockchain_client);
}

async fn init_blockchain_client() -> Result<BlockchainClient> {
    dotenv().ok();

    let mut client = BlockchainClient {
        contract_client: None,
        contract_address: None,
        contract: None,
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
        let contract = BlockchainContract::new(contract_address, client_arc.clone());

        client.contract_client = Some(client_arc);
        client.contract_address = Some(contract_address);
        client.contract = Some(contract);
    }

    Ok(client)
}
