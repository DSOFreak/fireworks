#!/usr/bin/env bash
# Render the README demo GIFs.
#
# The show is deterministic for a given seed, so the animated frames are
# headless screenshots at fixed simulation times. The help panel needs real
# text, so it is captured from a live tmux session instead.
#
# Needs: cargo, ImageMagick, ffmpeg, tmux, python3 + Pillow.
set -euo pipefail

cd "$(dirname "$0")/.."
cargo build --release --quiet

FPS=10
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

# render <name> <size> <width> <seconds> <start> [fireworks args...]
render() {
    local name=$1 size=$2 width=$3 seconds=$4 start=$5
    shift 5
    local dir="$TMP/$name" frames
    mkdir -p "$dir"
    frames=$(python3 -c "print(round($seconds * $FPS))")
    for ((i = 0; i < frames; i++)); do
        local t n
        t=$(python3 -c "print(f'{$start + $i / $FPS:.4f}')")
        n=$(printf "%04d" "$i")
        ./target/release/fireworks --screenshot "$dir/$n.ppm" \
            --warmup "$t" --size "$size" --no-hint --no-dither "$@" >/dev/null
        magick "$dir/$n.ppm" -resize "${width}x" "$dir/$n.png"
        rm "$dir/$n.ppm"
    done
    encode "$name" "$dir" "$FPS"
}

# capture <name> <seconds> [fireworks args...] - frames from a live terminal
capture() {
    local name=$1 seconds=$2
    shift 2
    local dir="$TMP/$name" session="fireworks-gif"
    mkdir -p "$dir"
    tmux kill-session -t "$session" 2>/dev/null || true
    tmux new-session -d -s "$session" -x 110 -y 33 \
        "./target/release/fireworks $*"
    sleep 2
    tmux send-keys -t "$session" h
    tmux send-keys -t "$session" d
    sleep 0.3
    local start
    start=$(date +%s.%N)
    local frames
    frames=$(python3 -c "print(round($seconds * 12))")
    for ((i = 0; i < frames; i++)); do
        local n
        n=$(printf "%04d" "$i")
        tmux capture-pane -t "$session" -e -p > "$dir/$n.txt"
        python3 docs/ansi2png.py "$dir/$n.txt" "$dir/$n.png" 600
        rm "$dir/$n.txt"
        sleep 0.05
    done
    local elapsed
    elapsed=$(python3 -c "print(f'{$(date +%s.%N) - $start:.3f}')")
    tmux kill-session -t "$session" 2>/dev/null || true
    # Play back at the speed it was actually captured.
    local fps
    fps=$(python3 -c "print(f'{$frames / $elapsed:.2f}')")
    encode "$name" "$dir" "$fps"
}

encode() {
    local name=$1 dir=$2 fps=$3
    ffmpeg -y -framerate "$fps" -i "$dir/%04d.png" \
        -vf "palettegen=max_colors=96:stats_mode=diff" -frames:v 1 "$dir/pal.png" \
        -loglevel error
    ffmpeg -y -framerate "$fps" -i "$dir/%04d.png" -i "$dir/pal.png" \
        -lavfi "paletteuse=dither=none:diff_mode=none" -loop 0 \
        "docs/$name.gif" -loglevel error
    ls -lh "docs/$name.gif"
}

# One full message cycle (HAPPY + BIRTHDAY) for a seamless loop. 160 columns
# keeps both words on a single line.
render happy 160x48 600 14.03 1.2 --seed 5 --text "HAPPY BIRTHDAY"

# The ordinary show.
render show 100x30 600 10.0 12.0 --seed 3 --density 1.6

# The help and stats panels, captured from a real terminal for crisp text.
capture help 4.0 --seed 9 --density 1.2
