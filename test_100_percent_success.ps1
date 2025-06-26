# 测试是否能达到100%成功率
# 通过只发送预期成功的请求

$baseUrl = "http://127.0.0.1:3000"

Write-Host "=== 测试100%成功率 ===" -ForegroundColor Green

# 等待服务器启动
Start-Sleep -Seconds 3

# 获取初始统计
$initialStats = Invoke-RestMethod -Uri "$baseUrl/api/performance/stats" -Method Get
Write-Host "初始统计:" -ForegroundColor Cyan
Write-Host "  总请求: $($initialStats.total_requests)" -ForegroundColor White
Write-Host "  成功: $($initialStats.successful_requests)" -ForegroundColor White
Write-Host "  错误: $($initialStats.error_requests)" -ForegroundColor White
Write-Host "  成功率: $($initialStats.success_rate)%" -ForegroundColor White

Write-Host "`n发送10个健康检查请求..." -ForegroundColor Yellow

# 发送10个健康检查请求（预期都成功）
for ($i = 1; $i -le 10; $i++) {
    try {
        $response = Invoke-WebRequest -Uri "$baseUrl/api/performance/health" -Method GET
        if ($response.StatusCode -eq 200) {
            Write-Host "  请求 $i : ✅ 成功 (200)" -ForegroundColor Green
        } else {
            Write-Host "  请求 $i : ⚠️  状态码 $($response.StatusCode)" -ForegroundColor Yellow
        }
    } catch {
        Write-Host "  请求 $i : ❌ 失败 - $($_.Exception.Message)" -ForegroundColor Red
    }
}

# 获取最终统计
$finalStats = Invoke-RestMethod -Uri "$baseUrl/api/performance/stats" -Method Get
Write-Host "`n最终统计:" -ForegroundColor Cyan
Write-Host "  总请求: $($finalStats.total_requests)" -ForegroundColor White
Write-Host "  成功: $($finalStats.successful_requests)" -ForegroundColor White
Write-Host "  错误: $($finalStats.error_requests)" -ForegroundColor White
Write-Host "  成功率: $($finalStats.success_rate)%" -ForegroundColor White

# 计算本次测试的结果
$newTotal = $finalStats.total_requests - $initialStats.total_requests
$newSuccess = $finalStats.successful_requests - $initialStats.successful_requests
$newErrors = $finalStats.error_requests - $initialStats.error_requests

Write-Host "`n本次测试结果:" -ForegroundColor Cyan
Write-Host "  新增总请求: $newTotal" -ForegroundColor White
Write-Host "  新增成功: $newSuccess" -ForegroundColor White
Write-Host "  新增错误: $newErrors" -ForegroundColor White

if ($newTotal -eq $newSuccess -and $newErrors -eq 0) {
    Write-Host "  🎉 本次测试达到100%成功率！" -ForegroundColor Green
} else {
    Write-Host "  ⚠️  本次测试未达到100%成功率" -ForegroundColor Yellow
    Write-Host "     可能原因：请求开始但未完成，或中间件计数逻辑问题" -ForegroundColor Gray
}

# 检查总体成功率
if ($finalStats.success_rate -eq 100.0) {
    Write-Host "`n🎉 总体成功率已达到100%！" -ForegroundColor Green
} elseif ($finalStats.success_rate -gt 95.0) {
    Write-Host "`n✅ 总体成功率很高: $($finalStats.success_rate)%" -ForegroundColor Green
} else {
    Write-Host "`n📈 总体成功率: $($finalStats.success_rate)%" -ForegroundColor Yellow
}

Write-Host "`n=== 测试完成 ===" -ForegroundColor Green
