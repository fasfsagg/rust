@echo off
echo 🔧 验证断路器功能修复...
echo.

echo 📋 步骤1: 编译检查
cargo check
if %errorlevel% neq 0 (
    echo ❌ 编译失败
    exit /b 1
)
echo ✅ 编译通过

echo.
echo 📋 步骤2: Clippy检查
cargo clippy --quiet
if %errorlevel% neq 0 (
    echo ❌ Clippy检查失败
    exit /b 1
)
echo ✅ Clippy检查通过

echo.
echo 📋 步骤3: 运行特定断路器测试
cargo test test_circuit_breaker_open --lib -- --nocapture
if %errorlevel% neq 0 (
    echo ❌ 断路器测试失败
    exit /b 1
)
echo ✅ 断路器测试通过

echo.
echo 🎉 所有验证步骤完成！
