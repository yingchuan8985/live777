# LiveVOD

LiveVOD 是一个轻量级回放服务，依赖本地 `index.json`（JSONL 格式）和存储后端（文件系统或 S3）。

## 配置

```toml
[http]
# listen = "0.0.0.0:8899"

# 回放索引文件路径（JSONL 或 JSON 数组格式）
index_path = "./storage/index.json"

# 本地文件系统存储（默认）
[storage]
type = "fs"
root = "./storage"

# AWS S3 存储
# [storage]
# type = "s3"
# bucket = "my-live777-bucket"
# root = "/recordings"
# region = "us-east-1"

[playback]
# signed_redirect = false   # 仅 S3：通过预签名 URL 重定向媒体分片
# signed_ttl_seconds = 60
```

## APIs

- 列出所有流：`GET /api/playback`
- 列出指定流的所有录制：`GET /api/playback/{stream}`
- 按时间戳查找录制：`GET /api/playback/{stream}/at?ts=...`
  - `ts` 支持秒、毫秒、微秒三种精度。
- 代理对象：`GET /api/record/object/{path}`

当 `playback.signed_redirect = true` 时，非 MPD 文件将通过预签名 URL 重定向。此功能需要 S3 存储，使用文件系统后端时无效。
