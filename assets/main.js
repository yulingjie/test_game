// 游戏配置
var GAME_CONFIG = {
    // 注意：playerSpeed 已迁移到 PlayerShared.speed，由 Rust 侧初始化为 200.0
    // 此处保留 screenBounds，纯 JS 逻辑使用
    screenBounds: {
        xMin: -300,
        xMax: 300,
        yMin: -200,
        yMax: 200
    }
};

// ─── 内存映射初始化 ──────────────────────────────────────────────────────────
//
// rustApi.playerBufferPtr  : PlayerShared 结构体在 Rust 堆上的原始地址（usize）
// rustApi.playerBufferLen  : 结构体字节大小
// rustApi.OFFSET_*         : 各字段的字节偏移（由 Rust 编译期常量传入，无需硬编码）
//
// QuickJS 通过 ArrayBuffer.fromAddress(ptr, len) 直接映射 Rust 内存，
// 再用 Float32Array 按偏移读写，零拷贝、零序列化。

var playerBuffer = null;   // ArrayBuffer，映射 PlayerShared 原始内存
var playerF32    = null;   // Float32Array 视图，每个元素 = 4 字节

function initPlayerBuffer(ptr, len) {
    // QuickJS 扩展：从原始指针构造 ArrayBuffer（直接映射 Rust 内存，无拷贝）
    playerBuffer = ArrayBuffer.fromAddress(ptr, len);
    // Float32Array 视图：每个索引对应 4 字节，索引 = 字节偏移 / 4
    playerF32 = new Float32Array(playerBuffer);
    rustApi.log("内存映射初始化完成，PlayerShared 大小: " + len + " 字节");
}

// 字节偏移 → Float32Array 索引的辅助函数
function f32Index(byteOffset) {
    return byteOffset / 4;
}

// ─── PlayerShared 字段读写封装 ───────────────────────────────────────────────
// 按 Rust 侧 OFFSET_* 常量访问，字段增减只需改 Rust 结构体，JS 自动适配

var Player = {
    get x()     { return playerF32[f32Index(rustApi.OFFSET_X)]; },
    set x(v)    { playerF32[f32Index(rustApi.OFFSET_X)] = v; },

    get y()     { return playerF32[f32Index(rustApi.OFFSET_Y)]; },
    set y(v)    { playerF32[f32Index(rustApi.OFFSET_Y)] = v; },

    get speed() { return playerF32[f32Index(rustApi.OFFSET_SPEED)]; },
    set speed(v){ playerF32[f32Index(rustApi.OFFSET_SPEED)] = v; },

    // skill_cd: [f32; 8]，返回指定技能槽的冷却时间
    getSkillCd: function(slot) {
        return playerF32[f32Index(rustApi.OFFSET_SKILL_CD) + slot];
    },
    setSkillCd: function(slot, value) {
        playerF32[f32Index(rustApi.OFFSET_SKILL_CD) + slot] = value;
    },

    // buff_values: [f32; 4]，返回指定 buff 槽的强度
    getBuffValue: function(slot) {
        return playerF32[f32Index(rustApi.OFFSET_BUFF_VALUES) + slot];
    },
    setBuffValue: function(slot, value) {
        playerF32[f32Index(rustApi.OFFSET_BUFF_VALUES) + slot] = value;
    },
};

// ─── 初始化内存映射（JS 文件加载时立即执行）────────────────────────────────
initPlayerBuffer(rustApi.playerBufferPtr, rustApi.playerBufferLen);

// ─── 游戏逻辑函数 ────────────────────────────────────────────────────────────

// 计算玩家速度（从 PlayerShared.speed 读取，而非硬编码）
function calculatePlayerVelocity(keyboardState) {
    var speed = Player.speed;  // 直接读 Rust 内存，无函数调用开销
    var velocity = { x: 0, y: 0 };
    if (keyboardState.right)  velocity.x += speed;
    if (keyboardState.left)   velocity.x -= speed;
    if (keyboardState.up)     velocity.y += speed;
    if (keyboardState.down)   velocity.y -= speed;
    return velocity;
}

// 应用边界检测
function applyBounds(position) {
    return {
        x: rustApi.clamp(position.x, GAME_CONFIG.screenBounds.xMin, GAME_CONFIG.screenBounds.xMax),
        y: rustApi.clamp(position.y, GAME_CONFIG.screenBounds.yMin, GAME_CONFIG.screenBounds.yMax)
    };
}

/**
 * 核心更新函数：Rust 每帧调用一次
 * args: { keyboard: {right,left,up,down}, position: {x,y}, deltaTime: number }
 * 返回: {}（位置数据直接写入 PlayerShared 内存，无需通过返回值传递）
 */
function updateGame(args) {
    var velocity = calculatePlayerVelocity(args.keyboard);
    var newPos = {
        x: args.position.x + velocity.x * args.deltaTime,
        y: args.position.y + velocity.y * args.deltaTime
    };
    var bounded = applyBounds(newPos);

    // 撞到边界时打印日志
    if (bounded.x !== newPos.x || bounded.y !== newPos.y) {
        rustApi.log("撞到边界！位置: " + bounded.x + ", " + bounded.y);
    }

    // ── 直接写入 Rust 内存，零函数调用开销 ──────────────────────────────────
    Player.x = bounded.x;
    Player.y = bounded.y;

    // 示例：技能冷却倒计时（每帧减少 deltaTime）
    for (var i = 0; i < 8; i++) {
        var cd = Player.getSkillCd(i);
        if (cd > 0) {
            Player.setSkillCd(i, Math.max(0, cd - args.deltaTime));
        }
    }

    return {};
}

/**
 * 键盘映射函数：将原始按键状态映射为游戏逻辑键位
 * args: { right, left, up, down }（bool）
 * 返回: { right, left, up, down }
 */
function processKeyboard(args) {
    return {
        right: args.right ? 1.0 : 0.0,
        left:  args.left  ? 1.0 : 0.0,
        up:    args.up    ? 1.0 : 0.0,
        down:  args.down  ? 1.0 : 0.0,
    };
}

// 暴露给 Rust 调用的 API
globalThis.gameLogic = {
    processKeyboard: processKeyboard,
    updateGame: updateGame,
};