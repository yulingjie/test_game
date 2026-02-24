# Bevy + rquickjs 游戏示例

这是一个使用 Rust 的 Bevy 游戏引擎和 rquickjs JavaScript 引擎构建的简单游戏示例。

## 功能特性

- ✅ **Bevy 游戏引擎**: 现代化的 ECS 架构游戏引擎
- ✅ **rquickjs JavaScript 引擎**: 集成 QuickJS JavaScript 引擎
- ✅ **键盘控制**: 使用方向键控制蓝色方块移动
- ✅ **边界检测**: 自动限制方块在屏幕范围内移动
- ✅ **定时 JavaScript 执行**: 每2秒执行一次 JavaScript 代码

## 项目结构

```
test_game/
├── Cargo.toml          # Rust 项目配置文件
├── src/
│   └── main.rs         # 主程序文件
└── README.md           # 项目说明文档
```

## 运行方法

1. 确保已安装 Rust 和 Cargo
2. 在项目目录下运行：
   ```bash
   cargo run
   ```

## 游戏操作

- **方向键 ↑↓←→**: 控制蓝色方块移动
- **JavaScript 执行**: 每2秒自动执行 JavaScript 代码并输出结果

## 技术栈

- **Bevy 0.13**: 游戏引擎
- **rquickjs 0.5**: JavaScript 引擎
- **Rust 2021 Edition**: 编程语言

## 代码示例

### JavaScript 集成

游戏每2秒执行一次 JavaScript 代码：

```rust
fn execute_javascript() {
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();
    
    ctx.with(|ctx| {
        // 执行数学计算
        let result: i32 = ctx.eval("2 + 3 * 4").unwrap();
        println!("JavaScript计算结果: {}", result);
        
        // 执行自定义函数
        let greeting: String = ctx.eval("greet('Bevy Game')").unwrap();
        println!("{}", greeting);
    });
}
```

### 游戏实体系统

使用 Bevy 的 ECS 系统管理游戏实体：

```rust
#[derive(Component)]
struct Player;

#[derive(Component)]
struct Velocity(Vec2);

fn move_player(time: Res<Time>, mut query: Query<(&mut Transform, &Velocity), With<Player>>) {
    for (mut transform, velocity) in query.iter_mut() {
        transform.translation.x += velocity.0.x * time.delta_seconds();
        transform.translation.y += velocity.0.y * time.delta_seconds();
    }
}
```

## 扩展建议

这个项目可以作为以下功能的起点：

1. **游戏逻辑脚本化**: 使用 JavaScript 编写游戏逻辑
2. **热重载**: 动态加载和更新 JavaScript 代码
3. **UI 集成**: 添加游戏界面和交互元素
4. **物理引擎**: 集成更复杂的物理效果
5. **网络功能**: 添加多人游戏支持

## 许可证

MIT License