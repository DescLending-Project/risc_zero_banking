use serde::{Deserialize, Serialize};

/// Trust verification levels for data validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustLevel {
    Basic = 1,
    Enhanced = 2,
    Premium = 3,
    Platinum = 4,
}

impl TrustLevel {
    /// Convert trust level to scoring multiplier
    pub fn to_multiplier(self) -> f64 {
        match self {
            TrustLevel::Basic => 0.7,
            TrustLevel::Enhanced => 0.85,
            TrustLevel::Premium => 1.0,
            TrustLevel::Platinum => 1.2,
        }
    }

    /// Get fixed maximum credit limit in ETH (wei) based on trust level
    pub fn max_credit_limit_wei(self) -> u128 {
        match self {
            TrustLevel::Basic => 5_000_000_000_000_000_000, // 5 ETH
            TrustLevel::Enhanced => 15_000_000_000_000_000_000, // 15 ETH
            TrustLevel::Premium => 50_000_000_000_000_000_000, // 50 ETH
            TrustLevel::Platinum => 150_000_000_000_000_000_000, // 150 ETH
        }
    }
}

/// Payment history summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentHistory {
    /// Total number of loans paid on time
    pub on_time_payments: u32,
    /// Total number of loans that were liquidated
    pub liquidations: u32,
}

/// User's credit input data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditInput {
    /// Block number of first platform interaction
    pub first_interaction_block: u64,
    /// Current block number
    pub current_block: u64,
    /// Payment history summary
    pub payment_history: PaymentHistory,
    /// Total ETH balance across user's accounts (in wei)
    pub total_eth_balance: u128,
    /// Current debt in the system (in wei)
    pub current_debt: u128,
    /// Off-chain credit score (300-850 range, None if not provided)
    pub tradify_credit_score: Option<u16>,
    /// Trust level for data verification
    pub trust_level: TrustLevel,
}

/// Detailed credit score breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditScoreBreakdown {
    pub length_of_history_score: u16,
    pub payment_history_score: u16,
    pub credit_utilization_score: u16,
    pub tradify_integration_score: u16,
    pub trust_factor_score: u16,
    pub final_score: u16,
}

/// Calculate comprehensive credit score - ROBUST VERSION
pub fn calculate_credit_score(input: &CreditInput) -> Result<CreditScoreBreakdown, String> {
    // Simplified validation - only check truly invalid cases
    if input.current_block < input.first_interaction_block {
        return Err("Invalid block sequence".to_string());
    }

    if let Some(score) = input.tradify_credit_score {
        if score > 850 {
            // Allow scores below 300 - just clamp them later
            return Err("TradFi score too high".to_string());
        }
    }

    // Calculate all components with safe math
    let length_score = calculate_length_score_safe(input);
    let payment_score = calculate_payment_score_safe(input);
    let utilization_score = calculate_utilization_score_safe(input);
    let tradify_score = calculate_tradify_score_safe(input);
    let trust_score = calculate_trust_score_safe(input);

    // Calculate final score with safe weighted average
    let final_score = calculate_final_score_safe(
        length_score,
        payment_score,
        utilization_score,
        tradify_score,
        trust_score,
    );

    Ok(CreditScoreBreakdown {
        length_of_history_score: length_score,
        payment_history_score: payment_score,
        credit_utilization_score: utilization_score,
        tradify_integration_score: tradify_score,
        trust_factor_score: trust_score,
        final_score,
    })
}

/// Safe clamp function to ensure scores stay in valid range
fn clamp_score(score: u16) -> u16 {
    score.min(850).max(300)
}

/// Calculate length of history score - SAFE VERSION
fn calculate_length_score_safe(input: &CreditInput) -> u16 {
    let account_age_blocks = input.current_block.saturating_sub(input.first_interaction_block);
    
    // Map age to score using simple linear interpolation
    // 0 blocks = 300, 2+ years (5.2M blocks) = 850
    const MAX_AGE_BLOCKS: u64 = 5_256_000; // ~2 years
    
    if account_age_blocks == 0 {
        300
    } else if account_age_blocks >= MAX_AGE_BLOCKS {
        850
    } else {
        // Safe linear interpolation
        let progress = (account_age_blocks as f64) / (MAX_AGE_BLOCKS as f64);
        let score = 300.0 + (550.0 * progress); // 300 + (850-300) * progress
        clamp_score(score as u16)
    }
}

/// Calculate payment history score - SAFE VERSION
fn calculate_payment_score_safe(input: &CreditInput) -> u16 {
    let on_time = input.payment_history.on_time_payments;
    let liquidations = input.payment_history.liquidations;
    let total = on_time.saturating_add(liquidations);

    if total == 0 {
        return 650; // Neutral for no history
    }

    // Calculate success rate safely
    let success_rate = if total == 0 {
        0.0
    } else {
        (on_time as f64) / (total as f64)
    };

    // Map success rate to score
    let base_score: u16 = if success_rate >= 0.95 {
        850u16
    } else if success_rate >= 0.90 {
        800u16
    } else if success_rate >= 0.80 {
        750u16
    } else if success_rate >= 0.70 {
        700u16
    } else if success_rate >= 0.60 {
        650u16
    } else if success_rate >= 0.50 {
        600u16
    } else if success_rate >= 0.30 {
        500u16
    } else {
        300u16
    };

    // Apply liquidation penalty (max 150 points)
    let penalty: u16 = if liquidations > 0 {
        (liquidations.saturating_mul(25)).min(150) as u16
    } else {
        0u16
    };

    clamp_score(base_score.saturating_sub(penalty))
}

/// Calculate credit utilization score - SAFE VERSION
fn calculate_utilization_score_safe(input: &CreditInput) -> u16 {
    // Handle zero debt case immediately
    if input.current_debt == 0 {
        return 850; // Perfect utilization
    }

    // Calculate credit limit safely
    let credit_limit = calculate_credit_limit_safe(input.total_eth_balance, input.trust_level);
    
    if credit_limit == 0 {
        return 300; // No credit available
    }

    // Handle case where debt exceeds limit
    if input.current_debt >= credit_limit {
        return 300; // Maxed out or over limit
    }

    // Safe utilization calculation using integer math where possible
    // Convert to percentage to avoid floating point precision issues
    let utilization_percent: u128 = (input.current_debt.saturating_mul(100)) / credit_limit;

    // Map utilization percentage to score
    if utilization_percent <= 10u128 {
        850 // Excellent: 0-10%
    } else if utilization_percent <= 30u128 {
        // Linear interpolation between 850 and 750
        let excess: u128 = utilization_percent - 10u128;
        let penalty: u16 = (excess * 5u128) as u16; // 5 points per percent over 10%
        clamp_score(850u16.saturating_sub(penalty))
    } else if utilization_percent <= 50u128 {
        // Linear interpolation between 750 and 600
        let excess: u128 = utilization_percent - 30u128;
        let penalty: u16 = (excess * 7u128) as u16 + 100u16; // Base 100 penalty + 7 per percent
        clamp_score(850u16.saturating_sub(penalty))
    } else if utilization_percent <= 80u128 {
        // Linear interpolation between 600 and 400
        let excess: u128 = utilization_percent - 50u128;
        let penalty: u16 = (excess * 6u128) as u16 + 250u16; // Base 250 penalty + 6 per percent
        clamp_score(850u16.saturating_sub(penalty))
    } else {
        300 // Very poor: 80%+
    }
}

/// Calculate TradFi integration score - SAFE VERSION
fn calculate_tradify_score_safe(input: &CreditInput) -> u16 {
    match input.tradify_credit_score {
        Some(score) => {
            // Clamp input score first, then multiply safely
            let clamped_input = score.min(850).max(1); // Min 1 to avoid zero
            let result = clamped_input.saturating_mul(20);
            clamp_score(result)
        },
        None => 650, // Neutral score
    }
}

/// Calculate trust factor score - SAFE VERSION
fn calculate_trust_score_safe(input: &CreditInput) -> u16 {
    let base_score = 650;
    let multiplier = input.trust_level.to_multiplier();
    
    // Safe floating point to integer conversion
    let adjusted = (base_score as f64 * multiplier).round() as u16;
    
    // Add trust level bonus
    let bonus: u16 = match input.trust_level {
        TrustLevel::Basic => 0,
        TrustLevel::Enhanced => 50,
        TrustLevel::Premium => 100,
        TrustLevel::Platinum => 150,
    };

    clamp_score(adjusted.saturating_add(bonus))
}

/// Calculate final weighted score - SAFE VERSION
fn calculate_final_score_safe(
    length_score: u16,
    payment_score: u16,
    utilization_score: u16,
    tradify_score: u16,
    trust_score: u16,
) -> u16 {
    // Use integer arithmetic to avoid floating point precision issues
    // Multiply by 1000 to preserve precision, then divide at the end
    
    let weighted_sum = 
        (payment_score as u32 * 300) +      // 30% = 300/1000
        (utilization_score as u32 * 300) +  // 30% = 300/1000
        (tradify_score as u32 * 150) +      // 15% = 150/1000
        (length_score as u32 * 150) +       // 15% = 150/1000
        (trust_score as u32 * 100);         // 10% = 100/1000

    let final_score: u16 = (weighted_sum / 1000) as u16;
    clamp_score(final_score)
}

/// Calculate credit limit safely
fn calculate_credit_limit_safe(eth_balance_wei: u128, trust_level: TrustLevel) -> u128 {
    let trust_limit = trust_level.max_credit_limit_wei();
    eth_balance_wei.min(trust_limit)
}

/// Calculate credit limit for external use
pub fn calculate_credit_limit(eth_balance_wei: u128, trust_level: TrustLevel) -> u128 {
    calculate_credit_limit_safe(eth_balance_wei, trust_level)
}

/// Main entry point for RISC Zero execution - ROBUST VERSION
pub fn calculate_score(input: CreditInput) -> Result<CreditScoreBreakdown, String> {
    calculate_credit_score(&input)
}