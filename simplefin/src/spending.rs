use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::storage::TransactionWithContext;

/// The fallback category name for unclassified transactions.
pub const OTHER_CATEGORY: &str = "other";

/// Convert a snake_case category name to a human-readable label.
/// e.g. "personal_care" → "Personal Care", "restaurants" → "Restaurants".
pub fn category_label(category: &str) -> String {
    category
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    let mut s = c.to_uppercase().to_string();
                    s.extend(chars);
                    s
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A rule for classifying transactions into spending categories.
/// Patterns support `|` separated keywords (e.g. "chipotle|starbucks|pizza").
/// All matching is case-insensitive substring matching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingRule {
    pub pattern: String,
    pub category: String,
}

/// Per-category spending total.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpendingTotal {
    pub category: String,
    pub label: String,
    pub total: Decimal,
    pub transaction_count: usize,
}

/// An unclassified transaction surfaced for user review.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnclassifiedTransaction {
    pub description: String,
    pub amount: Decimal,
}

/// Spending summary for a time period.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpendingSummary {
    pub categories: Vec<SpendingTotal>,
    pub total_spending: Decimal,
    pub total_income: Decimal,
    pub net: Decimal,
    /// Transactions that could not be classified (fell into Other).
    /// Includes description and amount so the user can identify them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unclassified: Vec<UnclassifiedTransaction>,
}

/// Returns the default spending patterns that are seeded into the data directory
/// on first use. These provide a reasonable starting point; users customize by
/// editing `spending_patterns.json` in the data directory or via the CLI.
///
/// Patterns support `|` separated keywords. Order matters — more specific
/// categories should come before broad ones (e.g. "pet" before "store").
pub fn default_spending_patterns() -> Vec<SpendingRule> {
    let raw: &[(&str, &str)] = &[
        // Transfer — ACH/wire/Zelle are unambiguous.
        // "payment" omitted (too broad). "wire " has trailing space to avoid "wireless".
        (
            "transfer|zelle|venmo|paypal|cashapp|cash app|wire |wire transfer\
            |crypto|coinbase|withdrawal|atm|check #|epayment|edeposit\
            |mobile payment|ach debit|ach credit|ach pmt\
            |payment - thank",
            "transfer",
        ),
        // Income — payroll, deposits, dividends, refunds
        (
            "payroll|direct dep|direct deposit|salary|wage|deposit from employer\
            |interest earned|interest charge|dividend|refund|reimburs|cashback\
            |cash back|reward redemp|tax refund",
            "income",
        ),
        // Pets — before shopping so pet stores match here, not "store"
        (
            "pet |pets |petco|petsmart|pet supply|veterinar|vet |animal hospital\
            |animal clinic|groomer|dog |cat ",
            "pets",
        ),
        // Housing — rent, HOA, property, maintenance
        (
            "rent |lease |hoa|property tax|plumb|roof|hvac|landscap\
            |lawn|pest control|handyman|maintenance fee|condo fee|apartment",
            "housing",
        ),
        // Insurance — all types
        (
            "insurance|insur |geico|state farm|allstate|progressive|usaa\
            |liberty mutual|farmers|nationwide",
            "insurance",
        ),
        // Education — tuition, courses, schools
        (
            "college|university|tuition|coursera|udemy|school|education\
            |academic|seminary|learning",
            "education",
        ),
        // Personal Care — barber, beauty, salon, spa, wellness
        (
            "barber|beauty|salon|spa |hair |nail |wax |massage|wellness\
            |wellbeing|cosmetic|skincare",
            "personal_care",
        ),
        // Healthcare — before shopping so CVS/Walgreens match here
        (
            "pharmacy|doctor|hospital|dental|dentist|medical|health plan\
            |clinic|urgent care|cvs|walgreen|rite aid|vision|optical|eye care\
            |therapist|counseling|chiropractic|physical therapy|laboratory\
            |diagnostic|prescription|kaiser|blue cross|aetna|cigna\
            |united health|family ph",
            "healthcare",
        ),
        // Utilities — gas/electric/water/internet/phone/waste
        (
            "electr|water bill|sewer|internet|cable|phone bill|verizon|comcast\
            |xfinity|t-mobile|spectrum|cox|frontier|broadband|fiber|wireless bill\
            |waste|garbage|trash|recycling|pgande|utility|disposal|natural gas\
            |nw natural|general elect|boost mobile|google fi|mint mobile|ziply",
            "utilities",
        ),
        // Groceries — before restaurants so grocery stores match here
        (
            "grocery|whole foods|trader joe|safeway|kroger|costco|fred meyer\
            |winco|albertson|supermarket|produce|butcher|food co-op\
            |aldi|lidl|sprouts|publix|wegmans|heb |meijer|piggly|food mart",
            "groceries",
        ),
        // Restaurants — broad food/dining/drink patterns
        (
            "restaurant|dine|dining|cafe|coffee|pizza|burger|sushi|taco\
            |chipotle|mcdonald|starbucks|grubhub|doordash|ubereats|grill\
            |kitchen|bakery|deli|sandwich|noodle|thai|pho|ramen|brew|taproom\
            |bar & |pub |bbq|wing|food truck|bistro|cantina|food |foods\
            |donut|doughnut|pancake|waffle|buffet|ale |tavern|wok|steak\
            |seafood|curry|burrito|teriyaki|kebab|gelato|ice cream|smoothie\
            |juice bar|brunch|catering|gastropub|gastr|coff|chick-fil\
            |popeye|domino|red robin|panera|olive garden|applebee|ihop\
            |denny|wendy|five guys|panda express|wingstop|crumbl|hot cake\
            |cake |dutch bros|benihana|mcmenamins",
            "restaurants",
        ),
        // Transportation — fuel, rideshare, transit, travel, lodging
        (
            "uber|lyft|parking|fuel|gas station|shell|chevron|transi|trimet\
            |taxi|cab |train|airline|flight|amtrak|metro|bus |toll|ev charg\
            |bp |arco|exxon|wawa|pilot|southwest|delta air|united air|jetblue\
            |alaska air|hotel|motel|airbnb|lodge|resort|hop fares|hop fast",
            "transportation",
        ),
        // Subscriptions — recurring digital/physical services
        (
            "subscription|recurr|member|annual fee|monthly fee|patreon|substack\
            |icloud|dropbox|1password|lastpass|adobe|microsoft 365|office 365\
            |google storage|chatgpt|openai|github|notion|apple.com/bill\
            |google *|prime video|new york times|nytimes|simplicity.com",
            "subscriptions",
        ),
        // Entertainment — media, fitness, recreation, gaming, venues
        (
            "netflix|spotify|hulu|theatr|movie|concert|disney|youtube|gaming\
            |steam|gym|fitness|sport|ticket|event|museum|zoo|golf|bowling\
            |arcade|apple music|audible|kindle|twitch|peacock|paramount|hbo\
            |cinema|regal |amc |casino|expo|recreation|park an|wizards\
            |moviepass|vending",
            "entertainment",
        ),
        // Shopping — broadest catch-all for retail (checked last before Other)
        (
            "amazon|target|walmart|best buy|ebay|etsy|retail|clothing|apparel\
            |furniture|home depot|lowes|ikea|hardware|office depot|staples\
            |michaels|hobby|craft|book|store|shop|purchase|jewel|marshall\
            |tj maxx|ross |nordstrom|gap |old navy|joann",
            "shopping",
        ),
    ];

    raw.iter()
        .map(|(pattern, category)| SpendingRule {
            pattern: pattern.to_string(),
            category: category.to_string(),
        })
        .collect()
}

/// Classify a transaction description into a spending category.
///
/// Rules are checked in order; first match wins. Patterns support `|` separated
/// keywords (e.g. "chipotle|starbucks|pizza"). All matching is case-insensitive.
///
/// The caller is responsible for assembling rules in priority order. Typically:
/// user-specific rules first, then default patterns from `spending_patterns.json`.
pub fn classify_transaction(description: &str, rules: &[SpendingRule]) -> String {
    let lower = description.to_lowercase();

    for rule in rules {
        for keyword in rule.pattern.split('|') {
            // Only trim leading whitespace — trailing spaces are intentional
            // (e.g. "wire " avoids matching "wireless")
            let keyword = keyword.trim_start().to_lowercase();
            if !keyword.is_empty() && lower.contains(&keyword) {
                return rule.category.clone();
            }
        }
    }

    OTHER_CATEGORY.to_string()
}

/// Compute spending summary from transactions.
/// Spending = negative amounts (debits). Income = positive amounts (credits).
///
/// Unclassified transaction descriptions (those falling into "Other") are collected
/// in the `unclassified` field so the AI skill can ask the user about them.
pub fn compute_spending(
    transactions: &[TransactionWithContext],
    rules: &[SpendingRule],
) -> SpendingSummary {
    let mut by_category: HashMap<String, (Decimal, usize)> = HashMap::new();
    let mut total_spending = Decimal::ZERO;
    let mut total_income = Decimal::ZERO;
    let mut unclassified: Vec<UnclassifiedTransaction> = Vec::new();

    for txn in transactions {
        if txn.pending {
            continue;
        }

        let category = classify_transaction(&txn.description, rules);

        if category == OTHER_CATEGORY
            && !unclassified.iter().any(|u| u.description == txn.description)
        {
            unclassified.push(UnclassifiedTransaction {
                description: txn.description.clone(),
                amount: txn.amount,
            });
        }

        let entry = by_category.entry(category).or_insert((Decimal::ZERO, 0));
        entry.0 += txn.amount;
        entry.1 += 1;

        if txn.amount < Decimal::ZERO {
            total_spending += txn.amount;
        } else {
            total_income += txn.amount;
        }
    }

    let mut categories: Vec<SpendingTotal> = by_category
        .into_iter()
        .map(|(cat, (total, count))| SpendingTotal {
            label: category_label(&cat),
            category: cat,
            total,
            transaction_count: count,
        })
        .collect();

    // Sort by absolute total descending
    categories.sort_by(|a, b| b.total.abs().cmp(&a.total.abs()));

    SpendingSummary {
        categories,
        total_spending,
        total_income,
        net: total_income + total_spending,
        unclassified,
    }
}
