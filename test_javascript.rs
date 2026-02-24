use rquickjs::{Context, Runtime};
use std::fs;

fn main() {
    println!("开始测试JavaScript执行...");
    
    // 读取JavaScript文件
    let js_code = match fs::read_to_string("assets/game_logic.js") {
        Ok(code) => code,
        Err(e) => {
            eprintln!("无法读取文件: {}", e);
            return;
        }
    };
    
    println!("JavaScript代码长度: {} 字符", js_code.len());
    
    // 创建运行时和上下文
    let rt = match Runtime::new() {
        Ok(runtime) => runtime,
        Err(e) => {
            eprintln!("无法创建运行时: {}", e);
            return;
        }
    };
    
    let ctx = match Context::full(&rt) {
        Ok(context) => context,
        Err(e) => {
            eprintln!("无法创建上下文: {}", e);
            return;
        }
    };
    
    // 测试执行
    ctx.with(|ctx| {
        println!("开始执行JavaScript代码...");
        
        // 先执行简单的测试
        match ctx.eval::<i32, _>("2 + 3 * 4") {
            Ok(result) => println!("简单计算测试成功: {}", result),
            Err(e) => eprintln!("简单计算测试失败: {}", e),
        }
        
        // 执行JavaScript文件
        match ctx.eval::<(), _>(&*js_code) {
            Ok(_) => println!("JavaScript文件执行成功"),
            Err(e) => {
                eprintln!("JavaScript文件执行失败: {}", e);
                return;
            }
        }
        
        // 测试游戏配置
        match ctx.eval::<f64, _>("GAME_CONFIG.playerSpeed") {
            Ok(speed) => println!("游戏速度配置: {}", speed),
            Err(e) => eprintln!("游戏配置测试失败: {}", e),
        }
        
        // 测试游戏状态类
        match ctx.eval::<(), _>("const testState = new GameState();") {
            Ok(_) => println!("游戏状态类创建成功"),
            Err(e) => eprintln!("游戏状态类创建失败: {}", e),
        }
        
        // 测试函数调用
        match ctx.eval::<(), _>("testState.updatePlayer(0.016);") {
            Ok(_) => println!("玩家更新函数调用成功"),
            Err(e) => eprintln!("玩家更新函数调用失败: {}", e),
        }
        
        // 测试位置获取
        match ctx.eval::<f64, _>("testState.getPlayerPosition().x") {
            Ok(x) => println!("X坐标获取成功: {}", x),
            Err(e) => eprintln!("X坐标获取失败: {}", e),
        }
        
        println!("JavaScript测试完成！");
    });
}