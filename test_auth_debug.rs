// 临时测试文件：调试认证问题
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::OsRng;

fn main() {
    let password = "testpass123";
    
    // 模拟注册过程：生成密码哈希
    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();
    
    println!("原始密码: {}", password);
    println!("生成的哈希: {}", password_hash);
    
    // 模拟登录过程：验证密码
    let parsed_hash = PasswordHash::new(&password_hash).unwrap();
    let verification_result = argon2.verify_password(password.as_bytes(), &parsed_hash);
    
    match verification_result {
        Ok(()) => println!("✅ 密码验证成功"),
        Err(e) => println!("❌ 密码验证失败: {:?}", e),
    }
    
    // 测试错误密码
    let wrong_password = "wrongpass";
    let wrong_verification = argon2.verify_password(wrong_password.as_bytes(), &parsed_hash);
    
    match wrong_verification {
        Ok(()) => println!("❌ 错误密码验证成功（这不应该发生）"),
        Err(_) => println!("✅ 错误密码验证失败（正确行为）"),
    }
}
