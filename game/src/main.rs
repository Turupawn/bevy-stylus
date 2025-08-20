use bevy::prelude::{*, Res};
use dotenv::dotenv;
use ethers::prelude::{Provider, Http, SignerMiddleware, LocalWallet, abigen, Middleware, Address};
use ethers::signers::Signer; // <-- add this
use eyre::Result;
use std::{str::FromStr, sync::Arc};

#[derive(Resource)]
struct CounterValue(u64);

fn main() -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    let initial_counter = rt.block_on(fetch_counter())?;

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(CounterValue(initial_counter))
        .add_systems(Startup, setup_text)
        .add_systems(Update, update_counter_text)
        .run();

    Ok(())
}

async fn fetch_counter() -> Result<u64> {
    dotenv().ok();

    let rpc_url = std::env::var("RPC_URL")?;
    let contract_address: Address = std::env::var("STYLUS_CONTRACT_ADDRESS")?.parse()?;
    let privkey = std::env::var("PRIVATE_KEY")?;

    abigen!(
        Counter,
        r#"[
            function number() external view returns (uint256)
            function increment() external
        ]"#
    );

    let provider = Provider::<Http>::try_from(rpc_url)?;
    let wallet = LocalWallet::from_str(&privkey)?;
    let chain_id = provider.get_chainid().await?.as_u64();
    let client = Arc::new(SignerMiddleware::new(
        provider,
        wallet.with_chain_id(chain_id),
    ));

    let counter = Counter::new(contract_address, client);

    let pending = counter.increment();
    if let Some(receipt) = pending.send().await?.await? {
        println!("Receipt = {:?}", receipt);
    }
    println!("Successfully incremented counter via a tx");

    let num = counter.number().call().await?;
    Ok(num.as_u64())
}

fn setup_text(mut commands: Commands, counter: Res<CounterValue>, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn((
        TextBundle::from_sections([
            TextSection::new(
                "Loading...",
                TextStyle {
                    font_size: 20.0,
                    color: Color::WHITE,
                    ..default()
                },
            ),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }),
    ));
}

fn update_counter_text(mut query: Query<&mut Text>, counter: Res<CounterValue>) {
    if counter.is_changed() {
        for mut text in query.iter_mut() {
            text.sections[0].value = format!("Counter: {}", counter.0);
        }
    }
}
