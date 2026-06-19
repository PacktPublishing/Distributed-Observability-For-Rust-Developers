# Quick PowerShell traffic driver for chapter 8 e2e validation.
# Drives N checkout attempts through the OtelMart gateway so that the
# order.category log attribute, checkout_attempts/failures counters,
# and inventory.stock.level gauge all see live data.

param(
    [string]$Gateway = 'http://localhost:4200',
    [int]$Count = 30
)

$ErrorActionPreference = 'Stop'

Write-Host "Fetching product catalog..." -ForegroundColor Cyan
$catalog = Invoke-RestMethod "$Gateway/api/products?page_size=80"
$products = $catalog.products
Write-Host ("  loaded {0} products" -f $products.Count)

$success = 0
$fail = 0
for ($i = 1; $i -le $Count; $i++) {
    $itemCount = Get-Random -Minimum 1 -Maximum 4
    $picked = $products | Get-Random -Count $itemCount

    $items = @()
    foreach ($p in $picked) {
        $items += @{
            product_uuid = $p.eid
            product_name = $p.product_name
            product_sku  = $null
            quantity     = (Get-Random -Minimum 1 -Maximum 3)
            unit_price   = $p.final_price
        }
    }

    $payload = @{
        customer_email   = "loadtest+$i@example.com"
        customer_phone   = '555-0100'
        items            = $items
        shipping_address = @{
            first_name    = 'Load'
            last_name     = 'Test'
            address_line1 = '1 Test St'
            city          = 'Seattle'
            state         = 'WA'
            postal_code   = '98101'
            country       = 'US'
        }
        payment          = @{
            payment_method = 'credit_card'
            card_last4     = '4242'
            card_brand     = 'visa'
        }
    } | ConvertTo-Json -Depth 5

    try {
        $r = Invoke-RestMethod -Method Post -Uri "$Gateway/api/orders" `
            -ContentType 'application/json' -Body $payload -TimeoutSec 15
        $success++
        if ($i % 10 -eq 0) { Write-Host ("  {0}/{1} ok (last order_number={2})" -f $i, $Count, $r.order_number) -ForegroundColor Green }
    }
    catch {
        $fail++
        Write-Host ("  {0}/{1} FAIL: {2}" -f $i, $Count, $_.Exception.Message) -ForegroundColor Yellow
    }
    Start-Sleep -Milliseconds 150
}

Write-Host ""
Write-Host ("Done. success={0}  fail={1}" -f $success, $fail) -ForegroundColor Cyan
