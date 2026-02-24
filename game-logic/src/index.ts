// ─── game-logic/src/index.ts ─────────────────────────────────────────────────
//
// 路径 B 的 TypeScript 侧实现。
//
// 关键点：
//   1. jco 根据 wit/game.wit 自动生成类型定义（KeyboardInput、UpdateResult 等）
//      你不需要手写任何类型，直接 import 即可
//   2. 函数签名由 WIT 约束，参数类型错误会在 tsc 编译期报错
//   3. `import { spawnPanel, spawnText, despawn, setVisible, log } from 'bevy:api'`
//      这些函数由 Bevy 实现，jco 自动生成调用胶水代码
//
// 对比原来的 main.js：
//   原来：rustApi.spawnPanel({ x, y, w, h, r, g, b, a })  ← 无类型检查，运行时崩溃
//   现在：spawnPanel({ x, y, width, height, ... })         ← 编译期类型检查

// ─── 导入 Bevy 提供的能力（对应 WIT 的 import bevy-api）────────────────────
// jco componentize 会根据 WIT 自动生成这些导入的胶水代码
// 类型定义由 jco 从 WIT 自动推导，无需手写
import {
    spawnPanel,
    spawnText,
    despawn,
    setVisible,
    log,
} from 'bevy:api/bevy-api';

// ─── WIT 生成的类型（jco 自动推导，无需手写）────────────────────────────────
// 这些类型与 wit/game.wit 中的 record 定义完全对应
// 如果 WIT 中修改了字段，tsc 会在这里报错，强制你同步更新

export interface KeyboardInput {
    right: boolean;
    left:  boolean;
    up:    boolean;
    down:  boolean;
}

export interface UpdateResult {
    x: number;
    y: number;
}

export interface PlayerState {
    x:     number;
    y:     number;
    speed: number;
}

export interface PanelConfig {
    key:     string;
    x:       number;
    y:       number;
    width:   number;
    height:  number;
    colorR:  number;
    colorG:  number;
    colorB:  number;
    colorA:  number;
}

export interface TextConfig {
    key:       string;
    parentKey: string;   // WIT 的 parent-key → camelCase parentKey
    text:      string;
    fontSize:  number;
    colorR:    number;
    colorG:    number;
    colorB:    number;
}

// ─── 游戏配置（TypeScript 原生，无需任何桥接）───────────────────────────────
const GAME_CONFIG = {
    playerSpeed:  200.0,
    boundaryX:    300.0,
    boundaryY:    200.0,
} as const;

// ─── UI 管理器 ────────────────────────────────────────────────────────────────
//
// 路径 B 的核心体验：TS 直接调用 Bevy 的 spawnPanel/spawnText，
// 类型完全由 WIT 约束，参数错误在 tsc 编译期报错，不会出现运行时崩溃。
//
// 对比路径 A（AssemblyScript）：
//   路径 A：手动操作 WASM 线性内存，需要记住字节偏移
//   路径 B：直接调用强类型函数，IDE 有完整的自动补全和类型提示

class UiManager {
    private panelKey: string = 'main_panel';
    /**
     * isCreated：实体是否已经通过 spawnPanel 创建（命令已入队）
     * isVisible：当前是否处于可见状态
     * 两者分离，避免「已创建但隐藏」与「未创建」混淆
     */
    private isCreated: boolean = false;
    private isVisible: boolean = false;

    /**
     * 创建并显示信息面板
     * 所有参数类型由 WIT 约束，tsc 编译期检查
     */
    showPanel(playerState: PlayerState): void {
        if (this.isCreated) {
            if (this.isVisible) {
                // 面板已创建且已可见，无需重复操作
                log('UI 面板已处于可见状态，跳过');
                return;
            }
            // 面板已创建但当前隐藏，直接恢复可见
            setVisible(this.panelKey, true);
            this.isVisible = true;
            log('UI 面板已恢复显示（复用已有实体）');
            return;
        }

        log('正在创建 UI 面板...');

        // ── 调用 Bevy 的 spawn-panel（WIT 强类型，编译期检查）────────────────
        // 使用 string key 声明式引用，不再依赖 entity ID
        spawnPanel({
            key:     this.panelKey,
            x:      -120,
            y:       20,
            width:   260,
            height:  200,
            colorR:  0.05,
            colorG:  0.05,
            colorB:  0.15,
            colorA:  0.92,
        });
        // 命令已入队，标记为已创建；实体尚未真正写入 World，但后续调用可安全复用
        this.isCreated = true;
        this.isVisible = true;

        // ── 添加文字节点（WIT 强类型）────────────────────────────────────────
        // parentKey 引用上面创建的面板 key
        spawnText({
            key:       `${this.panelKey}.title`,
            parentKey: this.panelKey,
            text:      '=== 游戏信息 ===',
            fontSize:  20.0,
            colorR:    1.0, colorG: 1.0, colorB: 1.0,
        });

        spawnText({
            key:       `${this.panelKey}.position`,
            parentKey: this.panelKey,
            text:      `玩家位置: (${playerState.x.toFixed(1)}, ${playerState.y.toFixed(1)})`,
            fontSize:  14.0,
            colorR:    0.8, colorG: 0.9, colorB: 1.0,
        });

        spawnText({
            key:       `${this.panelKey}.speed`,
            parentKey: this.panelKey,
            text:      `移动速度: ${playerState.speed.toFixed(0)}`,
            fontSize:  14.0,
            colorR:    0.8, colorG: 1.0, colorB: 0.8,
        });

        spawnText({
            key:       `${this.panelKey}.divider`,
            parentKey: this.panelKey,
            text:      '─────────────────',
            fontSize:  12.0,
            colorR:    0.5, colorG: 0.5, colorB: 0.5,
        });

        spawnText({
            key:       `${this.panelKey}.hint1`,
            parentKey: this.panelKey,
            text:      '方向键：移动角色',
            fontSize:  13.0,
            colorR:    0.9, colorG: 0.9, colorB: 0.6,
        });

        spawnText({
            key:       `${this.panelKey}.hint2`,
            parentKey: this.panelKey,
            text:      'E 键：打开/关闭面板',
            fontSize:  13.0,
            colorR:    0.9, colorG: 0.9, colorB: 0.6,
        });

        log(`UI 面板创建完成，key: ${this.panelKey}`);
    }

    hidePanel(): void {
        if (!this.isCreated || !this.isVisible) return;
        setVisible(this.panelKey, false);
        this.isVisible = false;
        log('UI 面板已隐藏');
    }

    destroyPanel(): void {
        if (!this.isCreated) return;
        despawn(this.panelKey);
        // 销毁后重置两个状态，下次 showPanel 会重新创建
        this.isCreated = false;
        this.isVisible = false;
        log('UI 面板已销毁');
    }

    togglePanel(playerState: PlayerState): void {
        if (this.isVisible) {
            this.hidePanel();
        } else {
            this.showPanel(playerState);
        }
    }
}

// ─── 模块级状态 ───────────────────────────────────────────────────────────────
// WASM Component 是单例，模块级变量在整个生命周期内持久

const uiManager = new UiManager();

// 缓存最新的玩家状态，供 onUiEvent 使用
let lastPlayerState: PlayerState = { x: 0, y: 0, speed: GAME_CONFIG.playerSpeed };

// ─── 导出函数（对应 WIT 的 export game-logic）────────────────────────────────
//
// 这些函数由 Bevy 调用，函数签名由 WIT 约束。
// 如果签名与 WIT 不匹配，jco componentize 会报错。

/**
 * 键盘映射：将原始按键状态转换为游戏方向
 * 对应 WIT: process-keyboard: func(input: keyboard-input) -> keyboard-input
 *
 * 在 TypeScript 中可以做任意按键重映射逻辑，例如：
 *   - WASD → 方向键
 *   - 手柄摇杆 → 方向
 *   - 按键组合 → 特殊方向
 */
export function processKeyboard(input: KeyboardInput): KeyboardInput {
    // 直接透传（可在此处添加按键重映射逻辑）
    return {
        right: input.right,
        left:  input.left,
        up:    input.up,
        down:  input.down,
    };
}

/**
 * 游戏逻辑更新：根据键盘状态和 delta 时间计算新位置
 * 对应 WIT: update-game: func(keyboard, state, delta) -> update-result
 *
 * 对比原来的 main.js：
 *   原来：function updateGame(args) { ... args.keyboard.right ... }  ← 无类型
 *   现在：function updateGame(keyboard, state, delta) { ... }        ← 强类型
 */
export function updateGame(
    keyboard: KeyboardInput,
    state:    PlayerState,
    delta:    number,
): UpdateResult {
    let vx = 0;
    let vy = 0;

    if (keyboard.right) vx += state.speed;
    if (keyboard.left)  vx -= state.speed;
    if (keyboard.up)    vy += state.speed;
    if (keyboard.down)  vy -= state.speed;

    const newX = Math.max(
        -GAME_CONFIG.boundaryX,
        Math.min(GAME_CONFIG.boundaryX, state.x + vx * delta)
    );
    const newY = Math.max(
        -GAME_CONFIG.boundaryY,
        Math.min(GAME_CONFIG.boundaryY, state.y + vy * delta)
    );

    // 更新缓存的玩家状态
    lastPlayerState = { x: newX, y: newY, speed: state.speed };

    return { x: newX, y: newY };
}

/**
 * UI 事件回调：Bevy 检测到按键等事件时调用
 * 对应 WIT: on-ui-event: func(event-type: string)
 *
 * TS 侧完全控制 UI 逻辑，Bevy 只负责触发事件
 */
export function onUiEvent(eventType: string): void {
    log(`收到 UI 事件: ${eventType}`);

    switch (eventType) {
        case 'toggle_panel':
            uiManager.togglePanel(lastPlayerState);
            break;
        case 'close_panel':
            uiManager.hidePanel();
            break;
        case 'destroy_panel':
            uiManager.destroyPanel();
            break;
        default:
            log(`未知事件类型: ${eventType}`);
    }
}
