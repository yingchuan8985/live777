#!/usr/bin/env -S just --justfile

host := "127.0.0.1"
port := "7777"
server := "http://" + host + ":" + port
stream := "test-stream"

isdp := "i.sdp"
osdp := "o.sdp"

asrc := "-f lavfi -i sine=frequency=1000"
vsrc := "-f lavfi -i testsrc=size=640x480:rate=30"

h264 := "libx264 -preset ultrafast -tune zerolatency -profile:v baseline -level 3.0 -pix_fmt yuv420p -g 30 -keyint_min 30 -b:v 1000k -minrate 1000k -maxrate 1000k -bufsize 1000k"
h265 := "libx265 -preset ultrafast -tune zerolatency -x265-params keyint=30:min-keyint=30:bframes=0:repeat-headers=1 -pix_fmt yuv420p -b:v 1000k -minrate 1000k -maxrate 1000k -bufsize 1000k"
vp9  := "libvpx-vp9 -pix_fmt yuv420p"

gst_hd := "video/x-raw,format=I420,width=1280,height=720,framerate=30/1"

gst_x264 := "x264enc tune=zerolatency speed-preset=ultrafast key-int-max=60 byte-stream=true"
gst_x265 := "x265enc tune=zerolatency speed-preset=ultrafast key-int-max=60 qp=23"
gst_vp8 := "vp8enc deadline=1 cpu-used=6 lag-in-frames=0 end-usage=cbr keyframe-max-dist=60"
gst_vp9 := "vp9enc deadline=1 cpu-used=6 lag-in-frames=0 end-usage=cbr keyframe-max-dist=60 row-mt=1"
gst_av1 := "av1enc usage-profile=realtime"

default:
    just --list

build:
    pnpm install
    pnpm run build
    cargo build --release --all-targets --all-features

# MacOS:
#   brew install gstreamer
# Debian:
#   apt install libgstreamer1.0-dev libgstrtspserver-1.0-dev
#
# Build some tools: test-rtsp-server
build-tools:
    gcc -o test-rtsp-server tools/test-rtsp-server.c $(pkg-config --cflags --libs gstreamer-1.0 gstreamer-rtsp-server-1.0)

docs:
    pnpm run docs:dev

run:
    cargo run --features=webui

run-cluster:
    cargo run --bin=livenil --features=webui -- -c conf/livenil

only-mpeg-rtp-h264:
    ffmpeg -re {{vsrc}} -vcodec {{h264}} -f rtp 'rtp://{{host}}:5002?pkt_size=1200' -sdp_file {{isdp}}

[group('gst-rtsp-server')]
gst-rtsp-server-h264:
    ./test-rtsp-server "( videotestsrc is-live=true ! {{gst_hd}} ! {{gst_x264}} ! h264parse ! rtph264pay name=pay0 pt=96 )"

[group('gst-rtsp-server')]
gst-rtsp-server-h265:
    ./test-rtsp-server "( videotestsrc is-live=true ! {{gst_hd}} ! {{gst_x265}} ! h265parse ! rtph265pay name=pay0 pt=96 )"

[group('gst-rtsp-server')]
gst-rtsp-server-vp8:
    ./test-rtsp-server "( videotestsrc is-live=true ! {{gst_hd}} ! {{gst_vp8}} ! rtpvp8pay name=pay0 pt=96 )"

[group('gst-rtsp-server')]
gst-rtsp-server-vp9:
    ./test-rtsp-server "( videotestsrc is-live=true ! {{gst_hd}} ! {{gst_vp9}} ! vp9parse ! rtpvp9pay name=pay0 pt=96 )"

[group('gst-rtsp-server')]
gst-rtsp-server-av1:
    ./test-rtsp-server "( videotestsrc is-live=true ! {{gst_hd}} ! {{gst_av1}} ! av1parse ! rtpav1pay name=pay0 pt=96 )"

[group('gst-rtsp-server')]
gst-rtsp-server-opus:
    ./test-rtsp-server "( audiotestsrc is-live=true ! opusenc ! rtpopuspay name=pay0 pt=96 )"

[group('gst-rtsp-server')]
gst-rtsp-server-g722:
    ./test-rtsp-server "( audiotestsrc is-live=true ! avenc_g722 ! rtpg722pay name=pay0 pt=96 )"

[group('gst-rtsp-server')]
gst-rtsp-server-both-h264-opus:
    ./test-rtsp-server "( videotestsrc is-live=true ! {{gst_x264}} ! rtph264pay name=pay0 pt=96 audiotestsrc is-live=true ! opusenc ! rtpopuspay name=pay1 pt=97 )"

[group('gst-rtsp-server')]
whip-rtsp:
    cargo run --bin=whipinto -- -i rtsp://{{host}}:8554/test -w {{server}}/whip/{{stream}}

[group('gst-rtsp-server')]
whip-rtp:
    cargo run --bin=whipinto -- -i {{isdp}} -w {{server}}/whip/{{stream}}

[group('simple-rtp')]
ffmpeg-rtp-h264:
    cargo run --bin=whipinto -- -i {{isdp}} -w {{server}}/whip/{{stream}} --command \
        "ffmpeg -re {{vsrc}} -vcodec {{h264}} -f rtp 'rtp://{{host}}:5002' -sdp_file {{isdp}}"

[group('simple-rtp')]
ffmpeg-rtp-h265:
    cargo run --bin=whipinto -- -i {{isdp}} -w {{server}}/whip/{{stream}} --command \
        "ffmpeg -re {{vsrc}} -vcodec {{h265}} -f rtp 'rtp://{{host}}:5002' -sdp_file {{isdp}}"

[group('simple-rtp')]
ffmpeg-rtp-vp8:
    cargo run --bin=whipinto -- -i {{isdp}} -w {{server}}/whip/{{stream}} --command \
        "ffmpeg -re {{vsrc}} -vcodec libvpx -f rtp rtp://{{host}}:5002 -sdp_file {{isdp}}"

# 4K (3840×2160)
[group('simple-rtp')]
ffmpeg-rtp-4k:
    cargo run --bin=whipinto -- -i {{isdp}} -w {{server}}/whip/{{stream}} --command \
        "ffmpeg -re -f lavfi -i testsrc=size=3840x2160:rate=30 -strict experimental -vcodec {{vp9}} -f rtp rtp://{{host}}:5002 -sdp_file {{isdp}}"

[group('simple-rtp')]
ffmpeg-rtp-opus:
    cargo run --bin=whipinto -- -i {{isdp}} -w {{server}}/whip/{{stream}} --command \
        "ffmpeg -re {{asrc}} -acodec libopus -f rtp rtp://{{host}}:5002 -sdp_file {{isdp}}"

[group('simple-rtp')]
ffmpeg-rtp-g722:
    cargo run --bin=whipinto -- -i {{isdp}} -w {{server}}/whip/{{stream}} --command \
        "ffmpeg -re {{asrc}} -acodec g722 -f rtp rtp://{{host}}:5002?pkt_size=1200 -sdp_file {{isdp}}"

[group('simple-rtp')]
ffmpeg-rtp-vp8-opus:
    cargo run --bin=whipinto -- -i {{isdp}} -w {{server}}/whip/{{stream}} --command \
        "ffmpeg -re {{asrc}} {{vsrc}} -acodec libopus -vn -f rtp rtp://{{host}}:5002 -vcodec libvpx -an -f rtp rtp://{{host}}:5004 -sdp_file {{isdp}}"

[group('simple-rtp')]
ffplay-rtp:
    cargo run --bin=whepfrom -- -o "rtp://localhost?video=9000&audio=9002" --sdp-file {{osdp}} -w {{server}}/whep/{{stream}} --command \
        "ffplay -protocol_whitelist rtp,file,udp -i {{osdp}}"


# Aa rtsp server receive stream
[group('simple-rtsp')]
ffmpeg-rtsp:
    cargo run --bin=whipinto -- -i rtsp-listen://{{host}}:8550 -w {{server}}/whip/{{stream}} --command \
        "ffmpeg -re {{asrc}} {{vsrc}} -acodec libopus -vcodec libvpx -f rtsp rtsp://{{host}}:8550"

[group('simple-rtsp')]
ffmpeg-rtsp-tcp:
    cargo run --bin=whipinto -- -i rtsp-listen://{{host}}:8550 -w {{server}}/whip/{{stream}} --command \
        "ffmpeg -re {{asrc}} {{vsrc}} -acodec libopus -vcodec libvpx -rtsp_transport tcp -f rtsp rtsp://{{host}}:8550"

[group('simple-rtsp')]
ffmpeg-rtsp-vp9:
    cargo run --bin=whipinto -- -i rtsp-listen://{{host}}:8550 -w {{server}}/whip/{{stream}} --command \
        "ffmpeg -re {{asrc}} {{vsrc}} -acodec libopus -strict experimental -vcodec {{vp9}} -f rtsp rtsp://{{host}}:8550"

[group('simple-rtsp')]
ffmpeg-rtsp-h264:
    cargo run --bin=whipinto -- -i rtsp-listen://{{host}}:8550 -w {{server}}/whip/{{stream}} --command \
        "ffmpeg -re {{vsrc}} -vcodec {{h264}} -f rtsp rtsp://{{host}}:8550"

ffmpeg-rtsp-h264-raw:
    cargo run --bin=whipinto -- -i rtsp-listen://{{host}}:8550 -w {{server}}/whip/{{stream}} --command \
        "ffmpeg -re {{vsrc}} -vcodec libx264 -f rtsp rtsp://{{host}}:8550"

[group('simple-rtsp')]
ffmpeg-rtsp-h265:
    cargo run --bin=whipinto -- -i rtsp-listen://{{host}}:8550 -w {{server}}/whip/{{stream}} --command \
        "ffmpeg -re {{vsrc}} -vcodec {{h265}} -f rtsp rtsp://{{host}}:8550"

[group('simple-rtsp')]
ffplay-rtsp:
    cargo run --bin=whepfrom -- -o rtsp-listen://{{host}}:8650 -w {{server}}/whep/{{stream}} --command \
        "ffplay rtsp://{{host}}:8650"

[group('simple-rtsp')]
ffplay-rtsp-tcp:
    cargo run --bin=whepfrom -- -o rtsp-listen://{{host}}:8650 -w {{server}}/whep/{{stream}} --command \
        "ffplay rtsp://{{host}}:8650 -rtsp_transport tcp"


[group('cycle-rtsp')]
cycle-rtsp-0a:
    cargo run --bin=whipinto -- -i rtsp-listen://{{host}}:8550 -w {{server}}/whip/cycle-rtsp-a --command \
        "ffmpeg -re {{asrc}} {{vsrc}} -acodec libopus -vcodec libvpx -f rtsp rtsp://{{host}}:8550"

[group('cycle-rtsp')]
cycle-rtsp-1a:
    cargo run --bin=whepfrom -- -o rtsp-listen://{{host}}:8650 -w {{server}}/whep/cycle-rtsp-a

[group('cycle-rtsp')]
cycle-rtsp-2b:
    cargo run --bin=whipinto -- -i rtsp://{{host}}:8650 -w {{server}}/whip/cycle-rtsp-b

[group('cycle-rtsp')]
cycle-rtsp-3c:
    cargo run --bin=whipinto -- -i rtsp-listen://{{host}}:8750 -w {{server}}/whip/cycle-rtsp-c

[group('cycle-rtsp')]
cycle-rtsp-4b:
    cargo run --bin=whepfrom -- -o rtsp://{{host}}:8750 -w {{server}}/whep/cycle-rtsp-b

[group('cycle-rtsp')]
cycle-rtsp-5c:
    cargo run --bin=whepfrom -- -o rtsp-listen://{{host}}:8850 -w {{server}}/whep/cycle-rtsp-c --command \
        "ffplay rtsp://{{host}}:8850"

