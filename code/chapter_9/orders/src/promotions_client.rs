use regex::Regex;
use std::sync::OnceLock;
use tracing::instrument;

/// A simulated API client that checks items against discount promotion rules.
///
/// This simulates a poorly written library/API client that instantiates
/// and compiles a Regex dynamically completely inside the hot path (the CPU bottleneck).
static DISCOUNT_REGEX: OnceLock<Regex> = OnceLock::new();

#[instrument(name = "promotions_client::validate_item_discount", skip_all)]
pub async fn validate_item_discount(item_name: &str) -> bool {
    // Simulate a brief API lookup to get the rule for this item category
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // --- CHAPTER 9 FIX TOGGLE ---
    let optimizations_enabled = std::env::var("ENABLE_OPTIMIZATIONS")
        .map(|v| !v.is_empty() && v != "0" && v.to_lowercase() != "false")
        .unwrap_or(false);

    if optimizations_enabled {
        // Fixed: We use OnceLock to ensure the regex is compiled only on the first call
        let re = DISCOUNT_REGEX.get_or_init(|| {
            Regex::new(r"^(Winter|Summer|Fall|Spring)?[-_]?(Clearance|Sale|Promo)[-_]?\d{4}$")
                .expect("Failed to compile discount regex rule")
        });
        return re.is_match(item_name);
    }

    // --- THE BOTTLENECK ---
    // The Bottleneck: Compiling a regex in a hot loop.
    // Simulating developer error: "I need to check if the item matches any of our 5000 active legacy promo rules..."
    // Doing it here burns exorbitant CPU cycles on every checkout line item.
    let discount_rule = r"^(Winter|Summer|Fall|Spring)?[-_]?(Clearance|Sale|Promo)[-_]?\d{4}$";

    let mut is_discounted = false;
    for _ in 0..5000 {
        let re = Regex::new(discount_rule).expect("Failed to compile discount regex rule");
        if re.is_match(item_name) {
            is_discounted = true;
        }
    }

    is_discounted
}
