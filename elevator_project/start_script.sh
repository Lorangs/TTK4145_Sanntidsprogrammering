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

XPOS=0
YPOS_ROW1=0
YPOS_ROW2=600

gnome-terminal --geometry=80x24+$XPOS+$YPOS_ROW1 -- bash -c "cargo run $release_flag --bin master_main; exec bash"
sleep 0.5
gnome-terminal --geometry=80x24+$XPOS+$YPOS_ROW2 -- bash -c "cargo run $release_flag --bin backup_main; exec bash"

XPOS=$((XPOS + 600))

# Kjør tre SimElevatorServer-instanser på ulike porter
for port in $(seq $base_port $((base_port + num_slaves - 1))); do
        (gnome-terminal --geometry=80x24+$XPOS+$YPOS_ROW1 -- bash -c "../SimElevatorServer --port=$port; exec bash" &)
        (gnome-terminal --geometry=80x24+$XPOS+$YPOS_ROW2 -- bash -c "cargo run $release_flag --bin slave_main $port; exec bash" &)

        sleep 0.1
        XPOS=$((XPOS + 600))
done
