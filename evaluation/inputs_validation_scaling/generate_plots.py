import json
import matplotlib.pyplot as plt
import numpy as np
import os
import glob
from pathlib import Path


def parse_metrics_files(base_path="./metrics_local"):
    """
    Parse all metrics files and extract account count vs proving time data
    """
    account_counts = []
    proving_times = []
    total_cycles = []
    all_metrics = []  # Store all metrics for breakdown chart
    
    # First, discover all available metrics files
    if not os.path.exists(base_path):
        print(f"Error: Metrics directory not found: {base_path}")
        return account_counts, proving_times, total_cycles, all_metrics
    
    # Find all metrics files matching the pattern
    pattern = os.path.join(base_path, "valid_defi_inputs_receipt_*.json")
    metrics_files = glob.glob(pattern)
    
    if not metrics_files:
        print(f"No metrics files found in {base_path}")
        print(f"Looking for pattern: valid_defi_inputs_receipt_*.json")
        return account_counts, proving_times, total_cycles, all_metrics
    
    print(f"Found {len(metrics_files)} metrics files:")
    for file in sorted(metrics_files):
        print(f"  - {os.path.basename(file)}")
    print()
    
    # Extract account counts from filenames and sort
    discovered_accounts = []
    for file in metrics_files:
        filename = os.path.basename(file)
        # Extract number from filename like "valid_defi_inputs_receipt_5.json"
        try:
            account_num = int(filename.split('_')[-1].replace('.json', '').replace('.bin', ''))
            discovered_accounts.append((account_num, file))
        except ValueError:
            print(f"Warning: Could not extract account count from {filename}")
    
    # Sort by account count
    discovered_accounts.sort(key=lambda x: x[0])
    
    for accounts, metrics_file in discovered_accounts:
        try:
            # Parse the JSON metrics file
            with open(metrics_file, 'r') as f:
                data = json.load(f)
            
            # Extract metrics
            if 'guest_metrics' in data:
                cycles = data['guest_metrics']['total_cycles']
                # Convert cycles to approximate time (assuming ~1GHz frequency)
                # You might need to adjust this conversion factor
                proving_time = data['proving_time'] / 1000  # Convert to seconds
                
                account_counts.append(accounts)
                proving_times.append(proving_time)
                total_cycles.append(cycles)
                all_metrics.append({"verify_all_account_proofs_cycles": data["guest_metrics"]["verify_all_account_proofs_cycles"] ,
                         "verify_contract_proof_cycles":data["guest_metrics"]["verify_contract_proof_cycles"],
                         "verify_all_storage_proofs_cycles":data["guest_metrics"]["verify_all_storage_proofs_cycles"],
                         "verify_all_signatures_cycles": data["guest_metrics"]["verify_all_signatures_cycles"],
                         "verify_all_nullifiers_cycles": data["guest_metrics"]["verify_all_nullifiers_cycles"],
                         "total_cycles": data["guest_metrics"]["total_cycles"],
                         "accounts_count": data["accounts_count"]})
            else:
                print(f"Warning: 'total_cycles' not found in {metrics_file}")
                print(f"  Available keys: {list(data.keys())}")
                
        except json.JSONDecodeError as e:
            print(f"Error parsing JSON in {metrics_file}: {e}")
        except Exception as e:
            print(f"Error reading {metrics_file}: {e}")
    
    return account_counts, proving_times, total_cycles, all_metrics

def create_basic_plots(account_counts, proving_times, total_cycles , proving_type):
    """
    Create the basic plots showing account count vs proving time and total cycles
    """
    if not account_counts:
        print("No data to plot!")
        return
    
    # Create figure with subplots
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(15, 6))
    
    # Plot 1: Account Count vs Proving Time
    ax1.plot(account_counts, proving_times, 'bo-', linewidth=2, markersize=8, label='Proving Time')
    ax1.set_xlabel('Number of Accounts', fontsize=12)
    ax1.set_ylabel('Proving Time (seconds)', fontsize=12)
    ax1.set_title(f'DeFi Proof Generation: Account Count vs Proving Time {proving_type}', fontsize=14, fontweight='bold')
    ax1.grid(True, alpha=0.3)
    ax1.legend()
    
    # Add value labels on points
    for i, (x, y) in enumerate(zip(account_counts, proving_times)):
        ax1.annotate(f'{y}s', (x, y), textcoords="offset points", xytext=(0,10), ha='center')
    
    # Plot 2: Account Count vs Total Cycles
    ax2.plot(account_counts, total_cycles, 'ro-', linewidth=2, markersize=8, label='Total Cycles')
    ax2.set_xlabel('Number of Accounts', fontsize=12)
    ax2.set_ylabel('Total Cycles', fontsize=12)
    ax2.set_title('DeFi Proof Generation: Account Count vs Total Cycles', fontsize=14, fontweight='bold')
    ax2.grid(True, alpha=0.3)
    ax2.legend()
    
    # Format y-axis to show cycles in millions/billions
    ax2.ticklabel_format(style='scientific', axis='y', scilimits=(0,0))
    
    # Add value labels on points
    for i, (x, y) in enumerate(zip(account_counts, total_cycles)):
        if y >= 1_000_000_000:
            label = f'{y/1_000_000_000:.2f}B'
        elif y >= 1_000_000:
            label = f'{y/1_000_000:.1f}M'
        else:
            label = f'{y:,}'
        ax2.annotate(label, (x, y), textcoords="offset points", xytext=(0,10), ha='center')
    
    plt.tight_layout()
    plt.savefig(f'defi_proof_metrics{proving_type}.png', dpi=300, bbox_inches='tight')
    plt.show()
    
    print(f'Basic metrics plots saved as defi_proof_metrics_{proving_type}.png')
def create_plots(account_counts, proving_times, total_cycles, all_metrics):
    """
    Create plots showing the relationship between account count and proving metrics
    """
    if not account_counts:
        print("No data to plot!")
        return
    
    # Create figure with multiple subplots
    fig = plt.figure(figsize=(20, 12))
    
    # Plot 1: Account Count vs Proving Time
    ax1 = plt.subplot(2, 2, 1)
    ax1.plot(account_counts, proving_times, 'bo-', linewidth=2, markersize=8, label='Proving Time')
    ax1.set_xlabel('Number of Accounts', fontsize=12)
    ax1.set_ylabel('Proving Time (seconds)', fontsize=12)
    ax1.set_title('DeFi Proof Generation: Account Count vs Proving Time', fontsize=14, fontweight='bold')
    ax1.grid(True, alpha=0.3)
    ax1.legend()
    
    # Add value labels on points
    for i, (x, y) in enumerate(zip(account_counts, proving_times)):
        ax1.annotate(f'{y}s', (x, y), textcoords="offset points", xytext=(0,10), ha='center')
    
    # Plot 2: Account Count vs Total Cycles
    ax2 = plt.subplot(2, 2, 2)
    ax2.plot(account_counts, total_cycles, 'ro-', linewidth=2, markersize=8, label='Total Cycles')
    ax2.set_xlabel('Number of Accounts', fontsize=12)
    ax2.set_ylabel('Total Cycles', fontsize=12)
    ax2.set_title('DeFi Proof Generation: Account Count vs Total Cycles', fontsize=14, fontweight='bold')
    ax2.grid(True, alpha=0.3)
    ax2.legend()
    
    # Format y-axis to show cycles in millions/billions
    ax2.ticklabel_format(style='scientific', axis='y', scilimits=(0,0))
    
    # Add value labels on points
    for i, (x, y) in enumerate(zip(account_counts, total_cycles)):
        if y >= 1_000_000_000:
            label = f'{y/1_000_000_000:.2f}'




def create_basic_plots(account_counts, proving_times, total_cycles , proving_type):
    """
    Create the basic plots showing account count vs proving time and total cycles
    """
    if not account_counts:
        print("No data to plot!")
        return
    
    # Create figure with subplots
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(15, 6))
    
    # Plot 1: Account Count vs Proving Time
    ax1.plot(account_counts, proving_times, 'bo-', linewidth=2, markersize=8, label='Proving Time')
    ax1.set_xlabel('Number of Accounts', fontsize=12)
    ax1.set_ylabel('Proving Time (seconds)', fontsize=12)
    ax1.set_title(f'DeFi Proof Generation: Account Count vs Proving Time {proving_type}', fontsize=14, fontweight='bold')
    ax1.grid(True, alpha=0.3)
    ax1.legend()
    
    # Add value labels on points
    for i, (x, y) in enumerate(zip(account_counts, proving_times)):
        ax1.annotate(f'{y}s', (x, y), textcoords="offset points", xytext=(0,10), ha='center')
    
    # Plot 2: Account Count vs Total Cycles
    ax2.plot(account_counts, total_cycles, 'ro-', linewidth=2, markersize=8, label='Total Cycles')
    ax2.set_xlabel('Number of Accounts', fontsize=12)
    ax2.set_ylabel('Total Cycles', fontsize=12)
    ax2.set_title(f'DeFi Proof Generation: Account Count vs Total Cycles {proving_type}', fontsize=14, fontweight='bold')
    ax2.grid(True, alpha=0.3)
    ax2.legend()
    
    # Format y-axis to show cycles in millions/billions
    ax2.ticklabel_format(style='scientific', axis='y', scilimits=(0,0))
    
    # Add value labels on points
    for i, (x, y) in enumerate(zip(account_counts, total_cycles)):
        if y >= 1_000_000_000:
            label = f'{y/1_000_000_000:.2f}B'
        elif y >= 1_000_000:
            label = f'{y/1_000_000:.1f}M'
        else:
            label = f'{y:,}'
        ax2.annotate(label, (x, y), textcoords="offset points", xytext=(0,10), ha='center')
    
    plt.tight_layout()
    plt.savefig(f'defi_proof_metrics_{proving_type}.png', dpi=300, bbox_inches='tight')
    plt.show()
    
    print("Basic metrics plots saved as 'defi_proof_metrics.png'")

def create_cycle_breakdown_chart(account_counts, all_metrics):
    """
    Create a stacked bar chart showing how cycle metrics sum up to total_cycles
    """
    print(all_metrics)
    if not account_counts or not all_metrics:
        print("No data for cycle breakdown chart!")
        return
    
    # Extract cycle components for each account count
    verify_account_proofs = []
    verify_contract_proof = []
    verify_storage_proofs = []
    verify_signatures = []
    verify_nullifiers = []
    
    for metrics in all_metrics:
        verify_account_proofs.append(metrics.get('verify_all_account_proofs_cycles', 0))
        verify_contract_proof.append(metrics.get('verify_contract_proof_cycles', 0))
        verify_storage_proofs.append(metrics.get('verify_all_storage_proofs_cycles', 0))
        verify_signatures.append(metrics.get('verify_all_signatures_cycles', 0))
        verify_nullifiers.append(metrics.get('verify_all_nullifiers_cycles', 0))
    
    # Create stacked bar chart
    fig, ax = plt.subplots(figsize=(14, 8))
    
    width = 0.6
    x = range(len(account_counts))
    
    # Create stacked bars
    p1 = ax.bar(x, verify_account_proofs, width, label='Account Proofs', color='#FF6B6B')
    p2 = ax.bar(x, verify_contract_proof, width, bottom=verify_account_proofs, 
                label='Contract Proof', color='#4ECDC4')
    p3 = ax.bar(x, verify_storage_proofs, width, 
                bottom=[i+j for i,j in zip(verify_account_proofs, verify_contract_proof)], 
                label='Storage Proofs', color='#45B7D1')
    p4 = ax.bar(x, verify_signatures, width,
                bottom=[i+j+k for i,j,k in zip(verify_account_proofs, verify_contract_proof, verify_storage_proofs)],
                label='Signatures', color='#F9CA24')
    p5 = ax.bar(x, verify_nullifiers, width,
                bottom=[i+j+k+l for i,j,k,l in zip(verify_account_proofs, verify_contract_proof, 
                                                   verify_storage_proofs, verify_signatures)],
                label='Nullifiers', color='#6C5CE7')
    
    # Customize the chart
    ax.set_xlabel('Number of Accounts', fontsize=12)
    ax.set_ylabel('Cycles', fontsize=12)
    ax.set_title('Cycle Breakdown by Component', fontsize=14, fontweight='bold')
    ax.set_xticks(x)
    ax.set_xticklabels(account_counts)
    ax.legend(loc='upper left')
    ax.grid(True, alpha=0.3, axis='y')
    
    # Format y-axis
    ax.ticklabel_format(style='scientific', axis='y', scilimits=(0,0))
    
    # Add total values on top of bars
    for i, metrics in enumerate(all_metrics):
        total = metrics.get('total_cycles', 0)
        if total >= 1_000_000_000:
            label = f'{total/1_000_000_000:.2f}B'
        elif total >= 1_000_000:
            label = f'{total/1_000_000:.1f}M'
        else:
            label = f'{total:,}'
        ax.text(i, total, label, ha='center', va='bottom', fontweight='bold')
    
    plt.tight_layout()
    plt.savefig('cycle_breakdown_chart.png', dpi=300, bbox_inches='tight')
    plt.show()
    
    print("Cycle breakdown chart saved as 'cycle_breakdown_chart.png'")

def create_cycles_vs_time_comparison(total_cycles, proving_times,bonsai_proving_times, account_counts):
    """
    Create a comparison plot of total_cycles vs proving time
    """
    if not total_cycles or not proving_times:
        print("No data for cycles vs time comparison!")
        return
    
    fig, (ax1,ax2) = plt.subplots(1, 2, figsize=(16, 6)) 
    
    # Plot 1: Scatter plot with trend line
    ax1.scatter(total_cycles, proving_times, c=account_counts, cmap='viridis', 
               s=100, alpha=0.7, edgecolors='black', linewidth=1)
    
    # Add trend line
    z = np.polyfit(total_cycles, proving_times, 1)
    p = np.poly1d(z)
    ax1.plot(total_cycles, p(total_cycles), "r--", alpha=0.8, linewidth=2, label=f'Trend: y={z[0]:.2e}x+{z[1]:.3f}')
    ax2.set_ylim(0, max(proving_times) * 1.1)
    
    ax1.set_xlabel('Total Cycles', fontsize=12)
    ax1.set_ylabel('Proving Time (seconds)', fontsize=12)
    ax1.set_title('Total Cycles vs Proving Time local', fontsize=14, fontweight='bold')
    ax1.grid(True, alpha=0.3)
    ax1.legend()
    
    # Format x-axis
    ax1.ticklabel_format(style='scientific', axis='x', scilimits=(0,0))
    
    # Add colorbar for account counts
    # cbar = plt.colorbar(ax1.collections[0], ax=ax1)
    # cbar.set_label('Number of Accounts', rotation=270, labelpad=15)
    
    # Annotate points with account counts
    for i, (x, y, accounts) in enumerate(zip(total_cycles, proving_times, account_counts)):
        ax1.annotate(f'{accounts}', (x, y), xytext=(5, 5), textcoords='offset points', 
                    fontsize=8, alpha=0.8)
    
    # Plot 2: Scatter plot with trend line
    ax2.scatter(total_cycles, bonsai_proving_times, c=account_counts, cmap='viridis', 
               s=100, alpha=0.7, edgecolors='black', linewidth=1)
    # Add trend line
    z = np.polyfit(total_cycles, bonsai_proving_times, 1)
    p = np.poly1d(z)
    ax2.plot(total_cycles, p(total_cycles), "r--", alpha=0.8, linewidth=2, label=f'Trend: y={z[0]:.2e}x+{z[1]:.3f}')
    ax2.set_ylim(0, max(bonsai_proving_times) * 1.1)

    ax2.set_xlabel('Total Cycles', fontsize=12)
    ax2.set_ylabel('Proving Time (seconds)', fontsize=12)
    ax2.set_title('Total Cycles vs Proving Time Bonsai', fontsize=14, fontweight='bold')
    ax2.grid(True, alpha=0.3)
    ax2.legend()
    
    # Format x-axis
    ax2.ticklabel_format(style='scientific', axis='x', scilimits=(0,0))
    
    # Add colorbar for account counts
    cbar = plt.colorbar(ax2.collections[0], ax=ax1)
    cbar.set_label('Number of Accounts', rotation=270, labelpad=15)
    
    # Annotate points with account counts
    for i, (x, y, accounts) in enumerate(zip(total_cycles, bonsai_proving_times, account_counts)):
        ax2.annotate(f'{accounts}', (x, y), xytext=(5, 5), textcoords='offset points', 
                    fontsize=8, alpha=0.8)
    
    
    plt.tight_layout()
    plt.savefig('cycles_vs_time_comparison.png', dpi=300, bbox_inches='tight')
    plt.show()
    
    print("Cycles vs time comparison saved as 'cycles_vs_time_comparison.png'")
    
    # Print correlation analysis
    correlation = np.corrcoef(total_cycles, proving_times)[0, 1]
    print(f"\nCorrelation between total cycles and proving time: {correlation:.4f}")
    if correlation > 0.9:
        print("Strong positive correlation - proving time scales linearly with cycles")
    elif correlation > 0.7:
        print("Moderate positive correlation - proving time generally increases with cycles")
    else:
        print("Weak correlation - other factors may influence proving time")

def create_local_vs_bonsai_comparison(local_account_counts, local_proving_times, 
                                     bonsai_account_counts, bonsai_proving_times):
    """
    Create a comparison plot showing proving times for local vs Bonsai execution
    """
    if not local_account_counts and not bonsai_account_counts:
        print("No data for local vs Bonsai comparison!")
        return
    
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(16, 6))
    
    # Plot 1: Direct comparison line plot
    if local_account_counts and local_proving_times:
        ax1.plot(local_account_counts, local_proving_times, 'bo-', linewidth=2, markersize=8, 
                label='Local ', alpha=0.8)
    
    if bonsai_account_counts and bonsai_proving_times:
        ax1.plot(bonsai_account_counts, bonsai_proving_times, 'ro-', linewidth=2, markersize=8, 
                label='Bonsai Service', alpha=0.8)
    
    ax1.set_xlabel('Number of Accounts', fontsize=12)
    ax1.set_ylabel('Proving Time (seconds)', fontsize=12)
    ax1.set_title('Proving Time Comparison: Local vs Bonsai', fontsize=14, fontweight='bold')
    ax1.grid(True, alpha=0.3)
    ax1.legend()
    ax1.set_yscale('log')  # Use log scale for better visualization if times vary greatly
    
    # Add value labels on points
    if local_account_counts and local_proving_times:
        for i, (x, y) in enumerate(zip(local_account_counts, local_proving_times)):
            ax1.annotate(f'{y}s', (x, y), textcoords="offset points", xytext=(0,10), 
                        ha='center', color='blue', fontsize=8)
    
    if bonsai_account_counts and bonsai_proving_times:
        for i, (x, y) in enumerate(zip(bonsai_account_counts, bonsai_proving_times)):
            ax1.annotate(f'{y}s', (x, y), textcoords="offset points", xytext=(0,-15), 
                        ha='center', color='red', fontsize=8)
    
    # Plot 2: Side-by-side bar comparison for common account counts
    common_accounts = set(local_account_counts) & set(bonsai_account_counts)
    if common_accounts:
        common_accounts = sorted(list(common_accounts))
        
        local_times_common = []
        bonsai_times_common = []
        
        for acc in common_accounts:
            if acc in local_account_counts:
                idx = local_account_counts.index(acc)
                local_times_common.append(local_proving_times[idx])
            else:
                local_times_common.append(0)
                
            if acc in bonsai_account_counts:
                idx = bonsai_account_counts.index(acc)
                bonsai_times_common.append(bonsai_proving_times[idx])
            else:
                bonsai_times_common.append(0)
        
        x = np.arange(len(common_accounts))
        width = 0.35
        
        bars1 = ax2.bar(x - width/2, local_times_common, width, label='Local ', 
                       color='lightblue', alpha=0.8, edgecolor='blue')
        bars2 = ax2.bar(x + width/2, bonsai_times_common, width, label='Bonsai Service', 
                       color='lightcoral', alpha=0.8, edgecolor='red')
        
        ax2.set_xlabel('Number of Accounts', fontsize=12)
        ax2.set_ylabel('Proving Time (ms)', fontsize=12)
        ax2.set_title('Side-by-Side Proving Time Comparison', fontsize=14, fontweight='bold')
        ax2.set_xticks(x)
        ax2.set_xticklabels(common_accounts)
        ax2.legend()
        ax2.grid(True, alpha=0.3, axis='y')
        
        # Add value labels on bars
        for bar, time in zip(bars1, local_times_common):
            if time > 0:
                ax2.text(bar.get_x() + bar.get_width()/2, bar.get_height(), 
                        f'{time}s', ha='center', va='bottom', fontsize=8, rotation=90)
        
        for bar, time in zip(bars2, bonsai_times_common):
            if time > 0:
                ax2.text(bar.get_x() + bar.get_width()/2, bar.get_height(), 
                        f'{time}s', ha='center', va='bottom', fontsize=8, rotation=90)
    else:
        ax2.text(0.5, 0.5, 'No common account counts\nfor comparison', 
                ha='center', va='center', transform=ax2.transAxes, fontsize=12)
        ax2.set_title('No Common Data Points', fontsize=14)
    
    plt.tight_layout()
    plt.savefig('local_vs_bonsai_comparison.png', dpi=300, bbox_inches='tight')
    plt.show()
    
    print("Local vs Bonsai comparison saved as 'local_vs_bonsai_comparison.png'")
    
    # Print comparison statistics
    if common_accounts:
        print(f"\nComparison Statistics for {len(common_accounts)} common account counts:")
        print("="*60)
        for i, acc in enumerate(common_accounts):
            local_time = local_times_common[i]
            bonsai_time = bonsai_times_common[i]
            if local_time > 0 and bonsai_time > 0:
                speedup = bonsai_time / local_time
                print(f"{acc} accounts: Local={local_time}s, Bonsai={bonsai_time}s, "
                      f"Ratio={speedup:.2f}x {'(Bonsai slower)' if speedup > 1 else '(Bonsai faster)'}")
    else:
        print("\nNo common account counts found for direct comparison.")

def print_summary_statistics(account_counts, proving_times, total_cycles):
    """
    Print summary statistics about the performance data
    """
    print("\n" + "="*50)
    print("SUMMARY STATISTICS")
    print("="*50)
    print(f"Account counts tested: {account_counts}")
    print(f"Proving time range: {min(proving_times)}s - {max(proving_times)}s")
    print(f"Cycle count range: {min(total_cycles):,} - {max(total_cycles):,}")
    
    if len(proving_times) > 1:
        # Calculate scaling factors
        time_ratio = max(proving_times) / min(proving_times)
        account_ratio = max(account_counts) / min(account_counts)
        cycle_ratio = max(total_cycles) / min(total_cycles)
        
        print(f"Time scaling factor: {time_ratio:.2f}x for {account_ratio:.1f}x more accounts")
        print(f"Cycle scaling factor: {cycle_ratio:.2f}x for {account_ratio:.1f}x more accounts")
        
        # Calculate average efficiency
        avg_cycles_per_account = [cycles/accounts for cycles, accounts in zip(total_cycles, account_counts)]
        print(f"Average cycles per account: {np.mean(avg_cycles_per_account):,.0f}")

def main():
    print("DeFi Proof Metrics Analyzer")
    print("="*40)
    print(f"Current working directory: {os.getcwd()}")
    print(f"Looking for local metrics in: ./metrics_local/")
    print(f"Looking for Bonsai metrics in: ./metrics_bonsai/")
    print()
    
    # Parse local metrics files
    account_counts, proving_times, total_cycles, all_metrics = parse_metrics_files("./metrics_local")
    
    # Parse Bonsai metrics files
    bonsai_account_counts, bonsai_proving_times, bonsai_total_cycles ,_= parse_metrics_files("./metrics_bonsai")
    
    if not account_counts and not bonsai_account_counts:
        print("\nNo metrics files found! Make sure you have run the proof generation.")
        print("Expected directory structures:")
        print("  ./metrics/valid_defi_inputs_receipt_[N].json")
        print("  ./metrics_bonsai/valid_defi_inputs_receipt_[N].json")
        return
    
    # Create all plots
    print("\nGenerating plots...")
    
    # 1. Basic performance plots (if local data available)
    if account_counts:
        create_basic_plots(account_counts, proving_times, total_cycles, "")
    # if bonsai_account_counts:
    #     create_basic_plots(bonsai_account_counts, bonsai_proving_times, bonsai_total_cycles,"xd")
    
    # 2. Cycle breakdown chart (if local data available)
    create_cycle_breakdown_chart(account_counts, all_metrics)
    
    # 3. Cycles vs time comparison (if local data available)
    if account_counts:
        create_cycles_vs_time_comparison(total_cycles, proving_times, bonsai_proving_times, account_counts)
    
    # 4. Local vs Bonsai comparison (if both datasets available)
    if account_counts or bonsai_account_counts:
        create_local_vs_bonsai_comparison(account_counts, proving_times, 
                                        bonsai_account_counts, bonsai_proving_times)
    
    # 5. Print summary statistics
    if account_counts:
        print_summary_statistics(account_counts, proving_times, total_cycles)
    
    if bonsai_account_counts:
        print("\n" + "="*50)
        print("BONSAI SUMMARY STATISTICS")
        print("="*50)
        print(f"Account counts tested: {bonsai_account_counts}")
        print(f"Proving time range: {min(bonsai_proving_times)}s - {max(bonsai_proving_times)}s")
        print(f"Cycle count range: {min(bonsai_total_cycles):,} - {max(bonsai_total_cycles):,}")
    
    print(f"\nAll plots generated successfully!")
    generated_files = []
    if account_counts:
        generated_files.extend([
            "  - defi_proof_metrics.png (basic performance plots)",
            "  - cycle_breakdown_chart.png (component breakdown)",
            "  - cycles_vs_time_comparison.png (cycles vs time analysis)"
        ])
    if account_counts or bonsai_account_counts:
        generated_files.append("  - local_vs_bonsai_comparison.png (local vs Bonsai comparison)")
    
    if generated_files:
        print("Generated files:")
        for file in generated_files:
            print(file)

if __name__ == "__main__":
    main()
