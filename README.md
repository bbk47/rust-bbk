# rust-bbk (v4)

`bbk` 的 Rust 异步实现，与 Go 版 [`bbk` v4](https://github.com/bbk47/bbk) **协议级 / 字节级互通**。
客户端在本地提供 SOCKS5 / HTTP CONNECT 代理，把流量通过加密隧道转发到服务端出网。

## 架构

完全重写为基于 `tokio` 的异步模型，与 Go v4 对齐：

- **多路复用**：`yamux`（hashicorp/yamux 协议，crate `yamux`），不再使用旧的自定义 `INIT/EST/FIN` 帧。
- **加密层 `SecureConn`**：连接建立时明文交换随机 IV，之后对字节流做连续流式加解密（CFB/CTR/RC4-MD5），相同明文在不同位置得到不同密文。见 `src/tunnel/secure.rs` 与 `src/utils/encrypt.rs`。
- **流级握手**：每条 `yamux` 流先发送 `[u16 len][socks5 addr]`，服务端拨号成功后回写 `[u8 status]`（`0x00` = ready）。见 `src/tunnel/session.rs`、`src/tunnel/stream.rs`。
- **传输层**：原始字节流（无应用层分帧）。支持 `tcp` / `tls` / `ws` / `h2`，见 `src/transport`（客户端）与 `src/serve`（服务端）。
- **UDP relay**：通过特殊的 UDP marker 流承载，记录格式 `[u16 len][socks5 addr + payload]`。见 `src/proxy/udprelay.rs`。

## 构建

依赖 `openssl`（已开启 `vendored`，从源码编译，无需系统 libssl）。

```bash
cargo build --release
# 产物：target/release/bbk
```

## 运行

```bash
# 服务端
bbk -c server.json
# 客户端
bbk -c client.json
```

服务端配置示例：

```json
{
  "mode": "server",
  "listenAddr": "0.0.0.0",
  "listenPort": 5900,
  "method": "aes-256-cfb",
  "password": "p@ssword",
  "workMode": "tcp",
  "workPath": "/ws",
  "sslCrt": "",
  "sslKey": ""
}
```

客户端配置示例：

```json
{
  "mode": "client",
  "listenAddr": "127.0.0.1",
  "listenPort": 1080,
  "listenHttpPort": 1081,
  "tunnelOpts": {
    "protocol": "tcp",
    "secure": false,
    "host": "127.0.0.1",
    "port": "5900",
    "path": "/ws",
    "method": "aes-256-cfb",
    "password": "p@ssword"
  },
  "ping": false
}
```

`workMode` / `protocol` 可取 `tcp`、`tls`、`ws`、`h2`。`tls` / `h2` 需要 `sslCrt` / `sslKey`（示例证书见 `examples/tls/certs`）。

## 与 Go v4 互通状态

已通过 Rust↔Go 双向实测（SOCKS5 over TCP，以及 UDP relay）：

| 传输 | rust-client → go-server | go-client → rust-server |
|------|-------------------------|-------------------------|
| tcp  | ✅                      | ✅                      |
| ws   | ✅                      | ✅                      |
| tls  | ✅                      | ✅                      |
| udp  | ✅                      | ✅                      |
| h2   | ⚠️ 见下                 | ✅（rust-server 正常）  |

> **h2 注意**：Go v4 服务端在 `h2conn` 模式下，HTTP 处理函数会立即返回（`Server.handleConnection` 把会话丢进 goroutine 后即返回），而 `posener/h2conn` 文档明确"handler 返回即关闭连接"。因此 **Go 自身的 h2 服务端在 v4 下不可用**（Go-client → Go-server 同样失败）。rust-bbk 的 h2 客户端忠实复现了 Go 客户端的行为；rust-bbk 的 h2 服务端则正确保持连接，可与 Go 客户端互通。

## 测试

```bash
cargo test
```

测试用例（`tests/`，通过 `src/lib.rs` 暴露内部模块）：

- `encrypt_test`：各 method 加解密回环、连续密钥流位置相关性、`EVP_BytesToKey` 已知向量、不支持算法报错。
- `secure_test`：`SecureConn` 在内存管道上完成随机 IV 交换并双向收发（含 256KiB 分块大流）。
- `tunnel_test`：完整隧道集成测试——`yamux` 开流/接受流、地址握手、多路复用并发流、服务端拒绝时不挂起。
- `socks5_test`：SOCKS5 地址编解码与 `socks5_addr_len`（含带载荷场景，防止 UDP 记录解析回归）。
- `udp_test`：UDP marker 与 `[u16 len][payload]` 记录分帧回环。

> `rc4-md5` 系列使用内置 RC4 实现（OpenSSL 3 的 RC4 在 legacy provider 中，vendored 静态构建通常不可用），并已与 Go `toolbox` 实测互通。
