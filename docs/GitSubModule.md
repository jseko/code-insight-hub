# Git Submodule 使用指南

## 目录

- [什么是 Git Submodule](#什么是-git-submodule)
- [基本操作](#基本操作)
- [常用命令](#常用命令)
- [Windows 下的特殊问题](#windows-下的特殊问题)
- [最佳实践](#最佳实践)
- [常见问题](#常见问题)

---

## 什么是 Git Submodule

Git Submodule（子模块）允许你将一个 Git 仓库作为另一个 Git 仓库的子目录。当你需要引用其他项目的代码时，子模块是非常有用的工具。

**典型使用场景：**

- 使用第三方库或框架
- 集成共享组件库
- 分离独立的项目模块
- 管理大型项目中的子项目

---

## 基本操作

### 添加子模块

```bash
# 添加子模块（克隆完整历史）
git submodule add <repository_url> <path>

# 添加子模块（浅克隆，仅最新提交）
git submodule add --depth=1 <repository_url> <path>

# 示例
git submodule add git@github.com:rancher/rancher.git ./vendors/rancher
```

### 初始化和更新子模块

```bash
# 初始化子模块（首次克隆后）
git submodule init

# 更新子模块到最新提交
git submodule update

# 一步完成初始化和更新
git submodule update --init

# 克隆包含子模块的项目时，直接拉取所有子模块
git clone --recursive <repository_url>
```

### 查看子模块状态

```bash
# 查看所有子模块状态
git submodule status

# 查看子模块详细信息
git submodule foreach 'echo $path `git rev-parse HEAD`'
```

### 删除子模块

```bash
# 步骤 1：取消子模块注册
git submodule deinit -f <path>

# 步骤 2：删除残留的 Git 数据
git rm -f <path>

# 步骤 3：手动删除 .git/modules 下的相关目录（可选）
rm -rf .git/modules/<path>
```

### 更新子模块到指定提交

```bash
# 进入子模块目录
cd <path>

# 检出到指定的提交或分支
git checkout <commit_hash>

# 返回主项目
cd ..

# 提交子模块更新
git add <path>
git commit -m "Update submodule to <commit_hash>"
```

---

## 常用命令

| 命令 | 说明 |
|------|------|
| `git submodule add <url> <path>` | 添加子模块 |
| `git submodule init` | 初始化子模块 |
| `git submodule update` | 更新子模块 |
| `git submodule update --init` | 初始化并更新子模块 |
| `git submodule update --remote` | 更新子模块到远程最新 |
| `git submodule foreach <command>` | 在所有子模块中执行命令 |
| `git submodule deinit -f <path>` | 移除子模块注册 |
| `git submodule status` | 查看子模块状态 |

---

## Windows 下的特殊问题

### 问题描述

在 Windows 系统上使用 Git Submodule 时，可能会遇到以下问题：

**核心原因**：Windows 文件系统对路径中的特殊字符（`&`、`[]`）和路径长度有严格限制。

**具体表现**：某些仓库（如 rancher）中包含特殊字符的文件路径，例如：

```
tests/v2/integration/steveapi/json/user-a_test-ns-1_filter=metadata.fields[2]>0&sort=metadata.fields[2].json
```

这会导致 Git 无法在 Windows 上检出该文件，最终子模块 checkout 失败。

---

### 解决方案

#### 步骤 1：清理失败的子模块残留

```powershell
# 1. 取消子模块注册
git submodule deinit -f ./vendors/rancher

# 2. 删除残留的 Git 数据（PowerShell）
Remove-Item -Path .git\modules\vendors\rancher -Recurse -Force

# 3. 删除工作区空目录
git rm -f ./vendors/rancher
```

#### 步骤 2：配置 Windows + Git 兼容特殊路径/长路径

##### 2.1 启用 Windows 长路径支持

Windows 默认限制文件路径长度为 260 字符，需先解除限制：

**方式 1：通过注册表（推荐，无需管理员）**

打开 PowerShell（普通权限），执行：

```powershell
Set-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem" -Name "LongPathsEnabled" -Value 1
```

> 若提示权限不足，右键 PowerShell → 以管理员身份运行再执行。

**方式 2：通过组策略（需专业版/企业版 Windows 11）**

1. 按 `Win+R` 输入 `gpedit.msc`
2. 依次展开：`计算机配置 → 管理模板 → 系统 → 文件系统 → NTFS`
3. 找到「启用 NTFS 长路径」→ 双击设置为「已启用」→ 确定。

##### 2.2 配置 Git 兼容 Windows 特殊字符

```powershell
# 允许 Git 识别 Windows 下的特殊路径字符
git config --global core.protectNTFS false

# 启用 Git 长路径支持（兜底）
git config --global core.longpaths true
```

#### 步骤 3：重新添加子模块（浅克隆）

```powershell
git submodule add --depth=1 git@github.com:rancher/rancher.git ./vendors/rancher
```

#### 步骤 4：验证是否成功

```powershell
# 查看子模块状态（显示 commit ID 即成功）
git submodule status

# 检查子模块目录是否有文件
ls ./vendors/rancher
```

---

### 备选方案：排除包含特殊路径的目录

如果上述配置后仍因特殊路径文件报错，可通过「只克隆核心目录」规避：

```powershell
# 1. 先创建空的子模块目录
mkdir -Force ./vendors/rancher

# 2. 进入目录，初始化 Git 并浅克隆（排除 tests 目录）
cd ./vendors/rancher
git init
git remote add origin git@github.com:rancher/rancher.git
git fetch --depth=1 origin main
git checkout FETCH_HEAD -- . ':!tests'  # 排除 tests 目录（含特殊路径）

# 3. 回到主仓库，注册子模块
cd ../..
git add .gitmodules ./vendors/rancher
git commit -m "add rancher submodule (skip tests dir to avoid Windows path error)"
```

> **注意**：此方案会排除 `tests` 目录，如不需要测试文件可以使用此方法。

---

## 最佳实践

### 1. 使用 `.gitmodules` 文件

`.gitmodules` 文件记录了所有子模块的信息，建议提交到版本控制：

```ini
[submodule "vendors/rancher"]
    path = vendors/rancher
    url = git@github.com:rancher/rancher.git
```

### 2. 固定子模块版本

子模块始终指向特定的 commit，确保项目构建的稳定性：

```bash
# 在子模块目录中查看当前提交
cd vendors/rancher
git log -1

# 在主项目中锁定子模块版本
cd ../..
git add vendors/rancher
git commit -m "Lock rancher submodule to specific commit"
```

### 3. 定期更新子模块

```bash
# 更新所有子模块到远程最新
git submodule update --remote

# 更新特定子模块
git submodule update --remote <path>

# 更新并合并到当前分支
git submodule update --remote --merge
```

### 4. 团队协作

克隆项目时：

```bash
# 克隆时自动拉取子模块
git clone --recursive <repository_url>

# 或者克隆后手动初始化
git clone <repository_url>
cd <project>
git submodule update --init
```

### 5. 性能优化

对于大型子模块，使用浅克隆减少下载时间和磁盘占用：

```bash
# 仅克隆最新提交
git submodule add --depth=1 <url> <path>

# 或指定深度
git submodule add --depth=5 <url> <path>
```

---

## 常见问题

### Q：子模块和 Git subtree 有什么区别？

**子模块（Submodule）**：
- 独立的 Git 仓库
- 需要显式更新
- 适合第三方库

**Git Subtree**：
- 将子项目代码合并到主项目
- 自动同步
- 适合紧密耦合的项目

### Q：为什么子模块显示为空目录？

子模块初始化后需要更新：

```bash
git submodule update --init
```

### Q：如何在子模块中开发？

进入子模块目录后，可以像正常仓库一样操作：

```bash
cd vendors/rancher
# 正常开发、提交、推送
git add .
git commit -m "Your changes"
git push
```

### Q：子模块更新后，主项目会自动更新吗？

不会。子模块指向固定的 commit，需要手动更新子模块并在主项目中提交：

```bash
git submodule update --remote
git add vendors/rancher
git commit -m "Update submodule"
```

### Q：Windows 下为什么会出现特殊路径错误？

Windows 文件系统对路径中的 `&`、`[]` 等特殊字符有限制，而某些项目（如 rancher）的文件路径包含这些字符。解决方法见上方[Windows 下的特殊问题](#windows-下的特殊问题)章节。

### Q：是否每次克隆都需要配置 Windows 兼容性？

Windows 长路径支持和 Git 配置只需执行一次，后续会自动应用。

---

## 参考资料

- [Git 官方文档 - Git Tools - Submodules](https://git-scm.com/book/en/v2/Git-Tools-Submodules)
- [Pro Git 中文版 - 子模块](https://git-scm.com/book/zh/v2/Git-工具-子模块)
