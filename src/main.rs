// ─── 路径 B：TypeScript + WIT（WASM Component Model）────────────────────────
//
// 架构说明：
//   1. wit/game.wit  → 接口契约（唯一真相来源）
//   2. wasmtime::component::bindgen! 宏读取 WIT，在编译期自动生成：
//        - 强类型结构体（KeyboardInput、UpdateResult、PlayerState、PanelConfig、TextConfig）
//        - Bevy 需要实现的 Host trait（game::logic::bevy_api::Host）
//        - Guest 调用句柄（通过 GameWorld.interface0.call_xxx）
//   3. TypeScript 实现 game-logic 接口，jco componentize 编译为 WASM Component
//   4. Bevy System 直接调用 Guest 的强类型方法，零手写桥接代码

use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use std::collections::HashMap;
use std::time::Duration;
use wasmtime::component::{bindgen, Component, Linker};
use wasmtime::{Config, Engine, Store};

// ─── WIT 绑定生成 ─────────────────────────────────────────────────────────────
//
// bindgen! 读取 wit/game.wit，自动生成全部类型和 trait，
// 彻底消除手写 build_args / parse_output 桥接代码。

bindgen!({
    world: "game-world",
    path: "wit/game.wit",
});

// 引入 bindgen! 生成的类型
use game::logic::bevy_api::{Host as BevyApiHost, PanelConfig, TextConfig};
use exports::game::logic::game_logic::{KeyboardInput, PlayerState};

// ─── UI 命令队列 ─────────────────────────────────────────────────────────────
//
// TS 调用 bevy-api 时写入此队列；Bevy system 在主线程消费，
// 真正操作 ECS，保证线程安全。
// 拆分为 Spawn 命令和 Mutation 命令两类，分别由不同 system 处理。

#[derive(Debug)]
enum UiSpawnCommand {
    SpawnPanel {
        key: String,
        x: f32, y: f32, width: f32, height: f32,
        color_r: f32, color_g: f32, color_b: f32, color_a: f32,
    },
    SpawnText {
        key: String,
        parent_key: String,
        text: String, font_size: f32,
        color_r: f32, color_g: f32, color_b: f32,
    },
}

#[derive(Debug)]
enum UiMutationCommand {
    Despawn    { key: String },
    SetVisible { key: String, visible: bool },
}

// ─── wasmtime Store 的 Host 数据 ──────────────────────────────────────────────

struct HostState {
    /// TS 调用 bevy-api 时写入的 Spawn 命令队列
    spawn_commands: Vec<UiSpawnCommand>,
    /// TS 调用 bevy-api 时写入的 Mutation 命令队列
    mutation_commands: Vec<UiMutationCommand>,
}

// ─── 实现 WIT 生成的 bevy-api Host trait ──────────────────────────────────────

impl BevyApiHost for HostState {
    fn spawn_panel(&mut self, config: PanelConfig) -> wasmtime::Result<()> {
        self.spawn_commands.push(UiSpawnCommand::SpawnPanel {
            key: config.key,
            x: config.x, y: config.y,
            width: config.width, height: config.height,
            color_r: config.color_r, color_g: config.color_g,
            color_b: config.color_b, color_a: config.color_a,
        });
        Ok(())
    }

    fn spawn_text(&mut self, config: TextConfig) -> wasmtime::Result<()> {
        self.spawn_commands.push(UiSpawnCommand::SpawnText {
            key: config.key,
            parent_key: config.parent_key,
            text:      config.text,
            font_size: config.font_size,
            color_r:   config.color_r,
            color_g:   config.color_g,
            color_b:   config.color_b,
        });
        Ok(())
    }

    fn despawn(&mut self, key: String) -> wasmtime::Result<()> {
        self.mutation_commands.push(UiMutationCommand::Despawn { key });
        Ok(())
    }

    fn set_visible(&mut self, key: String, visible: bool) -> wasmtime::Result<()> {
        self.mutation_commands.push(UiMutationCommand::SetVisible { key, visible });
        Ok(())
    }

    fn log(&mut self, msg: String) -> wasmtime::Result<()> {
        // 使用 debug! 避免生产环境性能损耗，发布时自动关闭
        bevy::log::debug!("[TS] {}", msg);
        Ok(())
    }
}

// ─── WASM 运行时（NonSend Resource）───────────────────────────────────────────
//
// 去掉 Arc<Mutex<>>，直接持有 wasmtime 运行时。
// 通过 Bevy 的 NonSend 调度机制保证线程安全，零锁开销。

struct WasmRuntime {
    store: Store<HostState>,
    /// WIT 生成的 GameWorld，通过 interface0 字段访问 Guest 调用句柄
    game_world: GameWorld,
}

// ─── UI 命令中转 Resource ─────────────────────────────────────────────────────
//
// wasm_tick 产出的 UI 命令先存放在此 Resource 中，
// process_ui_spawn / process_ui_mutations 从此处消费，完全不接触 WasmRuntime。

#[derive(Resource, Default)]
struct PendingUiCommands {
    spawns:    Vec<UiSpawnCommand>,
    mutations: Vec<UiMutationCommand>,
}

// ─── UI Key → Entity 映射表 Resource ──────────────────────────────────────────

/// TS 用 string key 引用实体，Rust 侧维护 key → Entity 映射
#[derive(Resource, Default)]
struct UiEntityMap {
    map: HashMap<String, Entity>,
}

// ─── 初始化 WASM 运行时 ───────────────────────────────────────────────────────

fn init_wasm() -> WasmRuntime {
    let wasm_bytes = std::fs::read("assets/game_logic.wasm")
        .expect("无法读取 assets/game_logic.wasm，请先运行 npm run build");

    // 启用 Component Model
    let mut config = Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config).expect("创建 wasmtime Engine 失败");

    // 构建 Linker：注册 bevy-api import 实现
    let mut linker: Linker<HostState> = Linker::new(&engine);

    // bindgen! 生成的函数：将 HostState 的 Host impl 注册到 Linker
    GameWorld::add_to_linker(&mut linker, |state: &mut HostState| state)
        .expect("注册 bevy-api 到 Linker 失败");

    let host_state = HostState {
        spawn_commands:    Vec::new(),
        mutation_commands: Vec::new(),
    };

    let mut store = Store::new(&engine, host_state);

    // 加载 WASM Component（TypeScript 编译产物）
    let component = Component::new(&engine, &wasm_bytes)
        .expect("WASM Component 解析失败");

    // 实例化：WIT 生成的 GameWorld::instantiate 替代手动 linker.instantiate
    let (game_world, _instance) = GameWorld::instantiate(&mut store, &component, &linker)
        .expect("WASM Component 实例化失败");

    println!("[WASM] Component Model 初始化完成");

    WasmRuntime { store, game_world }
}

// ─── Bevy 游戏状态 ────────────────────────────────────────────────────────────

#[derive(Resource)]
struct GameState {
    player_position: Vec2,
    player_speed:    f32,
}

#[derive(Component)]
struct Player;

/// 标记：该实体是由 TS 通过 bevy-api 创建的 UI 根面板
#[derive(Component)]
struct WitUiPanel;

// ─── Bevy 入口 ────────────────────────────────────────────────────────────────

fn main() {
    let wasm_runtime = init_wasm();

    App::new()
        .add_plugins(DefaultPlugins)
        // 关键：用 non_send 注册，Bevy 调度器保证线程安全，无需 Mutex
        .insert_non_send_resource(wasm_runtime)
        .init_resource::<UiEntityMap>()
        .init_resource::<PendingUiCommands>()
        .add_systems(Startup, setup)
        .add_systems(Update, (
            wasm_tick,            // 唯一接触 WASM 的系统，零锁开销
            process_ui_spawn,     // 只读 PendingUiCommands，不接触 WASM
            apply_deferred,
            process_ui_mutations, // 只读 PendingUiCommands，不接触 WASM
        ).chain())
        .add_systems(Update, debug_game_state.run_if(on_timer(Duration::from_secs(3))))
        .run();
}

// ─── Bevy Systems ─────────────────────────────────────────────────────────────

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::BLUE,
                custom_size: Some(Vec2::new(50.0, 50.0)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::ZERO),
            ..default()
        },
        Player,
    ));

    commands.insert_resource(GameState {
        player_position: Vec2::ZERO,
        player_speed:    200.0,
    });

    println!("游戏初始化完成！按 E 键打开/关闭 UI 面板");
}

/// 统一 WASM 调用系统
/// 一帧只访问一次 WasmRuntime（NonSendMut），零锁开销。
/// 将键盘处理、位置更新、UI 事件全部收拢在此。
fn wasm_tick(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut game_state: ResMut<GameState>,
    mut query: Query<&mut Transform, With<Player>>,
    mut wasm: NonSendMut<WasmRuntime>,
    mut pending: ResMut<PendingUiCommands>,
) {
    let WasmRuntime { ref game_world, ref mut store } = *wasm;

    // ① 键盘输入处理（processKeyboard 结果直接使用，无需中转存储）
    let raw_input = KeyboardInput {
        right: keyboard_input.pressed(KeyCode::ArrowRight),
        left:  keyboard_input.pressed(KeyCode::ArrowLeft),
        up:    keyboard_input.pressed(KeyCode::ArrowUp),
        down:  keyboard_input.pressed(KeyCode::ArrowDown),
    };

    let keyboard = match game_world.interface0.call_process_keyboard(&mut *store, raw_input) {
        Ok(mapped) => mapped,
        Err(e) => {
            eprintln!("[键盘映射] WASM 错误: {}", e);
            // 映射失败时降级使用原始输入，保证游戏不卡死
            KeyboardInput {
                right: keyboard_input.pressed(KeyCode::ArrowRight),
                left:  keyboard_input.pressed(KeyCode::ArrowLeft),
                up:    keyboard_input.pressed(KeyCode::ArrowUp),
                down:  keyboard_input.pressed(KeyCode::ArrowDown),
            }
        }
    };

    // ② 玩家位置更新
    let state = PlayerState {
        x:     game_state.player_position.x,
        y:     game_state.player_position.y,
        speed: game_state.player_speed,
    };

    match game_world.interface0.call_update_game(
        &mut *store,
        keyboard,
        state,
        time.delta_seconds(),
    ) {
        Ok(result) => {
            game_state.player_position = Vec2::new(result.x, result.y);
            for mut transform in query.iter_mut() {
                transform.translation.x = result.x;
                transform.translation.y = result.y;
            }
        }
        Err(e) => eprintln!("[位置更新] WASM 错误: {}", e),
    }

    // ③ UI 事件（E 键切换面板）
    if keyboard_input.just_pressed(KeyCode::KeyE) {
        match game_world.interface0.call_on_ui_event(&mut *store, "toggle_panel") {
            Ok(()) => {}
            Err(e) => eprintln!("[UI事件] WASM 错误: {}", e),
        }
    }

    // ④ 将本帧产生的 UI 命令转移到 PendingUiCommands，供后续 system 消费
    pending.spawns.extend(store.data_mut().spawn_commands.drain(..));
    pending.mutations.extend(store.data_mut().mutation_commands.drain(..));
}

/// 消费 Spawn 命令，创建实体，注册 key → Entity 映射
/// 只访问 PendingUiCommands，完全不接触 WasmRuntime
fn process_ui_spawn(
    mut commands: Commands,
    mut pending: ResMut<PendingUiCommands>,
    asset_server: Res<AssetServer>,
    mut entity_map: ResMut<UiEntityMap>,
) {
    let cmds: Vec<_> = pending.spawns.drain(..).collect();

    if cmds.is_empty() {
        return;
    }

    for cmd in cmds {
        match cmd {
            UiSpawnCommand::SpawnPanel { key, x, y, width, height, color_r, color_g, color_b, color_a } => {
                let entity = commands.spawn((
                    NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            left:   Val::Px(x + 400.0),
                            bottom: Val::Px(y + 300.0),
                            width:  Val::Px(width),
                            height: Val::Px(height),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            padding: UiRect::all(Val::Px(12.0)),
                            row_gap: Val::Px(8.0),
                            ..default()
                        },
                        background_color: Color::rgba(color_r, color_g, color_b, color_a).into(),
                        ..default()
                    },
                    WitUiPanel,
                )).id();

                entity_map.map.insert(key.clone(), entity);
                println!("[UI] 创建面板 key={} entity={:?}", key, entity);
            }

            UiSpawnCommand::SpawnText { key, parent_key, text, font_size, color_r, color_g, color_b } => {
                let parent_entity = match entity_map.map.get(&parent_key) {
                    Some(&e) => e,
                    None => {
                        eprintln!("[UI] SpawnText 失败：找不到父实体 key={}", parent_key);
                        continue;
                    }
                };
                let text_entity = commands.spawn(
                    TextBundle::from_section(
                        text,
                        TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size,
                            color: Color::rgb(color_r, color_g, color_b),
                        },
                    )
                ).id();
                commands.entity(parent_entity).add_child(text_entity);
                entity_map.map.insert(key.clone(), text_entity);
                println!("[UI] 创建文字 key={} entity={:?}", key, text_entity);
            }
        }
    }
}

/// 消费 Mutation 命令（despawn / set-visible），通过 key 查映射表操作实体
/// 运行在 apply_deferred 之后，保证 process_ui_spawn 创建的实体已真正写入 World
/// 只访问 PendingUiCommands，完全不接触 WasmRuntime
fn process_ui_mutations(
    mut commands: Commands,
    mut pending: ResMut<PendingUiCommands>,
    mut entity_map: ResMut<UiEntityMap>,
) {
    let cmds: Vec<_> = pending.mutations.drain(..).collect();

    if cmds.is_empty() {
        return;
    }

    for cmd in cmds {
        match cmd {
            UiMutationCommand::Despawn { key } => {
                if let Some(entity) = entity_map.map.remove(&key) {
                    commands.entity(entity).despawn_recursive();
                    let prefix = format!("{}.", key);
                    entity_map.map.retain(|k, _| !k.starts_with(&prefix));
                    println!("[UI] 销毁实体 key={}", key);
                } else {
                    eprintln!("[UI] Despawn 失败：找不到 key={}", key);
                }
            }

            UiMutationCommand::SetVisible { key, visible } => {
                if let Some(&entity) = entity_map.map.get(&key) {
                    // 使用 Visibility 组件控制显隐，避免覆盖 Style 导致布局丢失
                    let visibility = if visible { Visibility::Visible } else { Visibility::Hidden };
                    commands.entity(entity).insert(visibility);
                    println!("[UI] 设置可见性 key={} visible={}", key, visible);
                } else {
                    eprintln!("[UI] SetVisible 失败：找不到 key={}", key);
                }
            }
        }
    }
}

fn debug_game_state(game_state: Res<GameState>) {
    println!(
        "游戏状态 - 位置: ({:.1}, {:.1})",
        game_state.player_position.x,
        game_state.player_position.y,
    );
}