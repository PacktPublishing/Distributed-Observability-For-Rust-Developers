# High-Throughput Checkout Profiling Script (PowerShell)
#
# Purpose:
# This script is specifically designed to blast the POST /api/orders endpoint.
# Instead of simulating realistic traffic, it fires rapid concurrent requests
# to create maximum CPU and Memory pressure on the `orders` service. This
# guarantees a highly visible bottleneck when generating flamegraphs or
# heap profiles.
#
# Usage:
#   .\profile_checkout.ps1                  # Blast for 30 seconds
#   $env:DURATION=60; .\profile_checkout.ps1  # Blast for 60 seconds

$ErrorActionPreference = "Stop"

# Configuration
$GatewayUrl = if ($env:GATEWAY_URL) { $env:GATEWAY_URL } else { "http://localhost:4200" }
$Duration   = if ($env:DURATION) { [int]$env:DURATION } else { 30 }
$Concurrency = 5

Write-Host "========================================="
Write-Host "Checkout Profiling Load Generator"
Write-Host "========================================="
Write-Host "Targeting:   $GatewayUrl/api/orders"
Write-Host "Duration:    ${Duration}s"
Write-Host "Concurrency: $Concurrency workers"
Write-Host ""

# Health check
try {
    $null = Invoke-RestMethod -Uri "$GatewayUrl/health" -TimeoutSec 5
} catch {
    Write-Host "ERROR: Gateway service not reachable at $GatewayUrl" -ForegroundColor Red
    exit 1
}

# Fetch a valid product for the checkout payload
Write-Host "Fetching a valid product for checkout..."
$productsResponse = Invoke-RestMethod -Uri "$GatewayUrl/api/products?page_size=1"
$product = $productsResponse.products[0]

if (-not $product.eid) {
    Write-Host "ERROR: Failed to fetch a product for checkout." -ForegroundColor Red
    exit 1
}

$productUuid  = $product.eid
$productName  = $product.product_name
$productPrice = $product.final_price

Write-Host "Using product: $productName ($productUuid)"
Write-Host ""

# Worker scriptblock that each background job will execute
$workerScript = {
    param($WorkerId, $Url, $Payload, $EndTime)

    # Suppress progress bars for speed
    $ProgressPreference = 'SilentlyContinue'
    $reqCount = 0

    while ((Get-Date) -lt $EndTime) {
        try {
            $null = Invoke-WebRequest -Uri $Url `
                -Method POST `
                -ContentType "application/json" `
                -Body $Payload `
                -UseBasicParsing `
                -ErrorAction SilentlyContinue
        } catch {
            # We don't care if it succeeds (201) or fails (409 Insufficient Stock).
            # Both go through the validation hot path which is what we want to profile.
        }
        $reqCount++
    }

    return "Worker $WorkerId finished. Fired ~$reqCount requests."
}

# Calculate the end time once so all workers share the same deadline
$endTime = (Get-Date).AddSeconds($Duration)

Write-Host "Starting $Concurrency background workers..."
Write-Host ""

# Launch concurrent background jobs
$jobs = 1..$Concurrency | ForEach-Object {
    $workerId = $_

    # Build a unique payload per worker (different email)
    $payload = @"
{
    "customer_email": "profiler${workerId}@example.com",
    "items": [{
        "product_uuid": "$productUuid",
        "product_name": "$productName",
        "quantity": 1,
        "unit_price": "$productPrice"
    }],
    "shipping_address": {
        "first_name": "Perf",
        "last_name": "Test",
        "address_line1": "1 Profiling Way",
        "city": "Flamegraph City",
        "state": "CA",
        "postal_code": "90000",
        "country": "US"
    },
    "payment": {
        "payment_method": "credit_card",
        "card_last4": "1234",
        "card_brand": "visa"
    }
}
"@

    Start-Job -ScriptBlock $workerScript -ArgumentList $workerId, "$GatewayUrl/api/orders", $payload, $endTime
}

# Wait for all workers to finish and print their results
$jobs | Wait-Job | Receive-Job | ForEach-Object { Write-Host $_ }
$jobs | Remove-Job

Write-Host ""
Write-Host "========================================="
Write-Host "Profiling run complete!"
Write-Host "Check your flamegraph.svg or dhat-heap.json"
Write-Host "========================================="
