# 消息搜索API测试脚本
$baseUrl = "http://127.0.0.1:3000/api"

# 测试用户数据
$loginData = @{
    username = "testuser"
    password = "password123"
} | ConvertTo-Json

Write-Host "🔐 测试用户登录..." -ForegroundColor Green

try {
    # 登录获取token
    $loginResponse = Invoke-RestMethod -Uri "$baseUrl/auth/login" -Method POST -ContentType "application/json" -Body $loginData
    $token = $loginResponse.access_token
    Write-Host "✅ 登录成功，获得token: $($token.Substring(0,20))..." -ForegroundColor Green
    
    # 测试空关键词搜索
    Write-Host "`n🔍 测试空关键词搜索..." -ForegroundColor Yellow
    try {
        $searchResponse = Invoke-RestMethod -Uri "$baseUrl/messages/search?keyword=""&page=1" -Method GET -Headers @{Authorization = "Bearer $token"}
        Write-Host "❌ 错误：空关键词搜索应该返回400错误，但返回了成功响应" -ForegroundColor Red
        Write-Host "响应内容: $($searchResponse | ConvertTo-Json)" -ForegroundColor Red
    } catch {
        $statusCode = $_.Exception.Response.StatusCode.value__
        Write-Host "✅ 空关键词搜索正确返回错误: $statusCode" -ForegroundColor Green
    }
    
    # 测试无效聊天室ID
    Write-Host "`n🏠 测试无效聊天室ID..." -ForegroundColor Yellow
    try {
        $chatRoomResponse = Invoke-RestMethod -Uri "$baseUrl/messages/chat-room/invalid-uuid" -Method GET -Headers @{Authorization = "Bearer $token"}
        Write-Host "❌ 错误：无效聊天室ID应该返回400错误，但返回了成功响应" -ForegroundColor Red
        Write-Host "响应内容: $($chatRoomResponse | ConvertTo-Json)" -ForegroundColor Red
    } catch {
        $statusCode = $_.Exception.Response.StatusCode.value__
        Write-Host "✅ 无效聊天室ID正确返回错误: $statusCode" -ForegroundColor Green
    }
    
    # 测试正常搜索
    Write-Host "`n🔍 测试正常搜索..." -ForegroundColor Yellow
    try {
        $normalSearchResponse = Invoke-RestMethod -Uri "$baseUrl/messages/search?keyword=test""&page=1" -Method GET -Headers @{Authorization = "Bearer $token"}
        Write-Host "✅ 正常搜索成功，返回 $($normalSearchResponse.pagination.total_count) 条消息" -ForegroundColor Green
        Write-Host "响应结构: $($normalSearchResponse | ConvertTo-Json -Depth 2)" -ForegroundColor Cyan
    } catch {
        $statusCode = $_.Exception.Response.StatusCode.value__
        Write-Host "❌ 正常搜索失败: $statusCode" -ForegroundColor Red
    }
    
} catch {
    Write-Host "❌ 登录失败: $($_.Exception.Message)" -ForegroundColor Red
}
