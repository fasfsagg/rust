# WebSocket 安全 E2E 测试运行脚本
# 
# 此脚本用于设置和运行 Playwright E2E 测试，验证 WebSocket 安全功能
# 
# 使用方法：
# .\run-e2e-tests.ps1

Write-Host "=== Axum WebSocket 安全 E2E 测试 ===" -ForegroundColor Green

# 检查 Node.js 是否安装
Write-Host "检查 Node.js 环境..." -ForegroundColor Yellow
try {
    $nodeVersion = node --version
    Write-Host "Node.js 版本: $nodeVersion" -ForegroundColor Green
} catch {
    Write-Host "错误: 未找到 Node.js，请先安装 Node.js" -ForegroundColor Red
    Write-Host "下载地址: https://nodejs.org/" -ForegroundColor Yellow
    exit 1
}

# 检查 npm 是否可用
try {
    $npmVersion = npm --version
    Write-Host "npm 版本: $npmVersion" -ForegroundColor Green
} catch {
    Write-Host "错误: npm 不可用" -ForegroundColor Red
    exit 1
}

# 安装依赖
Write-Host "安装 Playwright 依赖..." -ForegroundColor Yellow
try {
    npm install
    if ($LASTEXITCODE -ne 0) {
        throw "npm install 失败"
    }
    Write-Host "依赖安装成功" -ForegroundColor Green
} catch {
    Write-Host "错误: 依赖安装失败 - $_" -ForegroundColor Red
    exit 1
}

# 安装 Playwright 浏览器
Write-Host "安装 Playwright 浏览器..." -ForegroundColor Yellow
try {
    npx playwright install
    if ($LASTEXITCODE -ne 0) {
        throw "Playwright 浏览器安装失败"
    }
    Write-Host "Playwright 浏览器安装成功" -ForegroundColor Green
} catch {
    Write-Host "错误: Playwright 浏览器安装失败 - $_" -ForegroundColor Red
    exit 1
}

# 检查 Rust 项目是否可以编译
Write-Host "检查 Rust 项目编译..." -ForegroundColor Yellow
try {
    cargo check
    if ($LASTEXITCODE -ne 0) {
        throw "Rust 项目编译检查失败"
    }
    Write-Host "Rust 项目编译检查通过" -ForegroundColor Green
} catch {
    Write-Host "错误: Rust 项目编译失败 - $_" -ForegroundColor Red
    Write-Host "请先修复编译错误再运行 E2E 测试" -ForegroundColor Yellow
    exit 1
}

# 运行 E2E 测试
Write-Host "运行 WebSocket 安全 E2E 测试..." -ForegroundColor Yellow
Write-Host "注意: 测试将自动启动 Axum 服务器" -ForegroundColor Cyan

try {
    # 运行测试（Playwright 会自动启动服务器）
    npx playwright test
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "=== E2E 测试全部通过! ===" -ForegroundColor Green
        Write-Host "WebSocket 安全功能验证成功" -ForegroundColor Green
    } else {
        Write-Host "=== E2E 测试失败 ===" -ForegroundColor Red
        Write-Host "请检查测试报告了解详细信息" -ForegroundColor Yellow
        
        # 询问是否查看测试报告
        $showReport = Read-Host "是否打开测试报告? (y/n)"
        if ($showReport -eq "y" -or $showReport -eq "Y") {
            npx playwright show-report
        }
        exit 1
    }
} catch {
    Write-Host "错误: E2E 测试执行失败 - $_" -ForegroundColor Red
    exit 1
}

Write-Host "=== 测试完成 ===" -ForegroundColor Green
Write-Host "如需查看详细报告，运行: npx playwright show-report" -ForegroundColor Cyan
