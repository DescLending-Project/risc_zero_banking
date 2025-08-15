#!/bin/bash

# Array of slice lengths (number of accounts)
slice_lengths=(1 )
# slice_lengths=(1 3 5 8 10 15 20)


echo "Starting RISC Zero proof generation for different account counts..."
echo "Working directory: $(pwd)"
echo ""
echo "Expected input files in ../../evaluation_inputs/:"
for accounts in "${slice_lengths[@]}"; do
    echo "  - ${accounts}_signatures.json"
    echo "  - ${accounts}_nullifiers.json"
    echo "  - ${accounts}_all_merkle_proofs.json"
done
echo ""
cd ../../risc0_proofs/defi_inputs_validation/

# Loop through each slice length
for accounts in "${slice_lengths[@]}"; do
    echo ""
    echo "========================================="
    echo "Generating proof for $accounts accounts"
    echo "========================================="
    
    # Create output directories if they don't exist
    mkdir -p "../../score_publisher/host/receipts/${accounts}_accounts_proof/"
    mkdir -p "../../evaluation/inputs_validation_scaling/metrics/"
    
    # Check if input files exist
    signatures_file="../../evaluation/inputs_validation_scaling/evaluation_inputs/${accounts}_signatures.json"
    nullifiers_file="../../evaluation/inputs_validation_scaling/evaluation_inputs/${accounts}_nullifiers.json"
    merkle_proofs_file="../../evaluation/inputs_validation_scaling/evaluation_inputs/${accounts}_all_merkle_proofs.json"
    
    if [[ ! -f "$signatures_file" || ! -f "$nullifiers_file" || ! -f "$merkle_proofs_file" ]]; then
        echo "⚠️  Input files for $accounts accounts not found. Please generate them first:"
        echo "   Missing: $signatures_file"
        echo "   Missing: $nullifiers_file" 
        echo "   Missing: $merkle_proofs_file"
        echo "   Skipping $accounts accounts..."
        continue
    fi
    
    echo "✓ Found all input files for $accounts accounts"

    (
RUST_LOG="[executor]=info"  RISC0_DEV_MODE=0 cargo run -- \
        --all-signatures-path "../../evaluation/inputs_validation_scaling/evaluation_inputs/${accounts}_signatures.json" \
        --all-nullifiers-path "../../evaluation/inputs_validation_scaling/evaluation_inputs/${accounts}_nullifiers.json" \
        --all-merkle-proofs-path "../../evaluation/inputs_validation_scaling/evaluation_inputs/${accounts}_all_merkle_proofs.json" \
        --user-owned-addresses-path "../../evaluation/inputs_validation_scaling/evaluation_inputs/${accounts}_accounts.json" \
        --proof-name "valid_defi_inputs_receipt_${accounts}" \
        --bin-output-path "../../score_publisher/host/receipts/${accounts}_accounts_proof/" \
    ) 2> stderr_output.txt 1> stdout.txt

  

    # Then check both files
    exit_code=${PIPESTATUS[0]}
    # Check if the command was successful
   if [ $exit_code -eq 0 ]; then
        echo "✓ Successfully generated proof for $accounts accounts"
        
        echo "Parsing cycle counts from command output..."
        output=$(cat stderr_output.txt)
        echo "Parsing enlapsed time from commandline..."
        output2=$(cat stdout.txt)
        
        echo "Debug: Searching for cycle information in output..."
        
        
        # Extract cycle information from the output using a more robust approach
        verify_all_account_proofs=$(echo "$output" | grep "verify_all_account_proofs" | grep -o '^[0-9]\+')
        verify_contract_proof=$(echo "$output" | grep "verify contract merkle proof" | grep -o '^[0-9]\+')
        verify_all_storage_proofs=$(echo "$output" | grep "verify_all_storage_proofs" | grep -o '^[0-9]\+')
        verify_all_signatures=$(echo "$output" | grep "verify_all_signatures" | grep -o '^[0-9]\+')
        verify_all_nullifiers=$(echo "$output" | grep "verify_all_nullifiers" | grep -o '^[0-9]\+')
        total_cycles=$(echo "$output" | grep "Total Cycyles" | grep -o '^[0-9]\+') 
        accounts_count=$(echo "$output2" | grep "Accounts count" | grep -o '^[0-9]\+') 
        total_time=$(echo "$output2" | grep "Time elapsed" | grep -o '^[0-9]\+') 

        # Create JSON file with parsed metrics
        metrics_file="../../evaluation/inputs_validation_scaling/metrics_${EXENV}/valid_defi_inputs_receipt_${accounts}.json"
        mkdir -p "../../evaluation/inputs_validation_scaling/metrics_${EXENV}"
        # metrics_file="../../evaluation/inputs_validation_scaling/metrics_bonsai/valid_defi_inputs_receipt_${accounts}.json"
        # mkdir -p "../../evaluation/inputs_validation_scaling/metrics_bonsai"
#         
        cat > "$metrics_file" << EOF
{ 
  "guest_metrics":{
    "verify_all_account_proofs_cycles": ${verify_all_account_proofs:-0},
    "verify_contract_proof_cycles": ${verify_contract_proof:-0},
    "verify_all_storage_proofs_cycles": ${verify_all_storage_proofs:-0},
    "verify_all_signatures_cycles": ${verify_all_signatures:-0},
    "verify_all_nullifiers_cycles": ${verify_all_nullifiers:-0},
    "total_cycles": ${total_cycles:-0}
  },
    "proving_time": ${total_time:-0},
    "accounts_count": ${accounts_count}
}
EOF
        echo "✓ Parsed and saved metrics to $metrics_file"
        echo "  - verify_all_account_proofs: ${verify_all_account_proofs}"
        echo "  - verify_contract_proof: ${verify_contract_proof:-0}"
        echo "  - verify_all_storage_proofs: ${verify_all_storage_proofs:-0}"
        echo "  - verify_all_signatures: ${verify_all_signatures:-0}"
        echo "  - verify_all_nullifiers: ${verify_all_nullifiers:-0}"
        echo "  - total_cycles: ${total_cycles:-0}"
        echo "  - total_time in ms: ${total_time:-0}"
        echo "  - accounts_count: ${accounts_count:-0}"
        
    else
        echo "✗ Failed to generate proof for $accounts accounts"
        echo "Continuing with next account count..."
    fi
    rm stderr_output.txt
    rm stdout.txt
    
done

echo ""
echo "========================================="
echo "All proof generations completed!"
echo "========================================="
echo ""
echo "Generated outputs:"
for accounts in "${slice_lengths[@]}"; do
    receipt_path="../../score_publisher/host/receipts/${accounts}_accounts_proof/valid_defi_inputs_receipt_${accounts}.bin"
    if [[ -f "$receipt_path" ]]; then
        echo "✓ $accounts accounts: Receipt and metrics generated"
    else
        echo "✗ $accounts accounts: Generation failed or skipped"
    fi
done
