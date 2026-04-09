# VLC

VLC RTP stream

**注意：VLC 无法支持所有视频编解码器**

```
vlc -> whipinto -> live777 -> whepfrom -> vlc
```

## Video: VP8

生成测试视频

```bash
ffmpeg -f lavfi -i testsrc=size=640x480:rate=30:d=30 \
-c:v libvpx output.webm
```

使用该视频发送 rtp

```bash
vlc -vvv output.webm --loop --sout '#rtp{dst=127.0.0.1,port=5003}'
```

```bash
cat > stream.sdp << EOF
v=0
m=video 5004 RTP/AVP 96
c=IN IP4 127.0.0.1
a=rtpmap:96 VP8/90000
EOF
```

使用 VLC player

```bash
vlc stream.sdp
```

## Video: H264

```bash
ffmpeg -f lavfi -i testsrc=size=640x480:rate=30:d=30 \
-c:v libx264 \
-x264-params "level-asymmetry-allowed=1:packetization-mode=1:profile-level-id=42001f" \
output.mp4
```

使用该视频发送 rtp

```bash
vlc -vvv output.mp4 --loop --sout '#rtp{dst=127.0.0.1,port=5003}'
```

```bash
cat > stream.sdp << EOF
v=0
c=IN IP4 127.0.0.1
a=recvonly
a=type:broadcast
a=charset:UTF-8
m=video 5003 RTP/AVP 96
b=AS:43
b=RR:0
a=rtpmap:96 H264/90000
a=fmtp:96 packetization-mode=1;profile-level-id=f4001e;sprop-parameter-sets=Z/QAHpGbKBQHtgIgAAADACAAAAeB4sWywA==,aOvjxEhE;
a=rtcp:5004
EOF
```

```bash
vlc stream.sdp
```

## Video: H265

```bash
ffmpeg -f lavfi -i testsrc=size=640x480:rate=30:d=30 \
-c:v libx265 \
-x265-params "level-idc=31:profile=main:repeat-headers=1" \
-pix_fmt yuv420p \
output.mp4
```

use this video send rtp

```bash
vlc -vvv output.mp4 --loop --sout '#rtp{dst=127.0.0.1,port=5003}'
```

Need VLC log `[000000015b706610] stream_out_rtp stream out debug: sdp=`

```bash
cat > stream.sdp << EOF
v=0
o=- 17102127568037963085 17102127568037963085 IN IP4 Samael.local
s=Unnamed
i=N/A
c=IN IP4 127.0.0.1
t=0 0
a=tool:vlc 3.0.21
a=recvonly
a=type:broadcast
a=charset:UTF-8
m=video 5003 RTP/AVP 96
b=RR:0
a=rtpmap:96 H265/90000
a=fmtp:96 tx-mode=SRST;profile-id=1;level-id=93;tier-flag=0;profile-space=0;sprop-vps=QAEMAf//AWAAAAMAkAAAAwAAAwBdlZgJ;sprop-sps=QgEBAWAAAAMAkAAAAwAAAwBdoAUCAeFllZpJMrwFoCAAAAMAIAAAAwPB;sprop-pps=RAHBcrRiQA==;sprop-sei=TgEF////////////eSyi3gm1F0fbu1Wk/n/C/E54MjY1IChidWlsZCAyMTUpIC0gNC4xKzEtMWQxMTdiZTpbTWFjIE9TIFhdW2NsYW5nIDE2LjAuMF1bNjQgYml0XSA4Yml0KzEwYml0KzEyYml0IC0gSC4yNjUvSEVWQyBjb2RlYyAtIENvcHlyaWdodCAyMDEzLTIwMTggKGMpIE11bHRpY29yZXdhcmUsIEluYyAtIGh0dHA6Ly94MjY1Lm9yZyAtIG9wdGlvbnM6IGNwdWlkPTk4IGZyYW1lLXRocmVhZHM9MyB3cHAgbm8tcG1vZGUgbm8tcG1lIG5vLXBzbnIgbm8tc3NpbSBsb2ctbGV2ZWw9MiBiaXRkZXB0aD04IGlucHV0LWNzcD0xIGZwcz0zMC8xIGlucHV0LXJlcz02NDB4NDgwIGludGVybGFjZT0wIHRvdGFsLWZyYW1lcz0wIGxldmVsLWlkYz0zMSBoaWdoLXRpZXI9MSB1aGQtYmQ9MCByZWY9MyBuby1hbGxvdy1ub24tY29uZm9ybWFuY2UgcmVwZWF0LWhlYWRlcnMgYW5uZXhiIG5vLWF1ZCBuby1lb2Igbm8tZW9zIG5vLWhyZCBpbmZvIGhhc2g9MCB0ZW1wb3JhbC1sYXllcnM9MCBvcGVuLWdvcCBtaW4ta2V5aW50PTI1IGtleWludD0yNTAgZ29wLWxvb2thaGVhZD0wIGJmcmFtZXM9NCBiLWFkYXB0PTIgYi1weXJhbWlkIGJmcmFtZS1iaWFzPTAgcmMtbG9va2FoZWFkPTIwIGxvb2thaGVhZC1zbGljZXM9MCBzY2VuZWN1dD00MCBuby1oaXN0LXNjZW5lY3V0IHJhZGw9MCBuby1zcGxpY2Ugbm8taW50cmEtcmVmcmVzaCBjdHU9NjQgbWluLWN1LXNpemU9OCBuby1yZWN0IG5vLWFtcCBtYXgtdHUtc2l6ZT0zMiB0dS1pbnRlci1kZXB0aD0xIHR1LWludHJhLWRlcHRoPTEgbGltaXQtdHU9MCByZG9xLWxldmVsPTAgZHluYW1pYy1yZD0wLjAwIG5vLXNzaW0tcmQgc2lnbmhpZGUgbm8tdHNraXAgbnItaW50cmE9MCBuci1pbnRlcj0wIG5vLWNvbnN0cmFpbmVkLWludHJhIHN0cm9uZy1pbnRyYS1zbW9vdGhpbmcgbWF4LW1lcmdlPTMgbGltaXQtcmVmcz0xIG5vLWxpbWl0LW1vZGVzIG1lPTEgc3VibWU9MiBtZXJhbmdlPTU3IHRlbXBvcmFsLW12cCBuby1mcmFtZS1kdXAgbm8taG1lIHdlaWdodHAgbm8td2VpZ2h0YiBuby1hbmFseXplLXNyYy1waWNzIGRlYmxvY2s9MDowIHNhbyBuby1zYW8tbm9uLWRlYmxvY2sgcmQ9MyBzZWxlY3RpdmUtc2FvPTQgZWFybHktc2tpcCByc2tpcCBuby1mYXN0LWludHJhIG5vLXRza2lwLWZhc3Qgbm8tY3UtbG9zc2xlc3MgYi1pbnRyYSBuby1zcGxpdHJkLXNraXAgcmRwZW5hbHR5PTAgcHN5LXJkPTIuMDAgcHN5LXJkb3E9MC4wMCBuby1yZC1yZWZpbmUgbm8tbG9zc2xlc3MgY2JxcG9mZnM9MCBjcnFwb2Zmcz0wIHJjPWNyZiBjcmY9MjguMCBxY29tcD0wLjYwIHFwc3RlcD00IHN0YXRzLXdyaXRlPTAgc3RhdHMtcmVhZD0wIHZidi1tYXhyYXRlPTEwMDAwIHZidi1idWZzaXplPTEwMDAwIHZidi1pbml0PTAuOSBtaW4tdmJ2LWZ1bGxuZXNzPTUwLjAgbWF4LXZidi1mdWxsbmVzcz04MC4wIGNyZi1tYXg9MC4wIGNyZi1taW49MC4wIGlwcmF0aW89MS40MCBwYnJhdGlvPTEuMzAgYXEtbW9kZT0yIGFxLXN0cmVuZ3RoPTEuMDAgY3V0cmVlIHpvbmUtY291bnQ9MCBuby1zdHJpY3QtY2JyIHFnLXNpemU9MzIgbm8tcmMtZ3JhaW4gcXBtYXg9NjkgcXBtaW49MCBuby1jb25zdC12YnYgc2FyPTEgb3ZlcnNjYW49MCB2aWRlb2Zvcm1hdD01IHJhbmdlPTAgY29sb3JwcmltPTIgdHJhbnNmZXI9MiBjb2xvcm1hdHJpeD0yIGNocm9tYWxvYz0wIGRpc3BsYXktd2luZG93PTAgY2xsPTAsMCBtaW4tbHVtYT0wIG1heC1sdW1hPTI1NSBsb2cyLW1heC1wb2MtbHNiPTggdnVpLXRpbWluZy1pbmZvIHZ1aS1ocmQtaW5mbyBzbGljZXM9MSBuby1vcHQtcXAtcHBzIG5vLW9wdC1yZWYtbGlzdC1sZW5ndGgtcHBzIG5vLW11bHRpLXBhc3Mtb3B0LXJwcyBzY2VuZWN1dC1iaWFzPTAuMDUgbm8tb3B0LWN1LWRlbHRhLXFwIG5vLWFxLW1vdGlvbiBuby1oZHIxMCBuby1oZHIxMC1vcHQgbm8tZGhkcjEwLW9wdCBuby1pZHItcmVjb3Zlcnktc2VpIGFuYWx5c2lzLXJldXNlLWxldmVsPTAgYW5hbHlzaXMtc2F2ZS1yZXVzZS1sZXZlbD0wIGFuYWx5c2lzLWxvYWQtcmV1c2UtbGV2ZWw9MCBzY2FsZS1mYWN0b3I9MCByZWZpbmUtaW50cmE9MCByZWZpbmUtaW50ZXI9MCByZWZpbmUtbXY9MSByZWZpbmUtY3R1LWRpc3RvcnRpb249MCBuby1saW1pdC1zYW8gY3R1LWluZm89MCBuby1sb3dwYXNzLWRjdCByZWZpbmUtYW5hbHlzaXMtdHlwZT0wIGNvcHktcGljPTEgbWF4LWF1c2l6ZS1mYWN0b3I9MS4wIG5vLWR5bmFtaWMtcmVmaW5lIG5vLXNpbmdsZS1zZWkgbm8taGV2Yy1hcSBuby1zdnQgbm8tZmllbGQgcXAtYWRhcHRhdGlvbi1yYW5nZT0xLjAwIHNjZW5lY3V0LWF3YXJlLXFwPTBjb25mb3JtYW5jZS13aW5kb3ctb2Zmc2V0cyByaWdodD0wIGJvdHRvbT0wIGRlY29kZXItbWF4LXJhdGU9MCBuby12YnYtbGl2ZS1tdWx0aS1wYXNzIG5vLW1jc3RmIG5vLXNicmMgbm8tZnJhbWUtcmOA;
a=rtcp:5004
EOF
```

```bash
vlc stream.sdp
```

## Audio: Opus

```bash
ffmpeg -f lavfi -i sine=frequency=1000:duration=30 \
-acodec libopus output.opus
```

```bash
vlc -vvv output.opus --loop --sout '#rtp{dst=127.0.0.1,port=5003}'
```

```bash
cat > stream.sdp << EOF
v=0
c=IN IP4 127.0.0.1
a=recvonly
a=type:broadcast
a=charset:UTF-8
m=audio 5003 RTP/AVP 96
b=RR:0
a=rtpmap:96 opus/48000/2
a=rtcp:5004
EOF
```

```bash
vlc stream.sdp
```

