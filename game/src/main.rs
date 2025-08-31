use bevy::prelude::*;
use bevy_stylus_plugin::{BlockchainPlugin, BlockchainClient};
use ethers::types::U256;
use eyre::Result;

pub fn init_game(
    _commands: Commands,
    _asset_server: Res<AssetServer>,
    _sprite_assets: ResMut<SpriteAssets>,
    blockchain_client: Res<BlockchainClient>,
    mut game_state: ResMut<GameState>
) {
    game_state.swords_collected = Vec::new();
    /*
    */
    game_state.swords_collected = Vec::new();
    let client = blockchain_client.clone();
    let swords = std::thread::spawn(move || {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let mut swords_collected = Vec::new();
            if let Some(contract) = &client.contract {
                if let Ok((red_count, green_count, blue_count)) = contract.get_sword_counts().call().await {
                    for _ in 0..red_count.as_u64() {
                        swords_collected.push(0); // Red
                    }
                    for _ in 0..green_count.as_u64() {
                        swords_collected.push(1); // Green
                    }
                    for _ in 0..blue_count.as_u64() {
                        swords_collected.push(2); // Blue
                    }
                } else {
                    eprintln!("Failed to get sword counts");
                }
            }
            swords_collected
        })
    }).join().unwrap_or_default();
    game_state.swords_collected = swords;
}

fn collect_swords(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    sword_query: Query<(Entity, &Transform, &Sword)>,
    blockchain_client: Res<BlockchainClient>, // BLOCKCHAIN
) {
    for (sword_entity, sword_transform, sword) in sword_query.iter() {
        let distance = game_state.player_position.distance(sword_transform.translation);
        if distance < 60.0 {
            game_state.swords_collected.push(sword.color);
            game_state.swing_color = sword.color;
            commands.entity(sword_entity).despawn();
            /*
            */
            let client = blockchain_client.clone();
            let color = sword.color;
            std::thread::spawn(move || {
                tokio::runtime::Runtime::new().unwrap().block_on(async {
                    if let Some(contract) = &client.clone().contract {
                        if let Err(e) = contract.increment_sword(U256::from(color)).send().await {
                            eprintln!("Failed to save sword to contract: {}", e);
                        }
                    }
                });
            });
        }
    }
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Enemy;

#[derive(Component)]
struct Sword {
    color: u8,
}

#[derive(Component)]
struct ItemDrop;

#[derive(Component)]
struct AnimatedSprite {
    current_frame: u8,
    animation_timer: f32,
    animation_speed: f32,
    total_frames: u8,
    is_swinging: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum PlayerDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Resource)]
pub struct SpriteAssets {
    player_up: Vec<Handle<Image>>,
    player_down: Vec<Handle<Image>>,
    player_left: Vec<Handle<Image>>,
    player_right: Vec<Handle<Image>>,
    enemy: Vec<Handle<Image>>,
    sword_swings: Vec<Vec<Handle<Image>>>,
    item_drops: Vec<Handle<Image>>,
}

#[derive(Resource)]
pub struct GameState {
    pub swords_collected: Vec<u8>,
    player_position: Vec3,
    last_direction: Vec3,
    player_moving: bool,
    player_direction: PlayerDirection,
    is_swinging: bool,
    swing_frame: u8,
    swing_timer: f32,
    swing_color: u8,
}

const PLAYER_SPEED: f32 = 400.0;
const ENEMY_SPAWN_RATE: f32 = 2.0;

fn main() -> Result<()> {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(BlockchainPlugin)
        .insert_resource(GameState {
            swords_collected: Vec::new(),
            player_position: Vec3::ZERO,
            last_direction: Vec3::new(1.0, 0.0, 0.0),
            player_moving: false,
            player_direction: PlayerDirection::Right,
            is_swinging: false,
            swing_frame: 0,
            swing_timer: 0.0,
            swing_color: 1,
        })
        .insert_resource(SpriteAssets {
            player_up: Vec::new(),
            player_down: Vec::new(),
            player_left: Vec::new(),
            player_right: Vec::new(),
            enemy: Vec::new(),
            sword_swings: vec![Vec::new(); 3],
            item_drops: Vec::new(),
        })
        .add_systems(Startup, load_assets)        
        //.add_systems(Startup, init_game.after(load_assets))
        .add_systems(Startup, init_game.after(bevy_stylus_plugin::init_blockchain))
        .add_systems(Startup, setup.after(init_game))
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


fn load_assets(
    asset_server: Res<AssetServer>,
    mut sprite_assets: ResMut<SpriteAssets>,
) {
    sprite_assets.player_up.push(asset_server.load("sprites/player/up_1.png"));
    sprite_assets.player_up.push(asset_server.load("sprites/player/up_2.png"));
    sprite_assets.player_down.push(asset_server.load("sprites/player/down_1.png"));
    sprite_assets.player_down.push(asset_server.load("sprites/player/down_2.png"));
    sprite_assets.player_left.push(asset_server.load("sprites/player/left_1.png"));
    sprite_assets.player_left.push(asset_server.load("sprites/player/left_2.png"));
    sprite_assets.player_right.push(asset_server.load("sprites/player/right_1.png"));
    sprite_assets.player_right.push(asset_server.load("sprites/player/right_2.png"));
    
    sprite_assets.enemy.push(asset_server.load("sprites/enemy/enemy_1.png"));
    sprite_assets.enemy.push(asset_server.load("sprites/enemy/enemy_2.png"));
    
    let color_names = ["red", "green", "blue"];
    let direction_names = ["up", "down", "left", "right"];
    
    for (_color_idx, color_name) in color_names.iter().enumerate() {
        sprite_assets.sword_swings[_color_idx] = Vec::new();
        for (dir_idx, dir_name) in direction_names.iter().enumerate() {
            for frame in 0..4 {
                let _sprite_idx = dir_idx * 4 + frame;
                sprite_assets.sword_swings[_color_idx].push(
                    asset_server.load(&format!("sprites/swords/{}_{}_{}.png", color_name, dir_name, frame + 1))
                );
            }
        }
    }
    
    for (_color_idx, color_name) in color_names.iter().enumerate() {
        sprite_assets.item_drops.push(asset_server.load(&format!("sprites/items/{}.png", color_name)));
    }
}

pub fn setup(mut commands: Commands, sprite_assets: Res<SpriteAssets>) {
    commands.spawn(Camera2dBundle::default());

    if sprite_assets.player_right.is_empty() {
        eprintln!("Warning: Sprite assets not loaded yet!");
        return;
    }

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
            animation_speed: 8.0,
            total_frames: 2,
            is_swinging: false,
        },
    ));

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
            game_state.last_direction = direction;
        }
        
        game_state.player_moving = is_moving;
        game_state.player_position = transform.translation;
    }
}

fn sword_swing_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut game_state: ResMut<GameState>,
) {
    if keyboard.just_pressed(KeyCode::Space) && !game_state.is_swinging {
        game_state.is_swinging = true;
        game_state.swing_frame = 0;
        game_state.swing_timer = 0.0;
    }
}

fn player_animation(
    mut player_query: Query<(&mut Handle<Image>, &mut AnimatedSprite), With<Player>>,
    mut game_state: ResMut<GameState>,
    sprite_assets: Res<SpriteAssets>,
    time: Res<Time>,
) {
    if let Ok((mut texture, mut animated_sprite)) = player_query.get_single_mut() {
        if sprite_assets.player_right.is_empty() {
            return;
        }
        
        if game_state.is_swinging {
            game_state.swing_timer += time.delta_seconds();
            
            if game_state.swing_timer >= 1.0 / 12.0 {
                game_state.swing_frame += 1;
                game_state.swing_timer = 0.0;
                
                if game_state.swing_frame >= 4 {
                    game_state.is_swinging = false;
                    animated_sprite.is_swinging = false;
                } else {
                    let direction_idx = match game_state.player_direction {
                        PlayerDirection::Up => 0,
                        PlayerDirection::Down => 1,
                        PlayerDirection::Left => 2,
                        PlayerDirection::Right => 3,
                    };
                    
                    let sprite_idx = direction_idx * 4 + game_state.swing_frame as usize;
                    
                    if (game_state.swing_color as usize) < sprite_assets.sword_swings.len() && 
                       sprite_idx < sprite_assets.sword_swings[game_state.swing_color as usize].len() {
                        *texture = sprite_assets.sword_swings[game_state.swing_color as usize][sprite_idx].clone();
                    }
                }
            }
        } else {
            animated_sprite.animation_timer += time.delta_seconds();
            
            let sprite_array = match game_state.player_direction {
                PlayerDirection::Up => &sprite_assets.player_up,
                PlayerDirection::Down => &sprite_assets.player_down,
                PlayerDirection::Left => &sprite_assets.player_left,
                PlayerDirection::Right => &sprite_assets.player_right,
            };
            
            if animated_sprite.current_frame as usize >= sprite_array.len() {
                return;
            }
            
            if game_state.player_moving && animated_sprite.animation_timer >= 1.0 / animated_sprite.animation_speed {
                animated_sprite.current_frame = (animated_sprite.current_frame + 1) % animated_sprite.total_frames;
                animated_sprite.animation_timer = 0.0;
            } else if !game_state.player_moving {
                animated_sprite.current_frame = 0;
            }
            
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
    if sprite_assets.enemy.is_empty() {
        return;
    }
    
    *timer += time.delta_seconds();
    if *timer >= ENEMY_SPAWN_RATE {
        *timer = 0.0;
        
        let screen_width = 1000.0;
        let screen_height = 800.0;
        
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
                animation_speed: 6.0,
                total_frames: 2,
                is_swinging: false,
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
        let distance_to_player = game_state.player_position.distance(enemy_transform.translation);
        
        // Keep enemies at a safe distance (120 pixels) from the player
        let safe_distance = 80.0;
        
        if distance_to_player > safe_distance {
            // Move towards player if too far
            enemy_transform.translation += direction * 100.0 * time.delta_seconds();
        } else if distance_to_player < safe_distance - 20.0 {
            // Move away from player if too close
            enemy_transform.translation -= direction * 100.0 * time.delta_seconds();
        }
        // If within the safe distance range, don't move
    }
}

fn enemy_animation(
    mut enemy_query: Query<(&mut Handle<Image>, &mut AnimatedSprite), With<Enemy>>,
    sprite_assets: Res<SpriteAssets>,
    time: Res<Time>,
) {
    if sprite_assets.enemy.is_empty() {
        return;
    }
    
    for (mut texture, mut animated_sprite) in enemy_query.iter_mut() {
        animated_sprite.animation_timer += time.delta_seconds();
        
        if animated_sprite.animation_timer >= 1.0 / animated_sprite.animation_speed {
            animated_sprite.current_frame = (animated_sprite.current_frame + 1) % animated_sprite.total_frames;
            animated_sprite.animation_timer = 0.0;
            
            if animated_sprite.current_frame as usize >= sprite_assets.enemy.len() {
                continue;
            }
            
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
    if sprite_assets.item_drops.is_empty() {
        return;
    }
    
    if !game_state.is_swinging || game_state.swing_frame < 1 || game_state.swing_frame > 2 {
        return;
    }
    
    let sword_offset = game_state.last_direction * 50.0;
    let sword_position = game_state.player_position + sword_offset;
    
    for (enemy_entity, enemy_transform) in enemy_query.iter() {
        let distance = sword_position.distance(enemy_transform.translation);
        if distance < 60.0 {
            commands.entity(enemy_entity).despawn();
            
            let color = rand::random::<u8>() % 3;
            
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
                ItemDrop,
            ));
        }
    }
}

fn update_ui(mut text_query: Query<&mut Text>, game_state: Res<GameState>) {
    if game_state.is_changed() {
        let mut color_counts = [0u32; 3];
        for &color in &game_state.swords_collected {
            color_counts[color as usize] += 1;
        }
        
        let color_names = ["Red", "Green", "Blue"];
        
        let mut display_text = format!("Total Swords: {}\n", game_state.swords_collected.len());
        for (name, count) in color_names.iter().zip(color_counts.iter()) {
            display_text.push_str(&format!("{}: {} ", name, count));
        }
        
        for mut text in text_query.iter_mut() {
            text.sections[0].value = display_text.clone();
        }
    }
}