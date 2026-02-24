// ─── game-logic/src/types/bevy-api.d.ts ──────────────────────────────────────
//
// 为 tsc 提供 `bevy:api/bevy-api` 模块的类型声明。
//
// 注意：
//   - 此文件仅用于 tsc 编译期类型检查，不参与运行时。
//   - 实际实现由 jco componentize 根据 wit/game.wit 自动生成胶水代码。
//   - 字段类型与 wit/game.wit 中的 bevy-api interface 完全对应。
//   - 若 WIT 修改，需同步更新此文件（或改用 jco typegen 自动生成）。

declare module 'bevy:api/bevy-api' {
    /** 对应 WIT: record panel-config */
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

    /** 对应 WIT: record text-config */
    export interface TextConfig {
        key:       string;
        parentKey: string;
        text:      string;
        fontSize:  number;
        colorR:    number;
        colorG:    number;
        colorB:    number;
    }

    /** 对应 WIT: spawn-panel: func(config: panel-config) */
    export function spawnPanel(config: PanelConfig): void;

    /** 对应 WIT: spawn-text: func(config: text-config) */
    export function spawnText(config: TextConfig): void;

    /** 对应 WIT: despawn: func(key: string) */
    export function despawn(key: string): void;

    /** 对应 WIT: set-visible: func(key: string, visible: bool) */
    export function setVisible(key: string, visible: boolean): void;

    /** 对应 WIT: log: func(msg: string) */
    export function log(msg: string): void;
}
