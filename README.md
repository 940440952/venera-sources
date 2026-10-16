# 双生态漫画源仓库

同一个漫画网站同时维护两套原生扩展：**Venera / VeneraNext / VeneraPlus 的 JavaScript 源**，以及 **Aidoku 的 Rust / WASM 源**。当前站点为 [漫画站 manhuazhan.com](https://m.manhuazhan.com)。仓库名称保留 `venera-sources`，以保持已约定的订阅地址稳定。

## Venera 系列订阅与安装

在 Venera、VeneraNext 或 VeneraPlus 的「漫画源 / 源仓库」设置中添加此订阅地址，再安装「漫画站」（不同版本的菜单名称可能略有不同）：

```text
https://cdn.jsdelivr.net/gh/940440952/venera-sources@main/index.json
```

也可下载 [venera.js](https://cdn.jsdelivr.net/gh/940440952/venera-sources@main/sources/manhuazhan/venera.js)，使用应用的本地文件导入功能安装。源要求兼容 Venera 1.6.0 及以上的源 API。

## Aidoku 订阅与安装

在支持新版源 API 的 **Aidoku 0.7.0 及以上**版本中，进入「设置 → 源列表」，添加以下 JSON 地址，再前往浏览页的添加源页面安装「漫画站」；如未显示，请确认已启用中文语言筛选。

```text
https://cdn.jsdelivr.net/gh/940440952/venera-sources@main/aidoku/index.json
```

也可从 [Releases](https://github.com/940440952/venera-sources/releases) 下载 `zh.manhuazhan-v2.aix`，通过系统分享菜单用 Aidoku 打开，或在 Aidoku 添加源界面选择本地 `.aix` 文件。Rust 源码不能直接导入 Aidoku，须使用构建后的 `.aix` 包。此实现使用新版 Aidoku API，不适用于旧版 legacy 源运行时。

首次推送后，GitHub Actions 完成检查和构建才会发布 `.aix`；如安装时提示文件不存在，请先查看 [构建状态](https://github.com/940440952/venera-sources/actions)。

## 当前支持与验证

| 功能 | Venera 系列 JS | Aidoku Rust / WASM |
| --- | --- | --- |
| 首页、全部与国产 / 日本 / 韩国 / 欧美分类 | 已实现 | 已实现 |
| 分类翻页、详情、作者、简介和章节 | 已实现 | 已实现 |
| 当前加密 DATA 图片解析、图片 Referer | 已实现 | 已实现 |
| HTML 懒加载图片与旧版 JSON newImgs | 已实现 | 已实现 |
| 关键词搜索 | 已实现；目前站点要求验证码 | 已实现；目前站点要求验证码 |
| 数字漫画 ID 搜索、漫画链接跳转 | 已实现 | 已实现 |
| 手机应用实机验证 | 待验证 | 待验证 |

2026-10-16 检查：JS 适配器通过当前站点的首页、分类、详情和章节图片请求；《斗罗大陆》（ID `219430`）返回 996 个章节条目、末章 33 张图片，首张图片下载成功。Rust 源已编译为 WASM，且两套解码实现均通过当前站点数据回归测试。这些检查不等同于所有应用、所有漫画及所有网络环境均已验证。

关键词搜索遇验证码时会明确报错，保留首页和分类入口；也可在搜索框输入漫画详情网址中的数字 ID，例如 `219430`。扩展不自动处理验证码。旧版 `newImgs` 兼容限于 JSON 数组，不执行站点任意脚本。

## 仓库结构

```text
index.json                          Venera 系列订阅目录
aidoku/index.json                   Aidoku 订阅目录
sources/
  manhuazhan/
    venera.js                      JS 实现
    aidoku/
      Cargo.toml / Cargo.lock      Rust 项目与依赖锁定
      src/                         Rust 实现与图片解析
      res/source.json              Aidoku 元数据和分类入口
tests/fixtures/manhuazhan/          两套实现共用的解码回归数据
scripts/build.py                   构建 .aix 并生成两套订阅目录
scripts/smoke.cjs                  手动执行的真实站点检查
.github/workflows/check.yml        回归检查、WASM 构建及版本发布
```

按网站划分目录，而非按阅读器拆开站点；未来新增网站时，在 `sources/<站点>/` 下加入 `venera.js` 和完整的 `aidoku/` 项目。构建脚本扫描所有站点并生成各生态目录，文件名与 Rust 包名使用同一站点标识。

## 开发与发布

需要 Node.js 22、Python 3 和支持 Rust 2024 的稳定版 Rust，并安装 `wasm32-unknown-unknown` 目标与 rustfmt。

```sh
npm ci --ignore-scripts
npm test
cargo test --locked --manifest-path sources/manhuazhan/aidoku/Cargo.toml
python3 scripts/build.py
```

`dist/` 中的 `.aix` 包含 `Payload/main.wasm`、`Payload/source.json` 及图标，可用于本地安装。仅刷新订阅目录时运行 `python3 scripts/build.py --catalogs-only`。修改 Rust 后执行 `cargo fmt --manifest-path sources/manhuazhan/aidoku/Cargo.toml`。

`npm run smoke` 访问真实站点并下载一张图片，可能受站点限流或网络影响；日常 CI 使用固定回归样例，避免把站点暂时不可达当作代码失败。

两套源独立版本：修改 JS 时递增 `venera.js` 的版本号；修改 Rust 时递增 `res/source.json` 的整数版本号。重新生成并提交两套目录。推送 `main` 后 CI 先检查 JS、Rust 测试、格式和 WASM 构建，再将 `.aix` 上传为构建产物，并按 `aidoku-zh.manhuazhan-v<版本>` 创建 Release。已存在的版本不会覆盖，修复须升版。

jsDelivr 有缓存，更新源后可能需要等待。反馈请提交 Issues，注明阅读器名称、版本、漫画 ID、章节及报错信息。

接口依据：[Venera 源 API](https://github.com/venera-app/venera/blob/master/doc/comic_source.md)、[Aidoku Rust API](https://github.com/Aidoku/aidoku-rs)、[Aidoku 源目录读取实现](https://github.com/Aidoku/Aidoku/blob/main/Aidoku/Core/Sources/SourceList/ExternalSourceInfo.swift)。
