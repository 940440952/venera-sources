# VeneraPlus 漫画源仓库

本仓库维护供 Venera / VeneraPlus 使用的 JavaScript 漫画源，目前维护 **漫画站（manhuazhan.com）**。

当前状态：仓库已初始化，原版 `manhuazhan.js` 待恢复上传。订阅目录暂为空，尚不能安装漫画站源。

## 订阅地址

原版脚本上传后，在 VeneraPlus 的漫画源设置中添加：

```text
https://cdn.jsdelivr.net/gh/940440952/venera-sources@main/index.json
```

预定脚本地址（待上传后生效）：

```text
https://cdn.jsdelivr.net/gh/940440952/venera-sources@main/sources/manhuazhan.js
```

## 维护

- 站点：https://m.manhuazhan.com
- 源脚本：`sources/manhuazhan.js`（待恢复原版文件）。
- 上传后同步填写 `index.json` 中的名称、版本和 jsDelivr 脚本地址。
- 修改源脚本时，同步更新脚本和订阅目录的版本号。
- GitHub Actions 检查 `sources/*.js` 的 JavaScript 语法；语法检查不代表实机功能可用。
- 搜索可能遇到验证码，阅读功能仍待 VeneraPlus 实机验证。
- 问题反馈请在 Issues 提供应用版本、操作步骤和报错信息。
- jsDelivr 可能缓存旧文件，更新后未立即生效时请稍后重试。

源仓库格式参考：[Venera Comic Source 文档](https://github.com/venera-app/venera/blob/master/doc/comic_source.md#comic-source-list)。
