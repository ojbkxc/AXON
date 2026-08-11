# AXON 部署指南

## 方式一:Termux(Android)

### 安装

```bash
# 1. 安装 Termux(F-Droid 版本推荐)
# 2. 在 Termux 中运行安装脚本
bash android/termux/install-termux.sh
```

### 手动安装

```bash
# 1. 下载 CI artifact(arm64-v8a 二进制)
#    https://github.com/ojbkxc/AXON/actions → 最新 run → axon-arm64-v8a

# 2. 放到 Termux
mkdir -p ~/.axon
cp axon ~/.axon/
cp config.example.yaml ~/.axon/config.yaml

# 3. 配置 API key
export OPENAI_API_KEY="sk-..."

# 4. 启动
~/.axon/axon --config ~/.axon/config.yaml &

# 5. 访问
termux-open-url http://localhost:8080/ui/
```

## 方式二:Android APK

### 从 CI 下载

```bash
# GitHub Actions → android-apk.yml → 最新 run → artifact: axon-apk-arm64-v8a
# 下载 AXON-v1.0.1-android-arm64-v8a.apk
# 安装到设备(需开启未知来源安装)
```

### APK 行为

- 启动后 `AxonService` 前台服务运行 axon 二进制
- WebView 加载 `http://127.0.0.1:8080/ui/`
- 二进制从 assets 提取到私有目录
- 通知栏显示运行状态

## 方式三:源码构建

### 前置

- Rust stable(MSRV 1.75+)
- Node.js 18+(UI 构建)

### 构建

```bash
# 1. 构建 UI
cd ui && npm install && npm run build && cd ..

# 2. 构建 server(UI 嵌入)
cargo build --release -p axon-server

# 3. 检查体积
ls -lh target/release/axon  # 应 < 50MB
```

### 运行

```bash
# 设置 API key
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."

# 启动
./target/release/axon --config config.example.yaml

# 或指定地址和日志级别
./target/release/axon --config config.example.yaml --addr 0.0.0.0:9090 --log-level debug
```

## 方式四:Android 交叉编译

```bash
# 1. 安装 NDK + cargo-ndk
rustup target add aarch64-linux-android
cargo install cargo-ndk

# 2. 交叉编译
bash scripts/cross-android.sh

# 3. 构建 APK
bash scripts/build-apk.sh
```

## CLI 参数

```
axon [OPTIONS]

Options:
  --config <PATH>      配置文件路径(默认 config.yaml)
  --addr <ADDR>        监听地址(覆盖配置)
  --log-level <LEVEL>  日志级别(覆盖配置)
  -h, --help           帮助
```

## 环境变量

| 变量 | 说明 |
|---|---|
| OPENAI_API_KEY | OpenAI API key |
| ANTHROPIC_API_KEY | Anthropic API key |
| DEEPSEEK_API_KEY | DeepSeek API key |
| AXON_LOG_FORMAT | 日志格式(`plain`/`json`,覆盖配置) |

## 体积与内存

| 指标 | 目标 | 实测 |
|---|---|---|
| 二进制大小 | < 50MB(目标 < 30MB) | 5.1MB(arm64) |
| 启动内存 | < 200MB(目标 < 150MB) | 待真机实测 |

## 验证

```bash
# 健康检查
curl http://localhost:8080/healthz

# 就绪检查
curl http://localhost:8080/readyz

# 服务状态
curl http://localhost:8080/status

# Prometheus 指标
curl http://localhost:8080/metrics
```
