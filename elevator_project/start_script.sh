#!/usr/bin/env bash

# Default values
base_port=9000
release_flag=""
num_slaves=3
num_floors=4

while getopts "p:r:n:f:" opt; do
    case $opt in
        p) base_port=$OPTARG ;;
        r) release_flag="--release" ;;
        n) num_slaves=$OPTARG ;;
        f) num_floors=$OPTARG ;;
        *) echo "Ugyldig flagg"; exit 1 ;;
    esac
done

cargo build $release_flag 

sleep 0.5

# TMUX

# Check if tmux is installed
if ! command -v tmux &> /dev/null; then
    echo "tmux not found, installing..."
    if [ -x "$(command -v apt-get)" ]; then
        sudo apt-get update
        sudo apt-get install -y tmux
    else
        echo "Package manager not supported. Install tmux manually."
        exit 1
    fi
fi

# Ensure mouse mode is enabled
if ! grep -q "set -g mouse on" ~/.tmux.conf 2>/dev/null; then
    echo "set -g mouse on" >> ~/.tmux.conf
    echo "Mouse mode enabled in ~/.tmux.conf"
fi

# Reload tmux config if tmux is running
if pgrep tmux &> /dev/null; then
    tmux source-file ~/.tmux.conf
fi


# Start tmux session in detached mode
tmux new-session -s elevator -n main

# Run master_main in first pane
tmux send-keys -t elevator:main "cargo run $release_flag --bin master_main" C-m
sleep 0.5

# Split vertically to run backup_main
tmux split-window -v -t elevator:main
tmux send-keys "cargo run $release_flag --bin backup_main" C-m

# Split panes for each server/slave pair
for port in $(seq $base_port $((base_port + num_slaves - 1))); do
    # Split horizontally for SimElevatorServer
    tmux split-window -h -t elevator:main
    tmux select-layout -t elevator:main tiled
    tmux send-keys "../SimElevatorServer --port=$port" C-m

    # Split vertically for slave_main
    tmux split-window -v -t elevator:main
    tmux select-layout -t elevator:main tiled
    tmux send-keys "cargo run $release_flag --bin slave_main $port" C-m
    sleep 0.1
done

# Attach to the session
tmux attach -t elevator
