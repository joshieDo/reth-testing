#!/bin/bash

#######
# Runs different cryo commands over the two nodes checking for correctness, as well as comparing timings.
#
# There are two running modes:
# 1) Without passing BLOCK_START & BLOCK_END arguments:
#       It will assume the first node is a node that has filled its in-memory range and is no longer moving forward.
#       BLOCK_START will be (STORAGE_TIP - 1) and BLOCK_END will be the chain tip.
# 2) Passing BLOCK_START & BLOCK_END will assume nothing and just compare both nodes as described.
####

# Define node URLs and types. First node should be the in-memory one if applicable
NODES=("http://localhost:8545" "http://localhost:8544")
CRYO_TYPES=("logs" "blocks" "txs")

set -e # exit if a command fails

# Create a log file with a timestamp
log_file="cryo_script_$(date +%Y%m%d_%H%M%S).log"
echo "Logging all output to: $log_file"
exec > >(tee -a "$log_file") 2>&1

# Check if required commands are installed
for cmd in jq b3sum; do
    if ! command -v $cmd &> /dev/null; then
        echo "$cmd is not installed. Please install $cmd to proceed."
        exit 1
    fi
done

# Validate positional arguments
if [ "$#" -ne 0 ] && [ "$#" -ne 2 ]; then
    echo "Usage: $0 [BLOCK_START BLOCK_END]" | tee -a "$log_file"
    exit 1
fi

# Query tester_status from IN_MEM_NODE
get_tester_status() {
    curl -s -X POST -H "Content-Type: application/json" \
        --data '{"jsonrpc":"2.0","method":"tester_status","params":[],"id":1}' \
        "${NODES[0]}" | tee -a "$log_file"
}

# Get BLOCK_START and BLOCK_END from tester_status if not provided
if [ "$#" -eq 0 ]; then
    status=$(get_tester_status)
    ready=$(echo $status | jq -r '.result.ready' | tee -a "$log_file")
    if [ "$ready" != "true" ]; then
        echo "Node is not ready, exiting." | tee -a "$log_file"
        exit 1
    fi
    BLOCK_START=$(($(echo $status | jq -r '.result.in_memory_first') - 2))
    BLOCK_END=$(echo $status | jq -r '.result.tip')
else
    BLOCK_START=$1
    BLOCK_END=$2
fi

# Validate block range
if [ "$BLOCK_START" -gt "$BLOCK_END" ]; then
    echo "Invalid block range: BLOCK_START should be less than or equal to BLOCK_END." | tee -a "$log_file"
    exit 1
fi

# Start timing the entire script
script_start_time=$(date +%s)

# Display results header with block range
echo "===============================================" | tee -a "$log_file"
echo "           BLOCK_START: $BLOCK_START           " | tee -a "$log_file"
echo "           BLOCK_END: $BLOCK_END               " | tee -a "$log_file"
echo "===============================================" | tee -a "$log_file"

# Initialize timing and hash results
declare -A timing_results
declare -A b3sum_all
declare -A b3sum_combined

# Function to parse the real time from the time command's output
parse_time() {
    local time_output=$1
    echo "$time_output" | grep real | awk '{print $2}'
}

# Process each type for each node
process_type_node() {
    local TYPE=$1
    local NODE=$2
    local key="$TYPE@$NODE"

    # Display progress for cryo command
    echo ""
    echo "Processing $TYPE on $NODE..." | tee -a "$log_file"

    # Capture the time output using the time command
    echo "$ cryo $TYPE -b $BLOCK_START:$BLOCK_END --rpc $NODE" >> "$log_file"
    time_output=$( (time cryo $TYPE -b $BLOCK_START:$BLOCK_END --rpc $NODE) 2>&1 | tee -a "$log_file" )

    # Parse the real time from the time output
    cryo_duration=$(parse_time "$time_output")
    timing_results[$key]=$cryo_duration

    # Calculate and store b3sum hash
    if ls *.parquet 1> /dev/null 2>&1; then
        echo "Calculating b3sum for $TYPE from $NODE..." | tee -a "$log_file"
        all_hashes=$(b3sum *.parquet)
        full_hash=$(echo $all_hashes | b3sum | awk '{print $1}')
        short_hash="${full_hash:0:4}...${full_hash: -4}"
        b3sum_combined[$key]=$short_hash
        b3sum_all[$key]=$all_hashes
        rm -rf *.parquet
    else
        b3sum_combined[$key]="No parquet files generated"
    fi
}

# Iterate over all types and nodes
for TYPE in "${CRYO_TYPES[@]}"; do
    for NODE in "${NODES[@]}"; do
        process_type_node $TYPE $NODE
    done
done

# Display timing results in a table with better spacing
echo ""
echo "TIMINGS"
printf "%-10s %-25s %-25s\n" "Type/Node" "${NODES[0]}" "${NODES[1]}" | tee -a "$log_file"
for TYPE in "${CRYO_TYPES[@]}"; do
    printf "%-10s" "$TYPE"
    for NODE_INDEX in "${!NODES[@]}"; do
        NODE="${NODES[$NODE_INDEX]}"
        key="$TYPE@$NODE"
        time="${timing_results[$key]}"
        if [ $NODE_INDEX -eq 0 ]; then
            printf "%-25s" "$time"
        else
            FIRST_NODE_TIME="${timing_results[$TYPE@${NODES[0]}]}"
            if [[ "$time" > "$FIRST_NODE_TIME" ]]; then
                printf "%-25s" "$time ⬆️"
            elif [[ "$time" < "$FIRST_NODE_TIME" ]]; then
                printf "%-25s" "$time ⬇️"
            else
                printf "%-25s" "$time"
            fi
        fi
    done
    echo
done

# Display b3sum comparison results in a table
echo ""
echo "B3SUM"
printf "%-10s %-25s %-25s\n" "Type/Node" "${NODES[0]}" "${NODES[1]}"
for TYPE in "${CRYO_TYPES[@]}"; do
    printf "%-10s" "$TYPE"
    for NODE_INDEX in "${!NODES[@]}"; do
        NODE="${NODES[$NODE_INDEX]}"
        key="$TYPE@$NODE"
        if [ $NODE_INDEX -eq 0 ]; then
            printf "%-25s" "${b3sum_combined[$key]}"
        else
            FIRST_NODE_KEY="$TYPE@${NODES[0]}"
            if [ "${b3sum_combined[$key]}" == "${b3sum_combined[$FIRST_NODE_KEY]}" ]; then
                printf "%-25s" "✅"
            else
                printf "%-25s" "❌ ${b3sum_combined[$key]}"
            fi
        fi
    done
    echo
done

script_end_time=$(date +%s)
total_time=$((script_end_time - script_start_time))

echo ""
echo "==============================================="
echo "Script completed in $total_time seconds."
echo "Output: $log_file"
echo "==============================================="

# Output all b3sum results to the log file only, organized by node and type, one line per type and file
echo "" >> "$log_file"
echo "Detailed b3sum for each .parquet file:" >> "$log_file"
for NODE in "${NODES[@]}"; do
    num_files=$(echo "${b3sum_all[$key]}" | wc -w | tr -d ' ')
    echo "Node: $NODE | $num_files files" >> "$log_file"
    for TYPE in "${CRYO_TYPES[@]}"; do
        key="$TYPE@$NODE"
        if [ -n "${b3sum_all[$key]}" ]; then
            while IFS= read -r line; do
                full_hash=$(echo "$line" | awk '{print $1}')
                filename=$(echo "$line" | awk '{print $2}')
                short_hash="${full_hash:0:4}...${full_hash: -4}"
                echo "  $short_hash  $filename" >> "$log_file"
            done <<< "${b3sum_all[$key]}"
        fi
    done
    echo "" >> "$log_file"  # Add a blank line between nodes for better readability
done
