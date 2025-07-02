# 测试修复验证脚本
# 验证依赖项修复是否成功

Write-Host "=== 依赖项修复验证脚本 ===" -ForegroundColor Green

# 1. 检查基本编译
Write-Host "1. 检查基本编译（不包含测试特性）..." -ForegroundColor Yellow
$result1 = cargo check 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ 基本编译成功" -ForegroundColor Green
} else {
    Write-Host "❌ 基本编译失败" -ForegroundColor Red
    Write-Host $result1
}

# 2. 检查带测试特性的编译
Write-Host "2. 检查带测试特性的编译..." -ForegroundColor Yellow
$result2 = cargo check --features testing 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ 测试特性编译成功" -ForegroundColor Green
} else {
    Write-Host "❌ 测试特性编译失败" -ForegroundColor Red
    Write-Host $result2
}

# 3. 检查测试编译
Write-Host "3. 检查测试编译..." -ForegroundColor Yellow
$result3 = cargo test --no-run --features testing 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ 测试编译成功" -ForegroundColor Green
} else {
    Write-Host "❌ 测试编译失败" -ForegroundColor Red
    Write-Host $result3
}

Write-Host "=== 验证完成 ===" -ForegroundColor Green
