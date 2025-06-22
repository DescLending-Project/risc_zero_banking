use score_calculation::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_user_score() {
        let input = CreditInput {
            first_interaction_timestamp: 1000000000,
            current_timestamp: 1000086400, // 1 day later
            payment_history: PaymentHistory {
                on_time_payments: 0,
                liquidations: 0,
            },
            total_eth_balance: 5_000_000_000_000_000_000, // 5 ETH
            current_debt: 0,
            tradify_credit_score: None,
            trust_level: TrustLevel::Basic,
        };

        let result = calculate_credit_score(&input).unwrap();

        assert!(result.final_score >= 300);
        assert!(result.final_score <= 850);
        println!("New user score: {}", result.final_score);
    }

    #[test]
    fn test_experienced_user_score() {
        let input = CreditInput {
            first_interaction_timestamp: 1000000000,
            current_timestamp: 1063152000, // ~2 years later
            payment_history: PaymentHistory {
                on_time_payments: 10,
                liquidations: 0,
            },
            total_eth_balance: 10_000_000_000_000_000_000, // 10 ETH
            current_debt: 1_000_000_000_000_000_000,       // 1 ETH debt
            tradify_credit_score: Some(750),
            trust_level: TrustLevel::Platinum,
        };

        let result = calculate_credit_score(&input).unwrap();

        assert!(result.final_score > 700);
        println!("Experienced user score: {}", result.final_score);
    }

    #[test]
    fn test_high_utilization_penalty() {
        let input = CreditInput {
            first_interaction_timestamp: 1000000000,
            current_timestamp: 1031536000, // 1 year later
            payment_history: PaymentHistory {
                on_time_payments: 5,
                liquidations: 0,
            },
            total_eth_balance: 2_000_000_000_000_000_000, // 2 ETH
            current_debt: 1_500_000_000_000_000_000,      // 1.5 ETH debt (high utilization)
            tradify_credit_score: None,
            trust_level: TrustLevel::Enhanced,
        };

        let result = calculate_credit_score(&input).unwrap();

        // High utilization should result in lower score
        assert!(result.credit_utilization_score < 600);
        println!("High utilization score: {}", result.final_score);
    }

    #[test]
    fn test_perfect_payment_history() {
        let input = CreditInput {
            first_interaction_timestamp: 1000000000,
            current_timestamp: 1063152000, // ~2 years later
            payment_history: PaymentHistory {
                on_time_payments: 20,
                liquidations: 0, // Perfect record
            },
            total_eth_balance: 10_000_000_000_000_000_000, // 10 ETH
            current_debt: 500_000_000_000_000_000,         // 0.5 ETH debt (low utilization)
            tradify_credit_score: Some(800),
            trust_level: TrustLevel::Platinum,
        };

        let result = calculate_credit_score(&input).unwrap();

        // Perfect payment history should yield maximum payment score
        println!("Perfect payment history score: {:?}", result);
        println!("Perfect payment history score: {}", result.final_score);
        assert_eq!(result.payment_history_score, 850);
        assert!(result.final_score > 800);
    }

    #[test]
    fn test_poor_payment_history() {
        let input = CreditInput {
            first_interaction_timestamp: 1000000000,
            current_timestamp: 1031536000, // 1 year later
            payment_history: PaymentHistory {
                on_time_payments: 2,
                liquidations: 8, // 20% success rate
            },
            total_eth_balance: 3_000_000_000_000_000_000, // 3 ETH
            current_debt: 100_000_000_000_000_000,        // 0.1 ETH debt
            tradify_credit_score: None,
            trust_level: TrustLevel::Basic,
        };

        let result = calculate_credit_score(&input).unwrap();

        // Poor payment history should result in low payment score
        assert!(result.payment_history_score <= 500);
        println!(
            "Poor payment history score: {}",
            result.payment_history_score
        );
    }

    #[test]
    fn test_edge_case_no_eth_balance() {
        let input = CreditInput {
            first_interaction_timestamp: 1000000000,
            current_timestamp: 1031536000, // 1 year later
            payment_history: PaymentHistory {
                on_time_payments: 3,
                liquidations: 0,
            },
            total_eth_balance: 0, // No ETH balance
            current_debt: 0,
            tradify_credit_score: None,
            trust_level: TrustLevel::Basic,
        };

        let result = calculate_credit_score(&input).unwrap();

        // No ETH balance should result in minimum credit utilization score
        assert_eq!(result.credit_utilization_score, 300);
        println!("No ETH balance score: {}", result.final_score);
    }

    #[test]
    fn test_validation_errors() {
        // Test invalid timestamp
        let invalid_timestamp_input = CreditInput {
            first_interaction_timestamp: 2000000000,
            current_timestamp: 1000000000, // Current before first interaction
            payment_history: PaymentHistory {
                on_time_payments: 0,
                liquidations: 0,
            },
            total_eth_balance: 1_000_000_000_000_000_000,
            current_debt: 0,
            tradify_credit_score: None,
            trust_level: TrustLevel::Basic,
        };

        assert!(calculate_credit_score(&invalid_timestamp_input).is_err());

        // Test invalid off-chain credit score
        let invalid_credit_score_input = CreditInput {
            first_interaction_timestamp: 1000000000,
            current_timestamp: 1031536000,
            payment_history: PaymentHistory {
                on_time_payments: 0,
                liquidations: 0,
            },
            total_eth_balance: 1_000_000_000_000_000_000,
            current_debt: 0,
            tradify_credit_score: Some(900), // Invalid score > 850
            trust_level: TrustLevel::Basic,
        };

        assert!(calculate_credit_score(&invalid_credit_score_input).is_err());
    }
}
