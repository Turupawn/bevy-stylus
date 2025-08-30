use bevy::prelude::*;
use dotenv::dotenv;
use ethers::prelude::{Provider, Http, SignerMiddleware, LocalWallet, abigen, Middleware};
use ethers::signers::Signer;
use eyre::Result;
use std::{str::FromStr, sync::Arc, fs};
use ethers::types::Address;
use serde::Deserialize;
use toml;

#[derive(Debug, Deserialize)]
struct StylusConfig {
    contract: ContractConfig,
    deployment: DeploymentConfig,
    functions: FunctionsConfig,
}

#[derive(Debug, Deserialize)]
struct ContractConfig {
    address: String,
    network: String,
    rpc_url: String,
}

#[derive(Debug, Deserialize)]
struct DeploymentConfig {
    tx_hash: String,
    activation_tx_hash: String,
    contract_size: String,
    wasm_size: String,
    wasm_data_fee: String,
}

#[derive(Debug, Deserialize)]
struct FunctionsConfig {
    signatures: Vec<String>,
}

// We'll generate the abigen macro dynamically based on the TOML file
// For now, we'll keep the hardcoded version as fallback
abigen!(
    BlockchainContract,
    r#"[
        function getSwordCounts() external view returns (uint256, uint256, uint256)
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

pub fn init_blockchain(mut commands: Commands) {
    let blockchain_client = std::thread::spawn(|| {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                init_blockchain_client().await
            })
    })
    .join()
    .unwrap();

    match blockchain_client {
        Ok(client) => {
            println!("✅ Blockchain client initialized successfully");
            commands.insert_resource(client);
        }
        Err(e) => {
            println!("❌ Failed to initialize blockchain client: {:?}", e);
            // Insert a default client with None values
            commands.insert_resource(BlockchainClient {
                contract_client: None,
                contract_address: None,
                contract: None,
            });
        }
    }
}

async fn init_blockchain_client() -> Result<BlockchainClient> {
    dotenv().ok();

    let mut client = BlockchainClient {
        contract_client: None,
        contract_address: None,
        contract: None,
    };

    // Read Stylus.toml configuration
    let config_content = fs::read_to_string("Stylus.toml")
        .map_err(|e| eyre::eyre!("Failed to read Stylus.toml: {}", e))?;
    
    let config: StylusConfig = toml::from_str(&config_content)
        .map_err(|e| eyre::eyre!("Failed to parse Stylus.toml: {}", e))?;

    println!("📋 Loaded Stylus configuration:");
    println!("  - Contract Address: {}", config.contract.address);
    println!("  - Network: {}", config.contract.network);
    println!("  - RPC URL: {}", config.contract.rpc_url);
    println!("  - Functions: {} signatures", config.functions.signatures.len());

    // Get private key from environment or use default
    let private_key = std::env::var("PRIVATE_KEY")
        .unwrap_or_else(|_| "0xb6b15c8cb491557369f3c7d2c287b053eb229daa9c22138887752191c9520659".to_string());

    println!("🔑 Using private key: {}", if private_key.len() > 10 { 
        format!("{}...{}", &private_key[..10], &private_key[private_key.len()-10..]) 
    } else { 
        private_key.clone() 
    });

    // Create provider and wallet
    let provider = Provider::<Http>::try_from(&config.contract.rpc_url)?;
    let wallet = LocalWallet::from_str(&private_key)?;
    let chain_id = provider.get_chainid().await?.as_u64();
    let client_arc = Arc::new(SignerMiddleware::new(
        provider,
        wallet.with_chain_id(chain_id),
    ));

    let contract_address: Address = config.contract.address.parse()?;
    let contract = BlockchainContract::new(contract_address, client_arc.clone());

    client.contract_client = Some(client_arc);
    client.contract_address = Some(contract_address);
    client.contract = Some(contract);

    println!("✅ Blockchain client initialized successfully!");

    Ok(client)
}


