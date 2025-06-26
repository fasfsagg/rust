# 测试性能监控中间件
# 验证性能监控中间件是否正确集成到Axum应用中

$baseUrl = "http://127.0.0.1:3000"

Write-Host "=== 性能监控中间件测试 ===" -ForegroundColor Green

# 等待服务器启动
Write-Host "等待服务器启动..." -ForegroundColor Yellow
Start-Sleep -Seconds 3

# 测试1: 健康检查端点
Write-Host "`n1. 测试健康检查端点" -ForegroundColor Cyan
try {
    $healthResponse = Invoke-RestMethod -Uri "$baseUrl/api/performance/health" -Method Get
    Write-Host "✓ 健康检查成功" -ForegroundColor Green
    Write-Host "状态: $($healthResponse.status)" -ForegroundColor White
    Write-Host "活跃连接数: $($healthResponse.details.performance.active_connections)" -ForegroundColor White
} catch {
    Write-Host "✗ 健康检查失败: $($_.Exception.Message)" -ForegroundColor Red
}

# 测试2: 性能统计端点
Write-Host "`n2. 测试性能统计端点" -ForegroundColor Cyan
try {
    $statsResponse = Invoke-RestMethod -Uri "$baseUrl/api/performance/stats" -Method Get
    Write-Host "✓ 性能统计获取成功" -ForegroundColor Green
    Write-Host "总请求数: $($statsResponse.total_requests)" -ForegroundColor White
    Write-Host "成功请求数: $($statsResponse.successful_requests)" -ForegroundColor White
    Write-Host "成功率: $($statsResponse.success_rate)%" -ForegroundColor White
} catch {
    Write-Host "✗ 性能统计获取失败: $($_.Exception.Message)" -ForegroundColor Red
}

# 测试3: 详细指标端点
Write-Host "`n3. 测试详细指标端点" -ForegroundColor Cyan
try {
    $metricsResponse = Invoke-RestMethod -Uri "$baseUrl/api/performance/metrics" -Method Get
    Write-Host "✓ 详细指标获取成功" -ForegroundColor Green
    Write-Host "系统CPU使用率: $($metricsResponse.system_info.cpu_usage_percent)%" -ForegroundColor White
    Write-Host "系统内存使用率: $($metricsResponse.system_info.memory_usage_percent)%" -ForegroundColor White
} catch {
    Write-Host "✗ 详细指标获取失败: $($_.Exception.Message)" -ForegroundColor Red
}

# 测试4: 多个请求以验证计数器
Write-Host "`n4. 发送多个请求验证计数器" -ForegroundColor Cyan
$initialStats = Invoke-RestMethod -Uri "$baseUrl/api/performance/stats" -Method Get
$initialCount = $initialStats.total_requests

Write-Host "初始请求数: $initialCount" -ForegroundColor White

# 发送5个额外请求
for ($i = 1; $i -le 5; $i++) {
    try {
        Invoke-RestMethod -Uri "$baseUrl/api/performance/health" -Method Get | Out-Null
        Write-Host "  请求 $i 完成" -ForegroundColor Gray
    } catch {
        Write-Host "  请求 $i 失败" -ForegroundColor Red
    }
}

# 检查最终统计
$finalStats = Invoke-RestMethod -Uri "$baseUrl/api/performance/stats" -Method Get
$finalCount = $finalStats.total_requests

Write-Host "最终请求数: $finalCount" -ForegroundColor White
Write-Host "新增请求数: $($finalCount - $initialCount)" -ForegroundColor White

if ($finalCount -gt $initialCount) {
    Write-Host "✓ 请求计数器正常工作" -ForegroundColor Green
} else {
    Write-Host "✗ 请求计数器可能有问题" -ForegroundColor Red
}

Write-Host "`n=== 测试完成 ===" -ForegroundColor Green
