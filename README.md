<div align="center">

# Codex Web Search

**使用你自己的 Codex 凭据，模拟 Codex 客户端请求，直接调用网页搜索功能**

可以作为其他 agent 的搜索工具使用或个人搜索引擎

![Version](https://img.shields.io/badge/version-0.1.0-blue)
![Rust](https://img.shields.io/badge/Rust-2024_edition-orange?logo=rust)
![License](https://img.shields.io/badge/license-Apache--2.0-green)

**中文** · [English](README.en.md)

[快速开始](#快速开始) · [使用示例](#使用示例) · [认证配置](#认证配置) · [命令行选项](#命令行选项) 
</div>

## 快速开始

### 1. 安装

需要支持 **Rust 2024 edition** 的工具链。在项目目录运行：

```sh
cargo install --path . --locked
```

也可以仅编译，使用 `./target/release/codex-web-search` 运行：

```sh
cargo build --release --locked
```

### 2. 配置认证

指定已有的 Codex 登录文件：

```sh
export CODEX_AUTH_FILE="$HOME/.codex/auth.json"
```

程序不会自动发现认证文件。使用自定义目录时，请填写实际路径；凭据仅保存在系统钥匙串中时，使用下方的 [access token 方式](#直接提供-access-token)。

### 3. 发起搜索

```sh
codex-web-search --query '2026年11月10日上海天气'
```

## 使用示例

以下示例均假设认证已配置成功

### 搜索与域名过滤

普通搜索直接使用 `--query`：

```sh
codex-web-search --query '上海新能源汽车补贴政策'
```

如果希望只搜索指定网站，可以用 JSON 一起传入搜索词、网站域名和返回长度。下面的命令查询上海新能源汽车补贴的申请条件，并将搜索范围限定在上海市政府网站：

```sh
printf '%s\n' '{"search_query":[{"q":"上海新能源汽车补贴政策 申请条件","domains":["shanghai.gov.cn"]}],"response_length":"short"}' |
  codex-web-search --session-id shanghai-ev-policy
```

JSON 中的参数含义如下：

| 参数 | 含义 |
| --- | --- |
| `search_query` | 搜索任务列表，每个对象代表一个搜索任务。 |
| `q` | 搜索词，这里是“上海新能源汽车补贴政策 申请条件”。 |
| `domains` | 限定搜索的网站域名；不需要限制网站时，可以省略这个字段。 |
| `response_length` | 返回结果的长度；这里用 `short` 请求简短结果。 |

`--session-id` 给这次搜索指定一个会话 ID，后续打开结果或查找文本时继续使用它即可。

### 打开结果与查找文本

复用同一个 `--session-id`，并将 `turn0search0` 替换为上一步实际返回的引用 ID：

```sh
printf '%s\n' '{"open":[{"ref_id":"turn0search0"}]}' |
  codex-web-search --session-id shanghai-ev-policy

printf '%s\n' '{"find":[{"ref_id":"turn0search0","pattern":"申请条件"}]}' |
  codex-web-search --session-id shanghai-ev-policy
```

未指定会话 ID 时，程序会自动生成 UUID，并在结果中返回。

### 在其他程序中调用

任何支持启动子进程的程序都可以调用此工具。下面的 Python 示例继承父进程中的认证环境变量，并读取明文结果：

```python
import json
import subprocess

result = subprocess.run(
    ["codex-web-search", "--session-id", "agent-research"],
    input=json.dumps({"search_query": [{"q": "上海新能源汽车补贴政策"}]}),
    text=True,
    capture_output=True,
    check=True,
)
response = json.loads(result.stdout)
print(response["output"])
```

## 认证配置

本项目承诺不会上传用户 Codex 的认证信息。

### 使用 Codex 登录文件

使用 `--auth-file` 显式指定路径，或设置 `CODEX_AUTH_FILE`：

```sh
codex-web-search \
  --auth-file "$HOME/.codex/auth.json" \
  --query '上海新能源汽车补贴政策'
```

程序需要读取文件中的 `tokens.access_token` 和可选的 `tokens.account_id`，不会对认证文件做任何其他操作。

### 直接提供 access token

在 Bash 中，通过环境变量提供 token：

```sh
unset CODEX_AUTH_FILE
read -rs -p 'Access token: ' CODEX_ACCESS_TOKEN
printf '\n'
export CODEX_ACCESS_TOKEN
codex-web-search --query '上海新能源汽车补贴政策'
```

设置了 `--auth-file` 或 `CODEX_AUTH_FILE` 时，优先使用文件认证。

本项目不执行登录或自动刷新 token。遇到 HTTP 401 时，请更新 token，或通过 Codex 重新登录后再读取认证文件。

## 返回结果

以下为示意输出：

```json
{
  "session_id": "shanghai-ev-policy",
  "output": "搜索结果正文，可能包含页面引用编号和来源信息……",
  "results": [],
  "encrypted_output": "..."
}
```

| 字段 | 含义 |
| --- | --- |
| `session_id` | 本次搜索会话 ID |
| `output` | 搜索结果正文 |
| `results` | 结构化返回结果，包括 url 等 |
| `encrypted_output` | 加密内容 |

注：Agent 集成使用 `output` 即可。在旧版 codex 中，搜索返回内容放在 `encrypted_output` 中，现该字段已失效。

## 命令行选项

| 选项 | 说明 | 默认值 |
| --- | --- | --- |
| `--query` | 单个搜索词；不传则从 stdin 读取命令 JSON。 | 无 |
| `--auth-file` | Codex 认证文件；也可设置 `CODEX_AUTH_FILE`。 | 无 |
| `--session-id` | 关联搜索、打开和查找等请求。 | 自动生成 UUID |
| `--model` | 提交给搜索服务的模型名。 | `gpt-6-astra` |
| `--mode` | `live`、`cached` 或 `indexed`。 | `live` |


`live` 允许实时联网，`cached` 请求缓存模式，`indexed` 请求索引模式；实际行为和模型可用性由搜索服务决定。

## 使用须知

本项目调用的是 Codex 使用的 `alpha/search` 接口，不是公开 Responses API 中的托管 `web_search`。这不是 OpenAI 官方 SDK，因此不保证该接口未来保持兼容。

## 许可证

项目在 `Cargo.toml` 中声明采用 **Apache-2.0** 许可证。
