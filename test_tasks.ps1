# Test Tasks API

$baseUrl = "http://127.0.0.1:3000"

# First login to get token
$loginBody = @{
    username = "testuser"
    password = "password123"
} | ConvertTo-Json

$loginResponse = Invoke-RestMethod -Uri "$baseUrl/api/auth/login" -Method Post -Body $loginBody -ContentType "application/json"
$token = $loginResponse.access_token

$headers = @{
    "Authorization" = "Bearer $token"
    "Content-Type" = "application/json"
}

Write-Host "Testing get tasks..." -ForegroundColor Green

try {
    $tasksResponse = Invoke-RestMethod -Uri "$baseUrl/api/tasks" -Method Get -Headers $headers
    Write-Host "Get tasks successful:" -ForegroundColor Green
    $tasksResponse | ConvertTo-Json -Depth 3
} catch {
    Write-Host "Get tasks failed:" -ForegroundColor Red
    Write-Host $_.Exception.Message
}

Write-Host "Testing create task..." -ForegroundColor Green

$createTaskBody = @{
    title = "Test Task"
    description = "This is a test task"
    completed = $false
} | ConvertTo-Json

try {
    $createTaskResponse = Invoke-RestMethod -Uri "$baseUrl/api/tasks" -Method Post -Body $createTaskBody -Headers $headers
    Write-Host "Create task successful:" -ForegroundColor Green
    $createTaskResponse | ConvertTo-Json -Depth 3
    
    $taskId = $createTaskResponse.id
    Write-Host "Created task ID: $taskId" -ForegroundColor Yellow
    
    # Test get single task
    Write-Host "Testing get single task..." -ForegroundColor Green
    
    try {
        $getTaskResponse = Invoke-RestMethod -Uri "$baseUrl/api/tasks/$taskId" -Method Get -Headers $headers
        Write-Host "Get single task successful:" -ForegroundColor Green
        $getTaskResponse | ConvertTo-Json -Depth 3
    } catch {
        Write-Host "Get single task failed:" -ForegroundColor Red
        Write-Host $_.Exception.Message
    }
    
    # Test update task
    Write-Host "Testing update task..." -ForegroundColor Green
    
    $updateTaskBody = @{
        title = "Updated Test Task"
        description = "This task has been updated"
        completed = $true
    } | ConvertTo-Json
    
    try {
        $updateTaskResponse = Invoke-RestMethod -Uri "$baseUrl/api/tasks/$taskId" -Method Put -Body $updateTaskBody -Headers $headers
        Write-Host "Update task successful:" -ForegroundColor Green
        $updateTaskResponse | ConvertTo-Json -Depth 3
    } catch {
        Write-Host "Update task failed:" -ForegroundColor Red
        Write-Host $_.Exception.Message
    }
    
} catch {
    Write-Host "Create task failed:" -ForegroundColor Red
    Write-Host $_.Exception.Message
}

Write-Host "Tasks API test completed" -ForegroundColor Cyan
