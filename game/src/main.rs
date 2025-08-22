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
struct SwordSwing {
    direction: Vec3,
    animation_frame: u8,
    animation_timer: f32,
}

#[derive(Component)]
struct ItemDrop {
    color: u8,
}

#[derive(Component)]
struct AnimatedSprite {
    current_frame: u8,
    animation_timer: f32,
    animation_speed: f32,
    total_frames: u8,
    is_swinging: bool,
    swing_color: u8,
}

#[derive(Clone, Copy, PartialEq)]
enum PlayerDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Resource)]
struct SpriteAssets {
    // Player sprites (8 total: 2 for each direction)
    player_up: Vec<Handle<Image>>,
    player_down: Vec<Handle<Image>>,
    player_left: Vec<Handle<Image>>,
    player_right: Vec<Handle<Image>>,
    
    // Enemy sprites (2 total)
    enemy: Vec<Handle<Image>>,
    
    // Sword swing sprites (16 per color: 4 for each direction)
    sword_swings: Vec<Vec<Handle<Image>>>, // 3 colors, 16 sprites each
    
    // Item drop sprites (1 per color)
    item_drops: Vec<Handle<Image>>, // 3 colors
}

#[derive(Resource)]
struct GameState {
    swords_collected: Vec<u8>,
    contract_client: Option<Arc<SignerMiddleware<Provider<Http>, LocalWallet>>>,
    contract_address: Option<Address>,
    player_position: Vec3,
    last_direction: Vec3,
    player_moving: bool,
    player_direction: PlayerDirection,
    is_swinging: bool,
    swing_frame: u8,
    swing_timer: f32,
    swing_color: u8,
}

const PLAYER_SPEED: f32 = 400.0; // Increased from 200.0 for 4x sprites
const ENEMY_SPAWN_RATE: f32 = 2.0;

fn main() -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    let game_state = rt.block_on(init_game_state())?;

    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .insert_resource(game_state)
        .insert_resource(SpriteAssets {
            player_up: Vec::new(),
            player_down: Vec::new(),
            player_left: Vec::new(),
            player_right: Vec::new(),
            enemy: Vec::new(),
            sword_swings: vec![Vec::new(); 3],
            item_drops: Vec::new(),
        })
        .add_systems(Startup, (load_assets, setup.after(load_assets)))
        .add_systems(Update, (
            player_movement,
            player_animation,
            sword_swing_input,
            enemy_spawning,
            enemy_movement,
            enemy_animation,
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
        last_direction: Vec3::new(1.0, 0.0, 0.0), // Default to facing right
        player_moving: false,
        player_direction: PlayerDirection::Right,
        is_swinging: false,
        swing_frame: 0,
        swing_timer: 0.0,
        swing_color: 1, // Start with blue (index 1)
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
        for color in 0u8..3u8 {
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

fn load_assets(
    _commands: Commands,
    asset_server: Res<AssetServer>,
    mut sprite_assets: ResMut<SpriteAssets>,
) {
    // Load player sprites (2 frames for each direction)
    sprite_assets.player_up.push(asset_server.load("sprites/player/up_1.png"));
    sprite_assets.player_up.push(asset_server.load("sprites/player/up_2.png"));
    sprite_assets.player_down.push(asset_server.load("sprites/player/down_1.png"));
    sprite_assets.player_down.push(asset_server.load("sprites/player/down_2.png"));
    sprite_assets.player_left.push(asset_server.load("sprites/player/left_1.png"));
    sprite_assets.player_left.push(asset_server.load("sprites/player/left_2.png"));
    sprite_assets.player_right.push(asset_server.load("sprites/player/right_1.png"));
    sprite_assets.player_right.push(asset_server.load("sprites/player/right_2.png"));
    
    // Load enemy sprites (2 frames)
    sprite_assets.enemy.push(asset_server.load("sprites/enemy/enemy_1.png"));
    sprite_assets.enemy.push(asset_server.load("sprites/enemy/enemy_2.png"));
    
    // Load sword swing sprites (16 frames per color, 4 per direction)
    let color_names = ["red", "blue", "green"];
    let direction_names = ["up", "down", "left", "right"];
    
    for (color_idx, color_name) in color_names.iter().enumerate() {
        sprite_assets.sword_swings[color_idx] = Vec::new();
        for (dir_idx, dir_name) in direction_names.iter().enumerate() {
            for frame in 0..4 {
                let _sprite_idx = dir_idx * 4 + frame;
                sprite_assets.sword_swings[color_idx].push(
                    asset_server.load(&format!("sprites/swords/{}_{}_{}.png", color_name, dir_name, frame + 1))
                );
            }
        }
    }
    
    // Load item drop sprites (1 per color)
    for (_color_idx, color_name) in color_names.iter().enumerate() {
        sprite_assets.item_drops.push(asset_server.load(&format!("sprites/items/{}.png", color_name)));
    }
}

fn setup(mut commands: Commands, sprite_assets: Res<SpriteAssets>) {
    commands.spawn(Camera2dBundle::default());

    // Check if assets are loaded
    if sprite_assets.player_right.is_empty() {
        eprintln!("Warning: Sprite assets not loaded yet!");
        return;
    }

    // Player
    commands.spawn((
        SpriteBundle {
            texture: sprite_assets.player_right[0].clone(),
            transform: Transform::from_xyz(0.0, 0.0, 1.0).with_scale(Vec3::splat(4.0)),
            ..default()
        },
        Player,
        AnimatedSprite {
            current_frame: 0,
            animation_timer: 0.0,
            animation_speed: 8.0, // 8 FPS
            total_frames: 2,
            is_swinging: false,
            swing_color: 0,
        },
    ));

    // UI
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new(
                "Swords: 0 (Start collecting!)",
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
    // Don't allow movement while swinging
    if game_state.is_swinging {
        return;
    }

    if let Ok(mut transform) = player_query.get_single_mut() {
        let mut direction = Vec3::ZERO;
        let mut is_moving = false;
        
        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
            direction.y += 1.0;
            game_state.player_direction = PlayerDirection::Up;
            is_moving = true;
        }
        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
            direction.y -= 1.0;
            game_state.player_direction = PlayerDirection::Down;
            is_moving = true;
        }
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            direction.x -= 1.0;
            game_state.player_direction = PlayerDirection::Left;
            is_moving = true;
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            direction.x += 1.0;
            game_state.player_direction = PlayerDirection::Right;
            is_moving = true;
        }

        if direction.length() > 0.0 {
            direction = direction.normalize();
            transform.translation += direction * PLAYER_SPEED * time.delta_seconds();
            // Update the last direction when moving
            game_state.last_direction = direction;
        }
        
        game_state.player_moving = is_moving;
        // Update the stored player position
        game_state.player_position = transform.translation;
    }
}

fn sword_swing_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut game_state: ResMut<GameState>,
) {
    // Start sword swing if space is pressed and not already swinging
    if keyboard.just_pressed(KeyCode::Space) && !game_state.is_swinging {
        game_state.is_swinging = true;
        game_state.swing_frame = 0;
        game_state.swing_timer = 0.0;
        // Use the current swing color (starts as blue, changes when collected)
        // game_state.swing_color is already set from collect_swords
    }
}

fn player_animation(
    mut player_query: Query<(&mut Handle<Image>, &mut AnimatedSprite), With<Player>>,
    mut game_state: ResMut<GameState>,
    sprite_assets: Res<SpriteAssets>,
    time: Res<Time>,
) {
    if let Ok((mut texture, mut animated_sprite)) = player_query.get_single_mut() {
        // Check if assets are loaded
        if sprite_assets.player_right.is_empty() {
            return;
        }
        
        // Update swing animation
        if game_state.is_swinging {
            game_state.swing_timer += time.delta_seconds();
            
            // Animate at 12 FPS
            if game_state.swing_timer >= 1.0 / 12.0 {
                game_state.swing_frame += 1;
                game_state.swing_timer = 0.0;
                
                // If swing animation is complete, stop swinging
                if game_state.swing_frame >= 4 {
                    game_state.is_swinging = false;
                    animated_sprite.is_swinging = false;
                } else {
                    // Update to sword swing sprite
                    let direction_idx = match game_state.player_direction {
                        PlayerDirection::Up => 0,
                        PlayerDirection::Down => 1,
                        PlayerDirection::Left => 2,
                        PlayerDirection::Right => 3,
                    };
                    
                    let sprite_idx = direction_idx * 4 + game_state.swing_frame as usize;
                    
                    // Check bounds
                    if (game_state.swing_color as usize) < sprite_assets.sword_swings.len() && 
                       sprite_idx < sprite_assets.sword_swings[game_state.swing_color as usize].len() {
                        *texture = sprite_assets.sword_swings[game_state.swing_color as usize][sprite_idx].clone();
                    }
                }
            }
        } else {
            // Normal walking animation
            animated_sprite.animation_timer += time.delta_seconds();
            
            // Get the appropriate sprite array based on direction
            let sprite_array = match game_state.player_direction {
                PlayerDirection::Up => &sprite_assets.player_up,
                PlayerDirection::Down => &sprite_assets.player_down,
                PlayerDirection::Left => &sprite_assets.player_left,
                PlayerDirection::Right => &sprite_assets.player_right,
            };
            
            // Check if the sprite array has the required frame
            if animated_sprite.current_frame as usize >= sprite_array.len() {
                return;
            }
            
            // Update animation frame if moving
            if game_state.player_moving && animated_sprite.animation_timer >= 1.0 / animated_sprite.animation_speed {
                animated_sprite.current_frame = (animated_sprite.current_frame + 1) % animated_sprite.total_frames;
                animated_sprite.animation_timer = 0.0;
            } else if !game_state.player_moving {
                // Reset to first frame when not moving
                animated_sprite.current_frame = 0;
            }
            
            // Update texture
            *texture = sprite_array[animated_sprite.current_frame as usize].clone();
        }
    }
}

fn enemy_spawning(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: Local<f32>,
    sprite_assets: Res<SpriteAssets>,
) {
    // Check if assets are loaded
    if sprite_assets.enemy.is_empty() {
        return;
    }
    
    *timer += time.delta_seconds();
    if *timer >= ENEMY_SPAWN_RATE {
        *timer = 0.0;
        
        // Spawn enemies from outside the screen
        // Screen boundaries (assuming 800x600, but we'll use larger values for safety)
        let screen_width = 1000.0;
        let screen_height = 800.0;
        
        // Randomly choose which edge to spawn from
        let spawn_side = rand::random::<u8>() % 4;
        let (x, y) = match spawn_side {
            0 => { // Top edge
                (rand::random::<f32>() * screen_width - screen_width / 2.0, screen_height / 2.0 + 50.0)
            },
            1 => { // Bottom edge
                (rand::random::<f32>() * screen_width - screen_width / 2.0, -screen_height / 2.0 - 50.0)
            },
            2 => { // Left edge
                (-screen_width / 2.0 - 50.0, rand::random::<f32>() * screen_height - screen_height / 2.0)
            },
            _ => { // Right edge
                (screen_width / 2.0 + 50.0, rand::random::<f32>() * screen_height - screen_height / 2.0)
            }
        };
        
        commands.spawn((
            SpriteBundle {
                texture: sprite_assets.enemy[0].clone(),
                transform: Transform::from_xyz(x, y, 0.0).with_scale(Vec3::splat(4.0)),
                ..default()
            },
            Enemy,
            AnimatedSprite {
                current_frame: 0,
                animation_timer: 0.0,
                animation_speed: 6.0, // 6 FPS
                total_frames: 2,
                is_swinging: false,
                swing_color: 0,
            },
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
        enemy_transform.translation += direction * 100.0 * time.delta_seconds(); // Increased from 50.0
    }
}

fn enemy_animation(
    mut enemy_query: Query<(&mut Handle<Image>, &mut AnimatedSprite), With<Enemy>>,
    sprite_assets: Res<SpriteAssets>,
    time: Res<Time>,
) {
    // Check if assets are loaded
    if sprite_assets.enemy.is_empty() {
        return;
    }
    
    for (mut texture, mut animated_sprite) in enemy_query.iter_mut() {
        animated_sprite.animation_timer += time.delta_seconds();
        
        if animated_sprite.animation_timer >= 1.0 / animated_sprite.animation_speed {
            animated_sprite.current_frame = (animated_sprite.current_frame + 1) % animated_sprite.total_frames;
            animated_sprite.animation_timer = 0.0;
            
            // Check bounds
            if animated_sprite.current_frame as usize >= sprite_assets.enemy.len() {
                continue;
            }
            
            // Update texture
            *texture = sprite_assets.enemy[animated_sprite.current_frame as usize].clone();
        }
    }
}

fn sword_collision(
    mut commands: Commands,
    game_state: Res<GameState>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
    sprite_assets: Res<SpriteAssets>,
) {
    // Check if assets are loaded
    if sprite_assets.item_drops.is_empty() {
        return;
    }
    
    // Only check collision if player is swinging and on the right frame (frame 1-2 are the "active" frames)
    if !game_state.is_swinging || game_state.swing_frame < 1 || game_state.swing_frame > 2 {
        return;
    }
    
    // Calculate sword position based on player position and direction
    let sword_offset = game_state.last_direction * 50.0; // Increased from 25.0 for 4x sprites
    let sword_position = game_state.player_position + sword_offset;
    
    for (enemy_entity, enemy_transform) in enemy_query.iter() {
        let distance = sword_position.distance(enemy_transform.translation);
        if distance < 60.0 { // Increased from 30.0 for 4x sprites
            commands.entity(enemy_entity).despawn();
            
            // Spawn sword drop
            let color = rand::random::<u8>() % 3;
            
            // Check bounds
            if color as usize >= sprite_assets.item_drops.len() {
                continue;
            }
            
            commands.spawn((
                SpriteBundle {
                    texture: sprite_assets.item_drops[color as usize].clone(),
                    transform: Transform::from_xyz(
                        enemy_transform.translation.x,
                        enemy_transform.translation.y,
                        0.0,
                    ).with_scale(Vec3::splat(2.0)),
                    ..default()
                },
                Sword { color },
                ItemDrop { color },
            ));
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
        if distance < 60.0 { // Increased from 30.0 for 4x sprites
            game_state.swords_collected.push(sword.color);
            // Change the sword color to the collected color
            game_state.swing_color = sword.color;
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
        let mut color_counts = [0u32; 3];
        for &color in &game_state.swords_collected {
            color_counts[color as usize] += 1;
        }
        
        // Create color names
        let color_names = ["Red", "Blue", "Green"];
        
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
