# Fraud Scoring Service

A stateless microservice that scores orders for fraud risk by calling a GenAI model. Instrumented with OpenTelemetry GenAI semantic conventions for spans, metrics, and logs.

## Quick Start (Mock Provider)

No API key needed. The mock provider is enabled by default:

```bash
# With Docker (from code/chapter_14/)
docker compose up -d

# Or locally
cd fraud_scoring
cargo run
```

Test:

```bash
curl -s -X POST http://localhost:3004/score \
  -H "Content-Type: application/json" \
  -d '{
    "order_total": 150.00,
    "is_returning_customer": true,
    "product_names": ["Wireless Mouse"],
    "gift_message": "Happy birthday!",
    "has_international_shipping": false,
    "flagged_product_category": false
  }'
```

## Using the Real Anthropic API

### 1. Get an API Key

1. Go to [console.anthropic.com](https://console.anthropic.com)
2. Sign up or log in
3. Navigate to **Settings → API Keys**
4. Click **Create Key** and copy it (starts with `sk-ant-api03-...`)
5. Add credits at **Settings → Billing** ($5 minimum)

### 2. Configure the Service

**Local (cargo run):**

```powershell
$env:ANTHROPIC_API_KEY = "sk-ant-your-key-here"
$env:FRAUD_SCORING_PROVIDER_NAME = "anthropic"
cargo run
```

**Docker:**

```powershell
$env:ANTHROPIC_API_KEY = "sk-ant-your-key-here"
$env:FRAUD_SCORING_PROVIDER_NAME = "anthropic"
docker compose up -d fraud_scoring
```

> **Never commit API keys to the repository.** Set them as environment variables or use a `.env` file (already in `.gitignore`).

### 3. Verify

```bash
curl -s -X POST http://localhost:3004/score \
  -H "Content-Type: application/json" \
  -d '{"order_total":150.00,"is_returning_customer":true,"product_names":["Wireless Mouse"],"has_international_shipping":false,"flagged_product_category":false}'
```

With the real API, you'll see:
- `risk_score`: a real fraud assessment (0.0–1.0)
- Response time: 200ms–2s (vs instant with mock)
- Real token counts in Jaeger traces

## API

### `POST /score`

Score an order for fraud risk.

**Request:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `order_total` | `f64` | Yes | Dollar amount |
| `is_returning_customer` | `bool` | Yes | Has ordered before |
| `product_names` | `string[]` | Yes | Products in order |
| `gift_message` | `string?` | No | Gift message text |
| `has_international_shipping` | `bool` | No | Ships internationally (default: false) |
| `flagged_product_category` | `bool` | No | Flagged category (default: false) |

**Response:**

```json
{
  "risk_score": 0.05,
  "decision": "approved"
}
```

Decisions: `approved` (score < 0.3), `manual_review` (0.3–0.7), `rejected` (> 0.7).

### `GET /health`

Returns `{"status": "healthy"}`.

## Configuration

Configuration is loaded from `config.toml` with environment variable overrides prefixed with `FRAUD_SCORING_`:

| Setting | Config Key | Env Override | Default |
|---------|-----------|--------------|---------|
| Port | `server.port` | `FRAUD_SCORING_SERVER_PORT` | 3004 |
| Provider | `provider.name` | `FRAUD_SCORING_PROVIDER_NAME` | mock |
| High-risk model | `provider.high_risk_model` | `FRAUD_SCORING_PROVIDER_HIGH_RISK_MODEL` | claude-opus-4-6 |
| Low-risk model | `provider.low_risk_model` | `FRAUD_SCORING_PROVIDER_LOW_RISK_MODEL` | claude-haiku-4-5 |
| Max tokens | `provider.max_tokens` | `FRAUD_SCORING_PROVIDER_MAX_TOKENS` | 512 |
| Temperature | `provider.temperature` | `FRAUD_SCORING_PROVIDER_TEMPERATURE` | 0.1 |
| GenAI timeout | `provider.timeout_secs` | `FRAUD_SCORING_PROVIDER_TIMEOUT_SECS` | 5 |
| API key | — | `ANTHROPIC_API_KEY` | — |

## Two-Tier Model Routing

Orders are automatically routed to different models based on risk profile:

- **Haiku** (fast, cheap): returning customers, orders under $200, domestic shipping
- **Opus** (capable, expensive): new customers, high-value orders, international shipping, flagged categories

## Telemetry

The service emits OpenTelemetry GenAI semantic convention attributes on spans and metrics:

- **Spans**: `gen_ai.operation.name`, `gen_ai.provider.name`, `gen_ai.request.model`, `gen_ai.usage.input_tokens`, `gen_ai.usage.output_tokens`, `gen_ai.response.finish_reasons`, `otelmart.fraud.risk_score`, `otelmart.fraud.decision`
- **Metrics**: `gen_ai.client.operation.duration`, `gen_ai.client.token.usage`, `otelmart.fraud.decision`, `otelmart.fraud.cost_per_order`

View in:
- **Jaeger**: http://localhost:16686 → service `fraud_scoring`
- **Grafana**: http://localhost:3000 → Prometheus datasource
- **Loki**: `{service_name="fraud_scoring"}`
