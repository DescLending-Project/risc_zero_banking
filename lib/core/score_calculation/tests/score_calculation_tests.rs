use serde::{Deserialize, Serialize};

/// Conversion constant
const WEI_PER_ETH: f64 = 1_000_000_000_000_000_000.0;

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
    pub fn max_credit_limit_eth(self) -> f64 {
        match self {
            TrustLevel::TEE => 10.0,       // 10 ETH
            TrustLevel::RiscZero => 100.0, // 100 ETH
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
    pub first_interaction_timestamp: u64,
    /// Current timestamp for age calculation
    pub current_timestamp: u64,
    /// Total number of loans paid on time
    pub on_time_payments: u32,
    /// Total number of loans that were liquidated
    pub liquidations: u32,
    /// Total ETH balance across user's accounts
    pub total_eth_balance: f64,
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
    if input.total_eth_balance < 0.1 {
        return Err("Minimum balance of 0.1 ETH required".to_string());
    }

    if input.total_eth_balance < 0.0 {
        return Err("ETH balance cannot be negative".to_string());
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
    let account_age_days = account_age_seconds / SECONDS_PER_DAY;

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

    if total_loans == 0 {
        return 650; // Neutral score for no history
    }

    // Calculate success rate
    let success_rate = input.on_time_payments as f64 / total_loans as f64;

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
    let liquidation_penalty = if input.liquidations > 0 {
        // Penalty increases with more liquidations, but caps at 150 points
        let penalty = (input.liquidations as f64 * 25.0).min(150.0) as u16;
        penalty
    } else {
        0
    };

    // Bonus for consistent good performance
    let consistency_bonus = if total_loans >= 10 && success_rate >= 0.95 {
        25
    } else if total_loans >= 5 && success_rate >= 0.90 {
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

    let eth_balance = input.total_eth_balance;

    // Handle edge cases
    if eth_balance < 0.1 {
        return MIN_SCORE; // Minimum score for balance below threshold
    }

    // Using logarithmic scaling with tiered approach
    let score = if eth_balance < 1.0 {
        // 0.1-1 ETH: 400-500 score range
        let progress = (eth_balance - 0.1) / 0.9;
        400 + (100.0 * progress) as u16
    } else if eth_balance < 5.0 {
        // 1-5 ETH: 500-650 score range
        let progress = (eth_balance - 1.0) / 4.0;
        500 + (150.0 * progress) as u16
    } else if eth_balance < 20.0 {
        // 5-20 ETH: 650-750 score range
        let progress = (eth_balance - 5.0) / 15.0;
        650 + (100.0 * progress) as u16
    } else if eth_balance < 50.0 {
        // 20-50 ETH: 750-800 score range
        let progress = (eth_balance - 20.0) / 30.0;
        750 + (50.0 * progress) as u16
    } else {
        // 50+ ETH: 800-850 score range with logarithmic scaling
        // Using logarithmic formula to prevent ultra-wealthy from auto-maxing
        let log_value = (eth_balance / 50.0).ln();
        let bonus = (50.0 * log_value).min(50.0) as u16;
        800 + bonus
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
        None => 650, // Neutral score if no off-chain data provided
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
pub fn calculate_credit_limit(eth_balance: f64, trust_level: TrustLevel) -> f64 {
    // Credit limit is 50% of balance, capped by trust level limit
    let ltv_limit = eth_balance * 0.5;
    let trust_limit = trust_level.max_credit_limit_eth();

    ltv_limit.min(trust_limit)
}

/// Convert wei amount to ETH
pub fn wei_to_eth(wei: u128) -> f64 {
    wei as f64 / WEI_PER_ETH
}

/// Convert ETH amount to wei
pub fn eth_to_wei(eth: f64) -> u128 {
    (eth * WEI_PER_ETH) as u128
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
            first_interaction_timestamp: 1000000000,
            current_timestamp: 1000086400, // 1 day later
            on_time_payments: 0,
            liquidations: 0,
            total_eth_balance: 5.0, // 5 ETH
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
            first_interaction_timestamp: 1000000000,
            current_timestamp: 1063152000, // ~2 years later
            on_time_payments: 10,
            liquidations: 0,
            total_eth_balance: 10.0, // 10 ETH
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
            first_interaction_timestamp: 1000000000,
            current_timestamp: 1031536000, // 1 year later
            on_time_payments: 5,
            liquidations: 0,
            total_eth_balance: 25.0, // 25 ETH - high balance
            tradify_credit_score: None,
            trust_level: TrustLevel::TEE,
        };

        let result = calculate_credit_score(&input);

        // High balance should contribute to a good score
        assert!(result > 600);
        println!("High balance score: {}", result);
    }

    #[test]
    fn test_perfect_payment_history() {
        let input = CreditInput {
            first_interaction_timestamp: 1000000000,
            current_timestamp: 1063152000, // ~2 years later
            on_time_payments: 20,
            liquidations: 0,         // Perfect record
            total_eth_balance: 10.0, // 10 ETH
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
            first_interaction_timestamp: 1000000000,
            current_timestamp: 1031536000, // 1 year later
            on_time_payments: 2,
            liquidations: 8,        // 20% success rate
            total_eth_balance: 3.0, // 3 ETH
            tradify_credit_score: None,
            trust_level: TrustLevel::TEE,
        };

        let result = calculate_credit_score(&input);

        // Poor payment history should result in low overall score
        assert!(result < 600);
        println!("Poor payment history score: {}", result);
    }

    #[test]
    fn test_edge_case_low_eth_balance() {
        let input = CreditInput {
            first_interaction_timestamp: 1000000000,
            current_timestamp: 1031536000, // 1 year later
            on_time_payments: 3,
            liquidations: 0,
            total_eth_balance: 0.05, // Below minimum 0.1 ETH
            tradify_credit_score: None,
            trust_level: TrustLevel::TEE,
        };

        let result = calculate_credit_score(&input);

        // Low ETH balance should result in minimum available credit score
        // But function still returns a valid score
        assert!(result >= 300 && result <= 850);
        println!("Low ETH balance score: {}", result);
    }

    #[test]
    fn test_validation_errors() {
        // Test invalid timestamp
        let invalid_timestamp_input = CreditInput {
            first_interaction_timestamp: 2000000000,
            current_timestamp: 1000000000, // Current before first interaction
            on_time_payments: 0,
            liquidations: 0,
            total_eth_balance: 1.0,
            tradify_credit_score: None,
            trust_level: TrustLevel::TEE,
        };

        // Score calculation handles this gracefully now
        let score = calculate_credit_score(&invalid_timestamp_input);
        assert!(score >= 300 && score <= 850);

        // But validation would catch it
        assert!(validate_input(&invalid_timestamp_input).is_err());

        // Test invalid off-chain credit score
        let invalid_credit_score_input = CreditInput {
            first_interaction_timestamp: 1000000000,
            current_timestamp: 1031536000,
            on_time_payments: 0,
            liquidations: 0,
            total_eth_balance: 1.0,
            tradify_credit_score: Some(900), // Invalid score > 850
            trust_level: TrustLevel::TEE,
        };

        // Score calculation handles this by clamping
        let score = calculate_credit_score(&invalid_credit_score_input);
        assert!(score >= 300 && score <= 850);

        // But validation would catch it
        assert!(validate_input(&invalid_credit_score_input).is_err());

        // Test below minimum balance
        let below_min_balance = CreditInput {
            first_interaction_timestamp: 1000000000,
            current_timestamp: 1031536000,
            on_time_payments: 0,
            liquidations: 0,
            total_eth_balance: 0.05, // Below 0.1 ETH minimum
            tradify_credit_score: None,
            trust_level: TrustLevel::TEE,
        };

        assert!(validate_input(&below_min_balance).is_err());
    }

    #[test]
    fn test_trust_level_impact() {
        let base_input = CreditInput {
            first_interaction_timestamp: 1000000000,
            current_timestamp: 1031536000, // 1 year later
            on_time_payments: 5,
            liquidations: 0,
            total_eth_balance: 5.0,
            tradify_credit_score: Some(700),
            trust_level: TrustLevel::TEE,
        };

        let tee_score = calculate_credit_score(&base_input);

        let risc_zero_input = CreditInput {
            trust_level: TrustLevel::RiscZero,
            ..base_input
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
        // Test TEE limit
        let tee_limit = calculate_credit_limit(50.0, TrustLevel::TEE);
        assert_eq!(tee_limit, 10.0); // Capped at 10 ETH for TEE

        // Test RiscZero with moderate balance
        let risc_limit = calculate_credit_limit(50.0, TrustLevel::RiscZero);
        assert_eq!(risc_limit, 25.0); // 50% of 50 ETH = 25 ETH

        // Test RiscZero with high balance
        let risc_high_limit = calculate_credit_limit(300.0, TrustLevel::RiscZero);
        assert_eq!(risc_high_limit, 100.0); // Capped at 100 ETH for RiscZero
    }
}
