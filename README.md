# Bevy + TypeScript WASM Component Model 游戏框架

用 **Rust / Bevy** 作为游戏引擎，用 **TypeScript** 编写游戏逻辑，通过 **WASM Component Model（WIT）** 将两者强类型连接。

## 架构概览

```
┌─────────────────────────────────────────────────────┐
│                   Bevy（Rust）                       │
│  ECS 渲染 / 输入 / UI 实体管理                        │
│                    ↕ wasmtime                        │
│              WIT 接口契约（game.wit）                 │
│                    ↕ jco componentize                │
│              TypeScript 游戏逻辑                      │
│  键盘映射 / 位置更新 / UI 事件 / 面板管理              │
└─────────────────────────────────────────────────────┘
```

- **`wit/game.wit`** — 唯一真相来源，定义 Bevy ↔ TS 的全部接口
- **`game-logic/`** — TypeScript 实现，编译为 WASM Component
- **`src/main.rs`** — Bevy 宿主，通过 `wasmtime::component::bindgen!` 零手写桥接调用 TS

## 项目结构

```
test_game/
├── wit/
│   └── game.wit              # WIT 接口契约（唯一真相来源）
├── game-logic/               # TypeScript 游戏逻辑
│   ├── src/
│   │   ├── index.ts          # 游戏逻辑实现
│   │   └── types/
│   │       └── bevy-api.d.ts # Bevy Host API 类型声明
│   ├── package.json
│   └── tsconfig.json
├── src/
│   └── main.rs               # Bevy 宿主 + wasmtime 运行时
├── assets/
│   └── game_logic.wasm       # TS 编译产物（由 npm run build 生成）
├── build.rs                  # 编译期 WIT 变更检测
└── Cargo.toml
```

## 环境依赖

| 工具 | 版本要求 | 说明 |
|------|----------|------|
| Rust | stable | `rustup update` |
| Node.js | ≥ 18 | 编译 TypeScript |
| jco | 自动安装 | `@bytecodealliance/jco` |

## 快速开始

### 1. 安装 TypeScript 依赖

```bash
cd game-logic
npm install
```

### 2. 编译 TypeScript → WASM Component

```bash
# 在 game-logic/ 目录下
npm run build
# 产物输出到 ../assets/game_logic.wasm
```

### 3. 运行游戏

```bash
# 在项目根目录
cargo run
```

## 游戏操作

| 按键 | 功能 |
|------|------|
| `↑ ↓ ← →` | 控制蓝色方块移动 |
| `E` | 打开 / 关闭 UI 面板 |

## 开发工作流

修改不同层级时，需要执行的步骤：

| 修改内容 | 需要执行 |
|----------|----------|
| `wit/game.wit` | 同步更新 `bevy-api.d.ts` + `main.rs` → `npm run build` → `cargo run` |
| `game-logic/src/index.ts` | `npm run build` → `cargo run` |
| `src/main.rs` | `cargo run` |

> 详细设计文档见 [DESIGN.md](./DESIGN.md)

## 技术栈

- **Bevy 0.13** — ECS 游戏引擎
- **wasmtime** — WASM Component Model 运行时
- **WIT（WebAssembly Interface Types）** — 跨语言接口契约
- **jco（@bytecodealliance/jco）** — TypeScript → WASM Component 编译器
- **TypeScript** — 游戏逻辑脚本语言
- **Rust 2021 Edition**

## 许可证

MIT License