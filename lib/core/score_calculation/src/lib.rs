// Note: Add `primitive-types = "0.12"` to your Cargo.toml dependencies
use ethereum_types::U256;
use serde::{Deserialize, Serialize};

/// Trust verification levels for data validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustLevel {
    TEE = 1,      // All verification done in TEE
    RiscZero = 2, // Everything verified and calculated in RISC Zero
}

impl TrustLevel {
    /// Convert trust level to scoring multiplier
    pub fn to_multiplier(self) -> f64 {
        match self {
            TrustLevel::TEE => 0.85,
            TrustLevel::RiscZero => 1.2,
        }
    }

    /// Get fixed maximum credit limit in ETH based on trust level
    pub fn max_credit_limit_eth(self) -> U256 {
        match self {
            TrustLevel::TEE => U256::from(10),       // 10 ETH
            TrustLevel::RiscZero => U256::from(100), // 100 ETH
        }
    }

    /// Get trust level bonus points for final score
    pub fn bonus_points(self) -> u16 {
        match self {
            TrustLevel::TEE => 0,
            TrustLevel::RiscZero => 50,
        }
    }
}

/// User's credit input data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditInput {
    /// Unix timestamp of first platform interaction
    pub first_interaction_timestamp: U256,
    /// Current timestamp for age calculation
    pub current_timestamp: U256,
    /// Total number of loans paid on time
    pub on_time_payments: U256,
    /// Total number of loans that were liquidated
    pub liquidations: U256,
    /// Total ETH balance across user's accounts
    pub total_eth_balance: U256,
    /// Off-chain credit score (300-850 range, None if not provided)
    pub tradify_credit_score: Option<u16>,
    /// Trust level for data verification
    pub trust_level: TrustLevel,
}

/// Calculate comprehensive credit score
pub fn calculate_credit_score(input: &CreditInput) -> u16 {
    // Calculate individual components
    let length_score = calculate_length_of_history_score(input);
    let payment_score = calculate_payment_history_score(input);
    let available_credit_score = calculate_available_credit_score(input);
    let tradify_score = calculate_tradify_integration_score(input);
    let trust_score = calculate_trust_factor_score(input);

    // Calculate weighted final score
    calculate_weighted_score(
        length_score,
        payment_score,
        available_credit_score,
        tradify_score,
        trust_score,
        input.trust_level,
    )
}

/// Validate input data - can be called separately if needed
pub fn validate_input(input: &CreditInput) -> Result<(), String> {
    if input.current_timestamp < input.first_interaction_timestamp {
        return Err("Current timestamp cannot be before first interaction".to_string());
    }

    if let Some(score) = input.tradify_credit_score {
        if score < 300 || score > 850 {
            return Err("Off-chain credit score must be between 300-850".to_string());
        }
    }

    // Minimum balance requirement to prevent gaming
    // Since we're working with whole ETH in U256, we require at least 1 ETH
    if input.total_eth_balance < U256::from(1) {
        return Err("Minimum balance of 1 ETH required".to_string());
    }

    Ok(())
}

/// Calculate length of credit history score (15% weight)
/// Score: 300-850 based on account age
fn calculate_length_of_history_score(input: &CreditInput) -> u16 {
    const SECONDS_PER_DAY: u64 = 86400;
    const MIN_SCORE: u16 = 300;
    const MAX_SCORE: u16 = 850;

    // Handle invalid timestamps
    if input.current_timestamp < input.first_interaction_timestamp {
        return MIN_SCORE;
    }

    let account_age_seconds = input.current_timestamp - input.first_interaction_timestamp;
    // Convert U256 to u64 for calculation (safe for reasonable timestamps)
    let account_age_seconds_u64 = account_age_seconds.as_u64();
    let account_age_days = account_age_seconds_u64 / SECONDS_PER_DAY;

    // Score improves over time, max score at 2+ years (730 days)
    let score = if account_age_days == 0 {
        MIN_SCORE
    } else if account_age_days >= 730 {
        MAX_SCORE
    } else {
        // Linear progression with a small constant boost for having any history
        let progress = account_age_days as f64 / 730.0;
        let base_boost = 50; // Small boost for having any history
        MIN_SCORE + base_boost + ((MAX_SCORE - MIN_SCORE - base_boost) as f64 * progress) as u16
    };

    score.min(MAX_SCORE).max(MIN_SCORE)
}

/// Calculate payment history score (35% weight)
/// Score based on ratio of on-time payments to liquidations
fn calculate_payment_history_score(input: &CreditInput) -> u16 {
    const MIN_SCORE: u16 = 300;
    const MAX_SCORE: u16 = 850;

    let total_loans = input.on_time_payments + input.liquidations;

    if total_loans == U256::zero() {
        return 650; // Neutral score for no history
    }

    // Convert to u32 for calculations (safe for reasonable loan counts)
    let on_time_u32 = input.on_time_payments.as_u32();
    let liquidations_u32 = input.liquidations.as_u32();
    let total_loans_u32 = total_loans.as_u32();

    // Calculate success rate
    let success_rate = on_time_u32 as f64 / total_loans_u32 as f64;

    // Score based on success rate
    let base_score = match success_rate {
        r if r >= 0.95 => MAX_SCORE, // 95%+ success rate
        r if r >= 0.90 => 800,       // 90-95% success rate
        r if r >= 0.80 => 750,       // 80-90% success rate
        r if r >= 0.70 => 700,       // 70-80% success rate
        r if r >= 0.60 => 650,       // 60-70% success rate
        r if r >= 0.50 => 600,       // 50-60% success rate
        r if r >= 0.30 => 500,       // 30-50% success rate
        _ => MIN_SCORE,              // <30% success rate
    };

    // Apply penalty for having liquidations
    let liquidation_penalty = if liquidations_u32 > 0 {
        // Penalty increases with more liquidations, but caps at 150 points
        let penalty = (liquidations_u32 as f64 * 25.0).min(150.0) as u16;
        penalty
    } else {
        0
    };

    // Bonus for consistent good performance
    let consistency_bonus = if total_loans_u32 >= 10 && success_rate >= 0.95 {
        25
    } else if total_loans_u32 >= 5 && success_rate >= 0.90 {
        15
    } else {
        0
    };

    let final_score = base_score
        .saturating_sub(liquidation_penalty)
        .saturating_add(consistency_bonus);
    final_score.min(MAX_SCORE).max(MIN_SCORE)
}

/// Calculate available credit score (30% weight)
/// Score based on ETH balance showing financial capacity
fn calculate_available_credit_score(input: &CreditInput) -> u16 {
    const MIN_SCORE: u16 = 300;
    const MAX_SCORE: u16 = 850;

    // Convert U256 to u64 for ETH amount (safe for reasonable ETH amounts)
    let eth_balance = input.total_eth_balance.as_u64() as f64;

    // Handle edge cases
    if eth_balance < 1.0 {
        return MIN_SCORE; // Minimum score for balance below threshold
    }

    // Using logarithmic scaling with tiered approach
    let score = if eth_balance < 5.0 {
        // 1-5 ETH: 400-600 score range
        let progress = (eth_balance - 1.0) / 4.0;
        400 + (200.0 * progress) as u16
    } else if eth_balance < 20.0 {
        // 5-20 ETH: 600-700 score range
        let progress = (eth_balance - 5.0) / 15.0;
        600 + (100.0 * progress) as u16
    } else if eth_balance < 50.0 {
        // 20-50 ETH: 700-775 score range
        let progress = (eth_balance - 20.0) / 30.0;
        700 + (75.0 * progress) as u16
    } else if eth_balance < 100.0 {
        // 50-100 ETH: 775-825 score range
        let progress = (eth_balance - 50.0) / 50.0;
        775 + (50.0 * progress) as u16
    } else {
        // 100+ ETH: 825-850 score range with logarithmic scaling
        let log_value = (eth_balance / 100.0).ln();
        let bonus = (25.0 * log_value).min(25.0) as u16;
        825 + bonus
    };

    score.min(MAX_SCORE).max(MIN_SCORE)
}

/// Calculate off-chain credit integration score (15% weight)
fn calculate_tradify_integration_score(input: &CreditInput) -> u16 {
    match input.tradify_credit_score {
        Some(score) => {
            // Clamp to valid range if somehow invalid
            score.min(850).max(300)
        }
        None => 0, // zero score if no off-chain data provided
    }
}

/// Calculate trust factor score (5% weight)
fn calculate_trust_factor_score(input: &CreditInput) -> u16 {
    const BASE_SCORE: u16 = 650;
    const MIN_SCORE: u16 = 300;
    const MAX_SCORE: u16 = 850;

    let multiplier = input.trust_level.to_multiplier();
    let adjusted_score = (BASE_SCORE as f64 * multiplier) as u16;

    adjusted_score.min(MAX_SCORE).max(MIN_SCORE)
}

/// Calculate weighted final score
fn calculate_weighted_score(
    length_score: u16,
    payment_score: u16,
    available_credit_score: u16,
    tradify_score: u16,
    trust_score: u16,
    trust_level: TrustLevel,
) -> u16 {
    const MIN_SCORE: u16 = 300;
    const MAX_SCORE: u16 = 850;

    let weighted_sum = (payment_score as f64 * 0.35) +        // 35%
        (available_credit_score as f64 * 0.30) +               // 30%
        (tradify_score as f64 * 0.15) +                       // 15%
        (length_score as f64 * 0.15) +                        // 15%
        (trust_score as f64 * 0.05); // 5%

    let base_score = weighted_sum as u16;

    // Apply trust level bonus
    let final_score = base_score.saturating_add(trust_level.bonus_points());

    final_score.min(MAX_SCORE).max(MIN_SCORE)
}

/// Calculate credit limit based on ETH balance and trust level
/// Uses 50% LTV ratio, more conservative without debt tracking
/// Returns credit limit in ETH
pub fn calculate_credit_limit(eth_balance: U256, trust_level: TrustLevel) -> U256 {
    // Credit limit is 50% of balance, capped by trust level limit
    let ltv_limit = eth_balance / 2;
    let trust_limit = trust_level.max_credit_limit_eth();

    if ltv_limit < trust_limit {
        ltv_limit
    } else {
        trust_limit
    }
}

/// Main entry point for RISC Zero execution
pub fn calculate_score(input: CreditInput) -> u16 {
    calculate_credit_score(&input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_user_score() {
        let input = CreditInput {
            first_interaction_timestamp: U256::from(1000000000u64),
            current_timestamp: U256::from(1000086400u64), // 1 day later
            on_time_payments: U256::zero(),
            liquidations: U256::zero(),
            total_eth_balance: U256::from(5), // 5 ETH
            tradify_credit_score: None,
            trust_level: TrustLevel::TEE,
        };

        let result = calculate_credit_score(&input);

        assert!(result >= 300);
        assert!(result <= 850);
        println!("New user score: {}", result);
    }

    #[test]
    fn test_experienced_user_score() {
        let input = CreditInput {
            first_interaction_timestamp: U256::from(1000000000u64),
            current_timestamp: U256::from(1063152000u64), // ~2 years later
            on_time_payments: U256::from(10),
            liquidations: U256::zero(),
            total_eth_balance: U256::from(10), // 10 ETH
            tradify_credit_score: Some(750),
            trust_level: TrustLevel::RiscZero,
        };

        let result = calculate_credit_score(&input);

        assert!(result > 700);
        println!("Experienced user score: {}", result);
    }

    #[test]
    fn test_high_balance_bonus() {
        let input = CreditInput {
            first_interaction_timestamp: U256::from(1000000000u64),
            current_timestamp: U256::from(1031536000u64), // 1 year later
            on_time_payments: U256::from(5),
            liquidations: U256::zero(),
            total_eth_balance: U256::from(2), // 2 ETH
            tradify_credit_score: None,
            trust_level: TrustLevel::TEE,
        };

        let result = calculate_credit_score(&input);

        // With 2 ETH balance, score should be decent
        assert!(result > 550);
        println!("2 ETH balance score: {}", result);
    }

    #[test]
    fn test_perfect_payment_history() {
        let input = CreditInput {
            first_interaction_timestamp: U256::from(1000000000u64),
            current_timestamp: U256::from(1063152000u64), // ~2 years later
            on_time_payments: U256::from(20),
            liquidations: U256::zero(),        // Perfect record
            total_eth_balance: U256::from(10), // 10 ETH
            tradify_credit_score: Some(800),
            trust_level: TrustLevel::RiscZero,
        };

        let result = calculate_credit_score(&input);

        // Perfect payment history with RiscZero should yield high score
        println!("Perfect payment history score: {}", result);
        assert!(result > 800);
    }

    #[test]
    fn test_poor_payment_history() {
        let input = CreditInput {
            first_interaction_timestamp: U256::from(1000000000u64),
            current_timestamp: U256::from(1031536000u64), // 1 year later
            on_time_payments: U256::from(2),
            liquidations: U256::from(8),      // 20% success rate
            total_eth_balance: U256::from(3), // 3 ETH
            tradify_credit_score: None,
            trust_level: TrustLevel::TEE,
        };

        let result = calculate_credit_score(&input);

        // Poor payment history should result in low overall score
        assert!(result < 600);
        println!("Poor payment history score: {}", result);
    }

    #[test]
    fn test_validation_errors() {
        // Test invalid timestamp
        let invalid_timestamp_input = CreditInput {
            first_interaction_timestamp: U256::from(2000000000u64),
            current_timestamp: U256::from(1000000000u64), // Current before first interaction
            on_time_payments: U256::zero(),
            liquidations: U256::zero(),
            total_eth_balance: U256::from(1), // 1 ETH
            tradify_credit_score: None,
            trust_level: TrustLevel::TEE,
        };

        assert!(validate_input(&invalid_timestamp_input).is_err());

        // Test invalid off-chain credit score
        let invalid_credit_score_input = CreditInput {
            first_interaction_timestamp: U256::from(1000000000u64),
            current_timestamp: U256::from(1031536000u64),
            on_time_payments: U256::zero(),
            liquidations: U256::zero(),
            total_eth_balance: U256::from(1), // 1 ETH
            tradify_credit_score: Some(900),  // Invalid score > 850
            trust_level: TrustLevel::TEE,
        };

        assert!(validate_input(&invalid_credit_score_input).is_err());
    }

    #[test]
    fn test_trust_level_impact() {
        let base_input = CreditInput {
            first_interaction_timestamp: U256::from(1000000000u64),
            current_timestamp: U256::from(1031536000u64), // 1 year later
            on_time_payments: U256::from(5),
            liquidations: U256::zero(),
            total_eth_balance: U256::from(5), // 5 ETH
            tradify_credit_score: Some(700),
            trust_level: TrustLevel::TEE,
        };

        let tee_score = calculate_credit_score(&base_input);

        let risc_zero_input = CreditInput {
            trust_level: TrustLevel::RiscZero,
            ..base_input.clone()
        };

        let risc_zero_score = calculate_credit_score(&risc_zero_input);

        // RiscZero should have higher score due to trust bonus
        assert!(risc_zero_score > tee_score);
        println!(
            "TEE score: {}, RiscZero score: {}",
            tee_score, risc_zero_score
        );
    }

    #[test]
    fn test_credit_limit_calculation() {
        // Test TEE limit with 50 ETH
        let tee_limit = calculate_credit_limit(
            U256::from(50), // 50 ETH
            TrustLevel::TEE,
        );
        assert_eq!(tee_limit, U256::from(10)); // Capped at 10 ETH

        // Test RiscZero with moderate balance
        let risc_limit = calculate_credit_limit(
            U256::from(50), // 50 ETH
            TrustLevel::RiscZero,
        );
        assert_eq!(risc_limit, U256::from(25)); // 50% of 50 ETH = 25 ETH

        // Test RiscZero with high balance
        let risc_high_limit = calculate_credit_limit(
            U256::from(300), // 300 ETH
            TrustLevel::RiscZero,
        );
        assert_eq!(risc_high_limit, U256::from(100)); // Capped at 100 ETH
    }

    #[test]
    fn test_minimum_balance_requirement() {
        // Test with balance below minimum (0 ETH)
        let below_min = CreditInput {
            first_interaction_timestamp: U256::from(1000000000u64),
            current_timestamp: U256::from(1031536000u64),
            on_time_payments: U256::zero(),
            liquidations: U256::zero(),
            total_eth_balance: U256::zero(), // 0 ETH (below 1 ETH minimum)
            tradify_credit_score: None,
            trust_level: TrustLevel::TEE,
        };

        // Validation should fail
        assert!(validate_input(&below_min).is_err());

        // Score calculation handles it gracefully
        // Even with 0 balance (300 available credit score at 30% weight),
        // other components contribute: payment history (650 at 35%),
        // length of history, tradify (650 at 15%), trust factor
        let score = calculate_credit_score(&below_min);
        assert!(score > 500 && score < 600); // Weighted score will be around 530-550
        println!("Zero balance score: {}", score);

        // Test with exactly 1 ETH (should pass validation)
        let exactly_min = CreditInput {
            total_eth_balance: U256::from(1), // Exactly 1 ETH
            ..below_min.clone()
        };

        assert!(validate_input(&exactly_min).is_ok());
        let score_with_min = calculate_credit_score(&exactly_min);
        assert!(score_with_min > score); // Should be higher with 1 ETH
        println!("1 ETH balance score: {}", score_with_min);
    }

    #[test]
    fn test_various_balance_tiers() {
        // Test 1 ETH (lower tier)
        let low_balance = CreditInput {
            first_interaction_timestamp: U256::from(1000000000u64),
            current_timestamp: U256::from(1031536000u64), // 1 year
            on_time_payments: U256::from(5),
            liquidations: U256::zero(),
            total_eth_balance: U256::from(1), // 1 ETH
            tradify_credit_score: Some(650),
            trust_level: TrustLevel::TEE,
        };
        let low_score = calculate_credit_score(&low_balance);

        // Test 10 ETH (mid tier)
        let mid_balance = CreditInput {
            total_eth_balance: U256::from(10), // 10 ETH
            ..low_balance.clone()
        };
        let mid_score = calculate_credit_score(&mid_balance);

        // Test 100 ETH (high tier)
        let high_balance = CreditInput {
            total_eth_balance: U256::from(100), // 100 ETH
            ..low_balance.clone()
        };
        let high_score = calculate_credit_score(&high_balance);

        // Scores should increase with balance
        assert!(low_score < mid_score);
        assert!(mid_score < high_score);
        println!(
            "Balance tier scores - 1 ETH: {}, 10 ETH: {}, 100 ETH: {}",
            low_score, mid_score, high_score
        );
    }

    #[test]
    fn test_payment_history_with_liquidations() {
        // Test with varying liquidation counts
        let mut input = CreditInput {
            first_interaction_timestamp: U256::from(1000000000u64),
            current_timestamp: U256::from(1031536000u64),
            on_time_payments: U256::from(10),
            liquidations: U256::from(1),      // 1 liquidation
            total_eth_balance: U256::from(5), // 5 ETH
            tradify_credit_score: Some(700),
            trust_level: TrustLevel::TEE,
        };

        let score_1_liquidation = calculate_credit_score(&input);

        input.liquidations = U256::from(3); // 3 liquidations
        let score_3_liquidations = calculate_credit_score(&input);

        input.liquidations = U256::from(10); // 10 liquidations (50% success rate)
        let score_10_liquidations = calculate_credit_score(&input);

        // More liquidations should result in lower scores
        assert!(score_1_liquidation > score_3_liquidations);
        assert!(score_3_liquidations > score_10_liquidations);
        println!(
            "Liquidation impact - 1: {}, 3: {}, 10: {}",
            score_1_liquidation, score_3_liquidations, score_10_liquidations
        );
    }
}