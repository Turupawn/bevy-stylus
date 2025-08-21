use bevy::prelude::*;
use dotenv::dotenv;
use ethers::prelude::{Provider, Http, SignerMiddleware, LocalWallet, abigen, Middleware};
use ethers::signers::Signer;
use eyre::Result;
use std::{str::FromStr, sync::Arc};


use ethers::{
    types::{Address, U256},
};

// Generate the contract bindings
abigen!(
    SwordCollection,
    r#"[
        function number() external view returns (uint256)
        function increment666() external
        function getSwordCount(uint256 color) external view returns (uint256)
        function incrementSword(uint256 color) external
    ]"#
);

// Game components
#[derive(Component)]
struct Player;

#[derive(Component)]
struct Enemy;

#[derive(Component)]
struct Sword {
    color: u8,
}

#[derive(Component)]
struct SwordSwing;

#[derive(Resource)]
struct GameState {
    swords_collected: Vec<u8>,
    contract_client: Option<Arc<SignerMiddleware<Provider<Http>, LocalWallet>>>,
    contract_address: Option<Address>,
    player_position: Vec3,
}

const PLAYER_SPEED: f32 = 200.0;
const ENEMY_SPAWN_RATE: f32 = 2.0;

fn main() -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    let game_state = rt.block_on(init_game_state())?;

    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .insert_resource(game_state)
        .add_systems(Startup, setup)
        .add_systems(Update, (
            player_movement,
            sword_swing,
            enemy_spawning,
            enemy_movement,
            sword_collision,
            collect_swords,
            update_ui,
        ))
        .run();

    Ok(())
}

async fn init_game_state() -> Result<GameState> {
    dotenv().ok();

    println!("RPC_URL: {}", std::env::var("RPC_URL").unwrap());
    println!("STYLUS_CONTRACT_ADDRESS: {}", std::env::var("STYLUS_CONTRACT_ADDRESS").unwrap());
    println!("PRIVATE_KEY: {}", std::env::var("PRIVATE_KEY").unwrap());
    
    let mut game_state = GameState {
        swords_collected: Vec::new(),
        contract_client: None,
        contract_address: None,
        player_position: Vec3::ZERO,
    };

    if let (Ok(rpc_url), Ok(contract_addr), Ok(privkey)) = (
        std::env::var("RPC_URL"),
        std::env::var("STYLUS_CONTRACT_ADDRESS"),
        std::env::var("PRIVATE_KEY"),
    ) {
        let provider = Provider::<Http>::try_from(rpc_url)?;
        let wallet = LocalWallet::from_str(&privkey)?;
        let chain_id = provider.get_chainid().await?.as_u64();
        let client = Arc::new(SignerMiddleware::new(
            provider,
            wallet.with_chain_id(chain_id),
        ));

        let contract_address: Address = contract_addr.parse()?;
        let contract = SwordCollection::new(contract_address, client.clone());

        // Load existing swords
        for color in 0u8..5u8 {
            println!("Loading sword count for colorr: {}", color);
            let count: U256 = contract.get_sword_count(U256::from(color)).call().await?;
            //let count = contract.number().call().await?;
            println!("counting: {}", count);
            println!("fin");
            for _ in 0..count.as_u64() {
                game_state.swords_collected.push(color);
            }
        }

        game_state.contract_client = Some(client);
        game_state.contract_address = Some(contract_address);
    }

    Ok(game_state)
}

fn setup(mut commands: Commands, game_state: Res<GameState>) {
    commands.spawn(Camera2dBundle::default());

    // Player
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(0.0, 0.0, 1.0), // Blue
                custom_size: Some(Vec2::new(32.0, 32.0)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 1.0),
            ..default()
        },
        Player,
    ));

    // UI
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new(
                format!("Swords: {} (Start collecting!)", game_state.swords_collected.len()),
                TextStyle {
                    font_size: 24.0,
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

fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<&mut Transform, With<Player>>,
    mut game_state: ResMut<GameState>,
    time: Res<Time>,
) {
    if let Ok(mut transform) = player_query.get_single_mut() {
        let mut direction = Vec3::ZERO;
        
        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
            direction.y += 1.0;
        }
        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
            direction.y -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            direction.x -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            direction.x += 1.0;
        }

        if direction.length() > 0.0 {
            direction = direction.normalize();
            transform.translation += direction * PLAYER_SPEED * time.delta_seconds();
        }
        
        // Update the stored player position
        game_state.player_position = transform.translation;
    }
}

fn sword_swing(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    game_state: Res<GameState>,
    swing_query: Query<Entity, With<SwordSwing>>,
) {
    // Remove expired swings
    for entity in swing_query.iter() {
        commands.entity(entity).despawn();
    }

    // Create new swing
    if keyboard.just_pressed(KeyCode::Space) {
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::srgb(1.0, 1.0, 0.0), // Yellow
                    custom_size: Some(Vec2::new(40.0, 8.0)),
                    ..default()
                },
                transform: Transform::from_xyz(
                    game_state.player_position.x + 20.0,
                    game_state.player_position.y,
                    0.5,
                ),
                ..default()
            },
            SwordSwing,
        ));
    }
}

fn enemy_spawning(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: Local<f32>,
) {
    *timer += time.delta_seconds();
    if *timer >= ENEMY_SPAWN_RATE {
        *timer = 0.0;
        
        let x = (rand::random::<f32>() - 0.5) * 800.0;
        let y = (rand::random::<f32>() - 0.5) * 600.0;
        
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::srgb(1.0, 0.0, 0.0), // Red
                    custom_size: Some(Vec2::new(24.0, 24.0)),
                    ..default()
                },
                transform: Transform::from_xyz(x, y, 0.0),
                ..default()
            },
            Enemy,
        ));
    }
}

fn enemy_movement(
    mut enemy_query: Query<&mut Transform, With<Enemy>>,
    game_state: Res<GameState>,
    time: Res<Time>,
) {
    for mut enemy_transform in enemy_query.iter_mut() {
        let direction = (game_state.player_position - enemy_transform.translation).normalize();
        enemy_transform.translation += direction * 50.0 * time.delta_seconds();
    }
}

fn sword_collision(
    mut commands: Commands,
    swing_query: Query<&Transform, With<SwordSwing>>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
) {
    for swing_transform in swing_query.iter() {
        for (enemy_entity, enemy_transform) in enemy_query.iter() {
            let distance = swing_transform.translation.distance(enemy_transform.translation);
            if distance < 30.0 {
                commands.entity(enemy_entity).despawn();
                
                // Spawn sword drop
                let color = rand::random::<u8>() % 5;
                let sword_color = match color {
                    0 => Color::srgb(1.0, 0.0, 0.0), // Red
                    1 => Color::srgb(0.0, 0.0, 1.0), // Blue
                    2 => Color::srgb(0.0, 1.0, 0.0), // Green
                    3 => Color::srgb(1.0, 1.0, 0.0), // Yellow
                    _ => Color::srgb(1.0, 0.0, 1.0), // Purple
                };
                
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: sword_color,
                            custom_size: Some(Vec2::new(16.0, 32.0)),
                            ..default()
                        },
                        transform: Transform::from_xyz(
                            enemy_transform.translation.x,
                            enemy_transform.translation.y,
                            0.0,
                        ),
                        ..default()
                    },
                    Sword { color },
                ));
            }
        }
    }
}

fn collect_swords(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    sword_query: Query<(Entity, &Transform, &Sword)>,
) {
    for (sword_entity, sword_transform, sword) in sword_query.iter() {
        let distance = game_state.player_position.distance(sword_transform.translation);
        if distance < 30.0 {
            game_state.swords_collected.push(sword.color);
            commands.entity(sword_entity).despawn();
            
            // Save to contract
            if let (Some(client), Some(address)) = (&game_state.contract_client, game_state.contract_address) {
                let contract = SwordCollection::new(address.clone(), client.clone());
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    if let Err(e) = contract.increment_sword(U256::from(sword.color)).send().await {
                        eprintln!("Failed to save sword to contract: {}", e);
                    }
                });
            }
        }
    }
}

fn update_ui(mut text_query: Query<&mut Text>, game_state: Res<GameState>) {
    if game_state.is_changed() {
        // Count swords by color
        let mut color_counts = [0u32; 5];
        for &color in &game_state.swords_collected {
            color_counts[color as usize] += 1;
        }
        
        // Create color names
        let color_names = ["Red", "Blue", "Green", "Yellow", "Purple"];
        
        // Build the display text
        let mut display_text = format!("Total Swords: {}\n", game_state.swords_collected.len());
        for (name, count) in color_names.iter().zip(color_counts.iter()) {
            display_text.push_str(&format!("{}: {} ", name, count));
        }
        
        for mut text in text_query.iter_mut() {
            text.sections[0].value = display_text.clone();
        }
    }
}
