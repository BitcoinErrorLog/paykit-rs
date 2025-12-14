#!/bin/bash
# Android Emulator Setup Script
# Automatically starts an Android emulator for running instrumented tests

set -e

# Check ANDROID_HOME
if [ -z "$ANDROID_HOME" ]; then
    # Try common locations
    if [ -d "$HOME/Library/Android/sdk" ]; then
        export ANDROID_HOME="$HOME/Library/Android/sdk"
    elif [ -d "$HOME/Android/Sdk" ]; then
        export ANDROID_HOME="$HOME/Android/Sdk"
    else
        echo "Error: ANDROID_HOME not set and Android SDK not found in common locations"
        echo "Please set ANDROID_HOME to your Android SDK path"
        exit 1
    fi
fi

export PATH="$ANDROID_HOME/emulator:$ANDROID_HOME/platform-tools:$PATH"

# List available AVDs
echo "Available AVDs:"
$ANDROID_HOME/emulator/emulator -list-avds

# Get first available AVD or use provided one
AVD_NAME="${1:-$($ANDROID_HOME/emulator/emulator -list-avds | head -1)}"

if [ -z "$AVD_NAME" ]; then
    echo "Error: No AVDs found. Please create an AVD first:"
    echo "  $ANDROID_HOME/cmdline-tools/latest/bin/avdmanager create avd -n <name> -k <system-image>"
    exit 1
fi

echo "Starting emulator: $AVD_NAME"

# Start emulator in background
$ANDROID_HOME/emulator/emulator -avd "$AVD_NAME" -no-window -no-audio -no-snapshot-load > /tmp/emulator.log 2>&1 &
EMULATOR_PID=$!

echo "Emulator starting (PID: $EMULATOR_PID)"
echo "Waiting for device..."

# Wait for device to be detected
adb wait-for-device

# Wait for boot to complete
echo "Waiting for boot to complete..."
timeout=180
elapsed=0
while [ $elapsed -lt $timeout ]; do
    boot=$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r' || echo "0")
    if [ "$boot" = "1" ]; then
        echo "Emulator booted successfully"
        break
    fi
    echo "  Boot status: $boot ($elapsed/$timeout seconds)..."
    sleep 2
    elapsed=$((elapsed + 2))
done

if [ "$boot" != "1" ]; then
    echo "Warning: Emulator did not fully boot within timeout"
    exit 1
fi

# Wait for device to be online (not just detected)
echo "Waiting for device to be online..."
timeout=60
elapsed=0
while [ $elapsed -lt $timeout ]; do
    status=$(adb devices | grep emulator | awk '{print $2}' || echo "offline")
    if [ "$status" = "device" ]; then
        echo "Emulator is online and ready"
        adb devices
        exit 0
    fi
    echo "  Device status: $status ($elapsed/$timeout seconds)..."
    sleep 2
    elapsed=$((elapsed + 2))
done

if [ "$status" != "device" ]; then
    echo "Warning: Emulator did not come online within timeout"
    adb devices
    exit 1
fi
