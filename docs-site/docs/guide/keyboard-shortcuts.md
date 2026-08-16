---
sidebar_position: 8
---

# 快捷键

NyaTerm 的快捷键分成两类理解最不容易混淆：

1. **应用级快捷键**：切换面板、开新会话、复制终端内容等
2. **Shell 级按键**：直接发送给远端或本地 shell，例如常见的 `Ctrl+C`

如果你想复制终端里的文本，请优先使用 NyaTerm 的应用级快捷键，而不是假设 shell 里的快捷键会变成复制。

## 约定

- 下表中的 **Ctrl / Cmd** 表示：Windows / Linux 使用 `Ctrl`，macOS 使用 `Cmd`
- `Ctrl+Tab` 与 `Ctrl+Shift+Tab` 保持当前实现写法

## 终端操作

| 快捷键 | 功能 |
|--------|------|
| `Ctrl / Cmd + Shift + C` | 复制 |
| `Ctrl / Cmd + Shift + V` | 粘贴 |
| `Ctrl / Cmd + Shift + X` | 粘贴选中的文本 |
| `Ctrl / Cmd + Shift + F` | 查找 |
| `Ctrl / Cmd + Shift + K` | 清屏 |
| `Ctrl / Cmd + Shift + A` | 全选 |

## 标签与工作区

| 快捷键 | 功能 |
|--------|------|
| `Ctrl / Cmd + Shift + N` | 新建会话 |
| `Ctrl / Cmd + Shift + S` | 打开命令面板 / 会话快速切换 |
| ``Ctrl / Cmd + ` `` | 新建本地终端 |
| `Ctrl / Cmd + Shift + W` | 关闭当前标签 |
| `Ctrl + Tab` | 切换到下一个标签 |
| `Ctrl + Shift + Tab` | 切换到上一个标签 |
| `Ctrl / Cmd + 1-8` | 跳转到指定标签 |
| `Ctrl / Cmd + 9` | 跳转到最后一个标签 |
| `Ctrl / Cmd + Shift + D` | 复制当前会话 |
| `Ctrl / Cmd + Shift + M` | 多路复用当前 SSH 会话 |
| `Ctrl / Cmd + Alt + D` | 复制当前会话并指定启动命令 |
| `Ctrl / Cmd + Alt + M` | 多路复用 SSH 并指定启动命令 |

## 视图与面板

| 快捷键 | 功能 |
|--------|------|
| `Ctrl / Cmd + Shift + E` | 切换左侧活动栏 / 面板 |
| `Ctrl / Cmd + Shift + B` | 切换右侧活动栏 / 面板 |
| `Ctrl / Cmd + =` | 放大 |
| `Ctrl / Cmd + -` | 缩小 |
| `Ctrl / Cmd + 0` | 重置缩放 |

## 特殊功能

| 快捷键 | 功能 |
|--------|------|
| `Ctrl / Cmd + Shift + L` | 锁定屏幕 |
| `Ctrl / Cmd + ,` | 打开设置 |
| `Ctrl / Cmd + Shift + G` | 管理同步输入组 |
| `Ctrl / Cmd + Alt + C` | 复制已选保存连接（在已保存连接面板中） |

## 使用建议

- 如果你经常在终端里复制日志，优先记住 `Ctrl / Cmd + Shift + C`
- 如果你会同时管理远程与本地会话，`Ctrl / Cmd + Shift + N` 与 ``Ctrl / Cmd + ` `` 会是最常用的两个入口
- 如果你使用锁屏功能，建议同时记住 `Ctrl / Cmd + Shift + L`

:::tip 提示
快捷键列表以当前设置页展示的实现为准。如果未来版本新增可配置快捷键，请以应用内设置中的“交互”页面为最新信息源。
:::
