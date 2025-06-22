# Test Error Handling

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

Write-Host "Testing invalid UUID format..." -ForegroundColor Green

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/tasks/invalid-uuid" -Method Get -Headers $headers
    Write-Host "Unexpected success:" -ForegroundColor Red
    $response | ConvertTo-Json -Depth 3
} catch {
    Write-Host "Expected error for invalid UUID:" -ForegroundColor Green
    Write-Host "Status: $($_.Exception.Response.StatusCode)"
    Write-Host "Message: $($_.Exception.Message)"
}

Write-Host "Testing non-existent task ID..." -ForegroundColor Green

$fakeUuid = "12345678-1234-1234-1234-123456789abc"

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/tasks/$fakeUuid" -Method Get -Headers $headers
    Write-Host "Unexpected success:" -ForegroundColor Red
    $response | ConvertTo-Json -Depth 3
} catch {
    Write-Host "Expected error for non-existent task:" -ForegroundColor Green
    Write-Host "Status: $($_.Exception.Response.StatusCode)"
    Write-Host "Message: $($_.Exception.Message)"
}

Write-Host "Testing unauthorized access..." -ForegroundColor Green

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/tasks" -Method Get
    Write-Host "Unexpected success:" -ForegroundColor Red
    $response | ConvertTo-Json -Depth 3
} catch {
    Write-Host "Expected error for unauthorized access:" -ForegroundColor Green
    Write-Host "Status: $($_.Exception.Response.StatusCode)"
    Write-Host "Message: $($_.Exception.Message)"
}

Write-Host "Error handling test completed" -ForegroundColor Cyan
