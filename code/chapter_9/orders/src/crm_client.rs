use tracing::instrument;

/// Simulated legacy CRM API Response
#[derive(Debug)]
#[allow(dead_code)]
pub struct LoyaltyHistoryResponse {
    pub points_balance: i32,
    pub lifetime_events: Vec<String>,
}

/// A simulated API client for a legacy CRM system.
///
/// This client purposefully simulates an unpaginated internal API call
/// that deserializes and allocates a massive amount of memory (the memory bottleneck).
#[instrument(name = "crm_client::fetch_user_loyalty_history", skip(_email))]
pub async fn fetch_user_loyalty_history(_email: &str) -> LoyaltyHistoryResponse {
    // Simulate API network latency
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // --- CHAPTER 9 FIX TOGGLE ---
    // If optimizations are enabled, we use the fast path
    if let Ok(val) = std::env::var("ENABLE_OPTIMIZATIONS") {
        if val == "1" || val == "true" {
            // Fixed: Legacy API upgraded to return just the balance if history isn't needed,
            // or we only fetch the most recent a few events instead of 15,000.
            let lifetime_events = vec![
                "Loyalty event: User earned 5 points for purchase".to_string(),
                "Loyalty event: User earned 10 points for review".to_string(),
            ];

            return LoyaltyHistoryResponse {
                points_balance: 1250,
                lifetime_events,
            };
        }
    }

    // --- THE BOTTLENECK ---
    // Simulate an unpaginated API response containing thousands of events
    // This creates immense memory pressure during high concurrency checkouts.
    // We allocate 15,000 strings per checkout dynamically to give dhat-heap a nice target.
    let mut lifetime_events = Vec::with_capacity(15_000);
    for i in 0..15_000 {
        // We use format! to ensure a fresh heap allocation for each string
        lifetime_events.push(format!(
            "Loyalty event {}: User earned 5 points for purchase",
            i
        ));
    }

    LoyaltyHistoryResponse {
        points_balance: 1250,
        lifetime_events,
    }
}
