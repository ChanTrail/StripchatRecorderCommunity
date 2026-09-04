# StripchatRecorder Community Modules

[简体中文](README.md) | [English](README.en.md)

本仓库是 [StripchatRecorder](https://github.com/ChanTrail/StripchatRecorder) 的社区后处理模块**中央索引**。

---

## 工作原理

采用两级结构：

- **本仓库**（中央索引）：`registry.json` 只存储每个模块的序号和仓库地址，内容极少。
- **模块维护者仓库**：完整的模块元数据（名称、版本、下载链接、sha256 等）存放在各维护者自己仓库根目录的 `registry.json` 中，维护者直接更新，**无需向本仓库提 PR**。

```
中央索引 registry.json             模块维护者仓库 registry.json
──────────────────────             ─────────────────────────────────
[                                  {
  { "id": "00000001",                "id": "my_module",
    "repo": "github.com/..." }  →    "latestVersion": "1.2.0",
]                                    "downloads": { ... },
                                     "sha256": { ... }
                                   }
```

---

## 如何提交新模块（一次性操作）

1. **创建你自己的模块仓库**，使用 [`module-template/`](./module-template/) 目录中的模板。
2. **添加 `RELEASE_PAT` secret**：GitHub 头像 → Settings → Developer settings → Personal access tokens → Tokens (classic) → Generate new token (classic)，勾选 **repo** 权限后生成，复制 token；再到模块仓库 Settings → Secrets → Actions → New repository secret，Name 填 `RELEASE_PAT`，Secret 粘贴 token。
3. **只需修改三处**：
   - `Cargo.toml`：改 `name`（模块 ID）和 `version`
   - `src/main.rs`：实现处理逻辑，修改 `DESCRIBE` 中的参数定义
   - `registry.json`：填写 `description`、`tags` 和 `license`（**仅这三个字段**，其余全部由 Release workflow 自动生成）
4. **向本仓库提一次 PR**，在 `registry.json` 末尾追加一行：

```json
{ "id": "00000NNN", "repo": "https://github.com/your-username/stripchat-pp-your-module" }
```

`id` 取当前最大序号 +1，补齐 8 位。

---

## 发布新版本（完全自治，无需再次提 PR）

修改 `Cargo.toml` 中的 `version` 字段并 push 到 main：

- CI 自动推 tag
- Release workflow 自动构建、计算 sha256、更新 `registry.json`、创建 GitHub Release
- StripchatRecorder 下次刷新时自动获取新版本

---

## registry.json 格式

```json
[
  { "id": "00000001", "repo": "https://github.com/user/stripchat-pp-module" },
  { "id": "00000002", "repo": "https://github.com/user2/stripchat-pp-module2" }
]
```

| 字段   | 说明 |
|--------|------|
| `id`   | 8 位数字序号（从 `00000001` 开始，按提交顺序递增），仅用于列表排序 |
| `repo` | 模块维护者仓库的 GitHub URL |

---

## 审核标准

- `repo` 指向有效的 GitHub 公开仓库
- 仓库根目录存在可被正确解析的 `registry.json`
- 不含恶意代码，不收集用户数据，不联网访问非必要的外部服务
