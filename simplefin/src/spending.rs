use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::storage::TransactionWithContext;

/// Spending category for transaction classification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SpendingCategory {
    Restaurants,
    Groceries,
    Utilities,
    Transportation,
    Shopping,
    Entertainment,
    Healthcare,
    Housing,
    Insurance,
    Subscriptions,
    Education,
    PersonalCare,
    Pets,
    Income,
    Transfer,
    Other,
}

impl std::fmt::Display for SpendingCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Restaurants => write!(f, "Restaurants"),
            Self::Groceries => write!(f, "Groceries"),
            Self::Utilities => write!(f, "Utilities"),
            Self::Transportation => write!(f, "Transportation"),
            Self::Shopping => write!(f, "Shopping"),
            Self::Entertainment => write!(f, "Entertainment"),
            Self::Healthcare => write!(f, "Healthcare"),
            Self::Housing => write!(f, "Housing"),
            Self::Insurance => write!(f, "Insurance"),
            Self::Subscriptions => write!(f, "Subscriptions"),
            Self::Education => write!(f, "Education"),
            Self::PersonalCare => write!(f, "Personal Care"),
            Self::Pets => write!(f, "Pets"),
            Self::Income => write!(f, "Income"),
            Self::Transfer => write!(f, "Transfer"),
            Self::Other => write!(f, "Other"),
        }
    }
}

/// A rule for classifying transactions into spending categories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingRule {
    pub pattern: String,
    pub category: SpendingCategory,
}

/// Per-category spending total.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpendingTotal {
    pub category: SpendingCategory,
    pub label: String,
    pub total: Decimal,
    pub transaction_count: usize,
}

/// Spending summary for a time period.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpendingSummary {
    pub categories: Vec<SpendingTotal>,
    pub total_spending: Decimal,
    pub total_income: Decimal,
    pub net: Decimal,
}

/// Built-in keyword patterns for transaction classification.
/// Each entry is (keywords separated by |, category).
/// Order matters — more specific categories are checked first to avoid
/// broad patterns (like "shop" or "store") from swallowing everything.
const BUILTIN_RULES: &[(&str, SpendingCategory)] = &[
    // Transfer — ACH/wire/Zelle are unambiguous.
    // "payment" omitted (too broad). "wire " has trailing space to avoid "wireless".
    (
        "transfer|zelle|venmo|paypal|cashapp|cash app|wire |wire transfer\
        |crypto|coinbase|withdrawal|atm|check #|epayment|edeposit\
        |mobile payment|ach debit|ach credit|ach pmt\
        |payment - thank",
        SpendingCategory::Transfer,
    ),
    // Income — payroll, deposits, dividends, refunds
    (
        "payroll|direct dep|direct deposit|salary|wage|deposit from employer\
        |interest earned|interest charge|dividend|refund|reimburs|cashback\
        |cash back|reward redemp|tax refund",
        SpendingCategory::Income,
    ),
    // Pets — before shopping so pet stores match here, not "store"
    (
        "pet |pets |petco|petsmart|pet supply|veterinar|vet |animal hospital\
        |animal clinic|groomer|dog |cat ",
        SpendingCategory::Pets,
    ),
    // Housing — rent, HOA, property, maintenance
    (
        "rent |lease |hoa|property tax|plumb|roof|hvac|landscap\
        |lawn|pest control|handyman|maintenance fee|condo fee|apartment",
        SpendingCategory::Housing,
    ),
    // Insurance — all types
    (
        "insurance|insur |geico|state farm|allstate|progressive|usaa\
        |liberty mutual|farmers|nationwide",
        SpendingCategory::Insurance,
    ),
    // Education — tuition, courses, schools
    (
        "college|university|tuition|coursera|udemy|school|education\
        |academic|seminary|learning",
        SpendingCategory::Education,
    ),
    // Personal Care — barber, beauty, salon, spa, wellness
    (
        "barber|beauty|salon|spa |hair |nail |wax |massage|wellness\
        |wellbeing|cosmetic|skincare",
        SpendingCategory::PersonalCare,
    ),
    // Healthcare — before shopping so CVS/Walgreens match here
    (
        "pharmacy|doctor|hospital|dental|dentist|medical|health plan\
        |clinic|urgent care|cvs|walgreen|rite aid|vision|optical|eye care\
        |therapist|counseling|chiropractic|physical therapy|laboratory\
        |diagnostic|prescription|kaiser|blue cross|aetna|cigna\
        |united health|family ph",
        SpendingCategory::Healthcare,
    ),
    // Utilities — gas/electric/water/internet/phone/waste
    // Short prefixes handle truncated descriptions (e.g. "ELECT" for "ELECTRIC")
    (
        "electr|water bill|sewer|internet|cable|phone bill|verizon|comcast\
        |xfinity|t-mobile|spectrum|cox|frontier|broadband|fiber|wireless bill\
        |waste|garbage|trash|recycling|pgande|utility|disposal|natural gas\
        |nw natural|general elect|boost mobile|google fi|mint mobile|ziply",
        SpendingCategory::Utilities,
    ),
    // Groceries — before restaurants so grocery stores match here
    (
        "grocery|whole foods|trader joe|safeway|kroger|costco|fred meyer\
        |winco|albertson|supermarket|produce|butcher|food co-op\
        |aldi|lidl|sprouts|publix|wegmans|heb |meijer|piggly|food mart",
        SpendingCategory::Groceries,
    ),
    // Restaurants — broad food/dining/drink patterns
    // Generic words (food, donut, buffet, pancake) catch local restaurants
    // that don't have obvious chain names
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
        SpendingCategory::Restaurants,
    ),
    // Transportation — fuel, rideshare, transit, travel, lodging
    // "transi" catches truncated "transit/transi". "hop " catches transit fare cards.
    (
        "uber|lyft|parking|fuel|gas station|shell|chevron|transi|trimet\
        |taxi|cab |train|airline|flight|amtrak|metro|bus |toll|ev charg\
        |bp |arco|exxon|wawa|pilot|southwest|delta air|united air|jetblue\
        |alaska air|hotel|motel|airbnb|lodge|resort|hop fares|hop fast",
        SpendingCategory::Transportation,
    ),
    // Subscriptions — recurring digital/physical services
    (
        "subscription|recurr|member|annual fee|monthly fee|patreon|substack\
        |icloud|dropbox|1password|lastpass|adobe|microsoft 365|office 365\
        |google storage|chatgpt|openai|github|notion|apple.com/bill\
        |google *|prime video|new york times|nytimes|simplicity.com",
        SpendingCategory::Subscriptions,
    ),
    // Entertainment — media, fitness, recreation, gaming, venues
    // "theatr" catches theater/theatre and truncated forms.
    // "cinema" and "regal" catch movie theater chains universally.
    (
        "netflix|spotify|hulu|theatr|movie|concert|disney|youtube|gaming\
        |steam|gym|fitness|sport|ticket|event|museum|zoo|golf|bowling\
        |arcade|apple music|audible|kindle|twitch|peacock|paramount|hbo\
        |cinema|regal |amc |casino|expo|recreation|park an|wizards\
        |moviepass|vending",
        SpendingCategory::Entertainment,
    ),
    // Shopping — broadest catch-all for retail (checked last before Other)
    (
        "amazon|target|walmart|best buy|ebay|etsy|retail|clothing|apparel\
        |furniture|home depot|lowes|ikea|hardware|office depot|staples\
        |michaels|hobby|craft|book|store|shop|purchase|jewel|marshall\
        |tj maxx|ross |nordstrom|gap |old navy|joann",
        SpendingCategory::Shopping,
    ),
];

/// Classify a transaction description into a spending category.
/// Custom rules are checked first, then built-in keyword patterns.
pub fn classify_transaction(description: &str, custom_rules: &[SpendingRule]) -> SpendingCategory {
    let lower = description.to_lowercase();

    // Custom rules first
    for rule in custom_rules {
        if lower.contains(&rule.pattern.to_lowercase()) {
            return rule.category.clone();
        }
    }

    // Built-in rules
    for (keywords, category) in BUILTIN_RULES {
        for keyword in keywords.split('|') {
            if lower.contains(keyword) {
                return category.clone();
            }
        }
    }

    SpendingCategory::Other
}

/// Compute spending summary from transactions.
/// Spending = negative amounts (debits). Income = positive amounts (credits).
pub fn compute_spending(
    transactions: &[TransactionWithContext],
    custom_rules: &[SpendingRule],
) -> SpendingSummary {
    let mut by_category: HashMap<SpendingCategory, (Decimal, usize)> = HashMap::new();
    let mut total_spending = Decimal::ZERO;
    let mut total_income = Decimal::ZERO;

    for txn in transactions {
        if txn.pending {
            continue;
        }

        let category = classify_transaction(&txn.description, custom_rules);

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
            label: cat.to_string(),
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
    }
}
