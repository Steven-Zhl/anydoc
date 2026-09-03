# anydoc 原生命令行单文件

仓库自带的原生 CLI 位于 [src/bin/anydoc.rs](src/bin/anydoc.rs)。Cargo 会把 `src/bin/` 下的源文件自动编译成同名可执行文件，不依赖 Node.js / Python，纯 Rust、零新增依赖、不改动库代码（Python / Node / wasm 三个 binding 均不受影响）。

## 功能与官方 Node CLI 的关系

行为与官方 `npx @firecrawl/anydoc`（[node/cli.js](node/cli.js)）对齐，可直接在脚本里互换：

| 功能 | 支持 |
| --- | --- |
| `anydoc <file>` 输出 Markdown 到 stdout | ✅ |
| `anydoc -` 从 stdin 读文档 | ✅ |
| `-o, --output <path>` 写文件 | ✅ |
| `-f, --format <format>` 指定格式（否则从内容检测，扩展名兜底） | ✅ |
| `--opt=value` 内联赋值、`--` 分隔符 | ✅ |
| `-h, --help` / `-V, --version` | ✅ |
| `--assets <dir>` 导出嵌入图片/对象（官方 CLI 没有的增强） | ✅ |
| `--ocr hosted` 发送 Firecrawl Parse | ❌ 只接受默认的 `reject`（退出码 3）；需联网 OCR 请用 Node 版 |

退出码约定：`0` 成功；`1` 读取或转换失败；`2` 用法错误；`3` PDF 需要 OCR。

## 在 macOS 上编译（本机单文件）

```bash
cargo build --release --bin anydoc
# 产物：target/release/anydoc（约 6.7MB，已 strip）
```

安装到系统：

```bash
cp target/release/anydoc /usr/local/bin/     # 或 ~/.local/bin
# 或者装进 cargo bin：
cargo install --path .
```

编译 Linux 版可用 [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild)：`cargo zigbuild --release --target x86_64-unknown-linux-musl --bin anydoc` 可产出完全静态的二进制。

## 在 macOS 上交叉编译 Windows x64（exe）

本项目全部依赖为纯 Rust，交叉编译只缺链接器。推荐 `cargo-xwin`，产出与官方发布一致的 MSVC 风格二进制（`x86_64-pc-windows-msvc`）。

### 一次性环境准备

```bash
# Rust 的 Windows MSVC 标准库
rustup target add x86_64-pc-windows-msvc

# lld-link 链接器（新版 Homebrew 把 lld 从 llvm 拆成了独立 formula，
# 只装 llvm 是没有 lld-link 的）
brew install lld

# 交叉编译驱动，会自动下载微软 CRT 头文件与库
cargo install cargo-xwin
```

### PATH 说明（重要）

Homebrew 的 lld 不在默认 PATH 里，`lld-link` 找不到会导致链接失败。编译前导出：

```bash
# Apple Silicon (/opt/homebrew)；Intel 机器为 /usr/local
export PATH="/opt/homebrew/opt/lld/bin:$PATH"
```

### 编译

```bash
export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
cargo xwin build --release --target x86_64-pc-windows-msvc --bin anydoc
# 产物：target/x86_64-pc-windows-msvc/release/anydoc.exe
```

### 验证限制

macOS 上无法直接运行 `.exe`。构建成功只证明编译链接无误，行为验证可选：

- 装 Wine 跑冒烟测试（`wine target/.../anydoc.exe --help`）；
- 或交给 GitHub Actions 的 `windows-latest` runner——要正式发布给他人使用时，CI 交叉/原生构建才是正解，本机交叉编译适合自用。

### 备选：mingw-w64（GNU 风格 exe）

如果不想装 llvm，可以用 mingw，产出的 exe 功能相同（二进制稍大、启动稍慢，日常无感）：

```bash
brew install mingw-w64
rustup target add x86_64-pc-windows-gnu
```

并在 `~/.cargo/config.toml` 指定链接器：

```toml
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
```

然后 `cargo build --release --target x86_64-pc-windows-gnu --bin anydoc`。
