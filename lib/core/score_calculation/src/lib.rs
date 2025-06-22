use serde::{Deserialize, Serialize};

/// Trust verification levels for data validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustLevel {
    Basic = 1, // all data verification happend on TEE user only verifys the signature in RISC Zero
    Enhanced = 2, // verifcation of TLSN proofs on TEE, user verifys signature of stateHashRoots
    // tradify score, merkle proofs, nullifiers and the signature created with owned
    // accounts
    Platinum = 3, // everything verified in RISC zero
}

impl TrustLevel {
    /// Convert trust level to scoring multiplier
    pub fn to_multiplier(self) -> f64 {
        match self {
            TrustLevel::Basic => 0.7,
            TrustLevel::Enhanced => 0.85,
            TrustLevel::Platinum => 1.0,
        }
    }

    /// Get fixed maximum credit limit in ETH (wei) based on trust level
    /// NOTE: Should be probably adjusted, to be discussed.
    pub fn max_credit_limit_wei(self) -> u128 {
        match self {
            TrustLevel::Basic => 5_000_000_000_000_000_000, // 5 ETH
            TrustLevel::Enhanced => 15_000_000_000_000_000_000, // 15 ETH
            TrustLevel::Platinum => 50_000_000_000_000_000_000, // 50 ETH
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
    /// Unix timestamp of first platform interaction
    pub first_interaction_timestamp: u64,
    /// Current timestamp for age calculation
    pub current_timestamp: u64,
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

/// Calculate comprehensive credit score
pub fn calculate_credit_score(input: &CreditInput) -> Result<CreditScoreBreakdown, String> {
    // Validate input
    validate_input(input)?;

    // Calculate individual components
    let length_score = calculate_length_of_history_score(input);
    let payment_score = calculate_payment_history_score(input);
    let utilization_score = calculate_credit_utilization_score(input);
    let tradify_score = calculate_tradify_integration_score(input);
    let trust_score = calculate_trust_factor_score(input);

    // Calculate weighted final score
    let final_score = calculate_weighted_score(
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

/// Validate input data. This should only include validation logic as verification will be already
/// done before.
fn validate_input(input: &CreditInput) -> Result<(), String> {
    if input.current_timestamp < input.first_interaction_timestamp {
        return Err("Current timestamp cannot be before first interaction".to_string());
    }

    if let Some(score) = input.tradify_credit_score {
        if score < 300 || score > 850 {
            return Err("Off-chain credit score must be between 300-850".to_string());
        }
    }

    if input.current_debt > u128::from(u64::MAX) {
        return Err("Debt amount too large".to_string());
    }

    Ok(())
}

/// Calculate length of credit history score (10% weight)
/// Score: 300-850 based on account age
fn calculate_length_of_history_score(input: &CreditInput) -> u16 {
    const SECONDS_PER_DAY: u64 = 86400;
    const MIN_SCORE: u16 = 300;
    const MAX_SCORE: u16 = 850;

    let account_age_seconds = input.current_timestamp - input.first_interaction_timestamp;
    let account_age_days = account_age_seconds / SECONDS_PER_DAY;

    // Score improves over time, max score at 2+ years (730 days)
    let score = if account_age_days == 0 {
        MIN_SCORE
    } else if account_age_days >= 730 {
        MAX_SCORE
    } else {
        // NOTE: adding some kind of contstant factor could be done here
        let progress = account_age_days as f64 / 730.0;
        MIN_SCORE + ((MAX_SCORE - MIN_SCORE) as f64 * progress) as u16
    };

    score.min(MAX_SCORE).max(MIN_SCORE)
}

/// Calculate payment history score (30% weight)
/// Score based on ratio of on-time payments to liquidations
fn calculate_payment_history_score(input: &CreditInput) -> u16 {
    const MIN_SCORE: u16 = 300;
    const MAX_SCORE: u16 = 850;

    let total_loans = input.payment_history.on_time_payments + input.payment_history.liquidations;

    if total_loans == 0 {
        return 650; // Neutral score for no history
    }

    // Calculate success rate
    let success_rate = input.payment_history.on_time_payments as f64 / total_loans as f64;

    // Score based on success rate
    // NOTE: This mapping should be discussed
    // maybe leeting to specify it as ENV variable would be cool
    let base_score = match success_rate {
        r if r >= 0.95 => MAX_SCORE, //,uccess rate
        r if r >= 0.90 => 800,       // 90-95% success rate
        r if r >= 0.80 => 750,       //80-90% success rate
        r if r >= 0.70 => 700,       //    success rate
        r if r >= 0.60 => 650,       // 60-70% success rate
        r if r >= 0.50 => 600,       //50-60% success rate
        r if r >= 0.30 => 500,       //      success rate
        _ => MIN_SCORE,              //    uccess rate
    };

    // Apply penalty for having liquidations
    let liquidation_penalty = if input.payment_history.liquidations > 0 {
        // Penalty increases with more liquidations, but caps at 150 points
        // NOTE: the cap should be discussed
        let penalty = (input.payment_history.liquidations as f64 * 25.0).min(150.0) as u16;
        penalty
    } else {
        0
    };

    let final_score = base_score.saturating_sub(liquidation_penalty);
    final_score.min(MAX_SCORE).max(MIN_SCORE)
}

/// Calculate credit utilization score (25% weight)
/// Score based on debt-to-credit-limit ratio
fn calculate_credit_utilization_score(input: &CreditInput) -> u16 {
    const MIN_SCORE: u16 = 300;
    const MAX_SCORE: u16 = 850;

    // Get credit limit based on trust level and ETH balance
    let credit_limit_wei = calculate_credit_limit(input.total_eth_balance, input.trust_level);

    if credit_limit_wei == 0 {
        return MIN_SCORE;
    }

    // Calculate utilization ratio
    let utilization_ratio = input.current_debt as f64 / credit_limit_wei as f64;

    // Score decreases with higher utilization
    let score = if utilization_ratio <= 0.1 {
        MAX_SCORE // Excellent: 0-10% utilization
    } else if utilization_ratio <= 0.3 {
        MAX_SCORE - ((utilization_ratio - 0.1) / 0.2 * 100.0) as u16 // Good: 10-30%
    } else if utilization_ratio <= 0.5 {
        750 - ((utilization_ratio - 0.3) / 0.2 * 150.0) as u16 // Fair: 30-50%
    } else if utilization_ratio <= 0.8 {
        600 - ((utilization_ratio - 0.5) / 0.3 * 200.0) as u16 // Poor: 50-80%
    } else {
        MIN_SCORE // Very Poor: 80%+
    };

    score.min(MAX_SCORE).max(MIN_SCORE)
}

/// Calculate off-chain credit integration score (15% weight)
fn calculate_tradify_integration_score(input: &CreditInput) -> u16 {
    match input.tradify_credit_score {
        Some(score) => score,
        None => 650, // Neutral score if no off-chain data provided
    }
}

/// Calculate trust factor score (10% weight)
fn calculate_trust_factor_score(input: &CreditInput) -> u16 {
    const BASE_SCORE: u16 = 650;
    const MIN_SCORE: u16 = 300;
    const MAX_SCORE: u16 = 850;

    let multiplier = input.trust_level.to_multiplier();
    let adjusted_score = (BASE_SCORE as f64 * multiplier) as u16;

    // Bonus for higher trust levels
    let trust_bonus = match input.trust_level {
        TrustLevel::Basic => 0,
        TrustLevel::Enhanced => 50,
        TrustLevel::Platinum => 100,
    };

    (adjusted_score + trust_bonus).min(MAX_SCORE).max(MIN_SCORE)
}

/// Calculate weighted final score
fn calculate_weighted_score(
    length_score: u16,
    payment_score: u16,
    utilization_score: u16,
    tradify_score: u16,
    trust_score: u16,
) -> u16 {
    const MIN_SCORE: u16 = 300;
    const MAX_SCORE: u16 = 850;

    let weighted_sum = (payment_score as f64 * 0.30) +     // 30%
        (utilization_score as f64 * 0.30) + // 30%
        (tradify_score as f64 * 0.15) +    // 15%
        (length_score as  f64 * 0.15) +    // 10%
        (trust_score as f64 * 0.10); // 10%

    let final_score = weighted_sum as u16;
    final_score.min(MAX_SCORE).max(MIN_SCORE)
}

/// Calculate credit limit based on ETH balance and trust level
pub fn calculate_credit_limit(eth_balance_wei: u128, trust_level: TrustLevel) -> u128 {
    // Credit limit is the minimum of ETH balance and trust level limit
    let trust_limit = trust_level.max_credit_limit_wei();
    eth_balance_wei.min(trust_limit)
}

/// Main entry point for RISC Zero execution
pub fn calculate_score(input: CreditInput) -> Result<CreditScoreBreakdown, String> {
    calculate_credit_score(&input)
}
