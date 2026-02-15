# HayFrp Console Helper - 浏览器插件

HayFrp 控制台助手浏览器插件，用于自动注入 CSRF 和快速切换账号。

## 功能

1. **自动注入 CSRF**: 从 URL hash 中读取 CSRF 并自动注入到 localStorage、sessionStorage 和 Cookie
2. **快速切换账号**: 通过插件 popup 快速切换不同的 HayFrp 账号
3. **与桌面应用通信**: 通过 HTTP API 与 HayFrp 桌面应用同步账号信息

## 安装方法

### Chrome/Edge

1. 打开浏览器，进入扩展管理页面
   - Chrome: `chrome://extensions/`
   - Edge: `edge://extensions/`

2. 开启"开发者模式"

3. 点击"加载已解压的扩展程序"

4. 选择 `browser-extension` 文件夹

### Firefox

1. 打开 `about:debugging#/runtime/this-firefox`

2. 点击"临时加载附加组件"

3. 选择 `browser-extension/manifest.json` 文件

## 使用方法

### 1. 配置服务器

1. 点击浏览器工具栏中的插件图标
2. 在弹出的窗口中配置:
   - **服务器地址**: 默认 `http://127.0.0.1:3737`
   - **API Key**: 在桌面应用的设置中配置的 API Key
3. 点击"保存配置"

### 2. 从桌面应用打开控制台

1. 在桌面应用的隧道页面，点击账号旁边的地球图标
2. 浏览器会自动打开控制台页面
3. 插件会自动读取 URL 中的 CSRF 并注入
4. 页面会自动刷新以应用 CSRF

### 3. 快速切换账号

1. 在 `console.hayfrp.com` 页面，点击插件图标
2. 在账号列表中点击要切换的账号
3. 页面会自动刷新并切换到选中的账号

## 工作原理

1. **URL 注入**: 桌面应用打开浏览器时，在 URL hash 中传入 CSRF
2. **Content Script**: 插件的 content script 读取 URL 中的 CSRF 并注入到页面
3. **自动监听**: 插件监听多个事件来检测 CSRF 变化：
   - localStorage 变化（跨标签页和当前标签页）
   - 页面 URL 变化（如从 /login 跳转到 /dashboard）
   - 页面可见性变化（切换标签页回来时）
   - 定期检查（每 30 秒）
4. **自动同步**: 检测到 CSRF 变化时，自动调用 `/sync` API 同步到桌面应用
5. **HTTP API**: 插件通过 HTTP API 与桌面应用通信，获取所有账号信息
6. **账号切换**: 点击账号时，插件更新 localStorage 并刷新页面

## 监听机制

插件使用多种方式监听 CSRF 变化：

1. **localStorage.setItem 拦截**: 监听当前标签页的 localStorage 修改
2. **storage 事件**: 监听跨标签页的 localStorage 变化
3. **MutationObserver**: 监听页面 URL 变化（SPA 路由）
4. **visibilitychange 事件**: 标签页切换回来时检查
5. **定期轮询**: 每 30 秒检查一次作为后备方案

## 触发同步的场景

- 从桌面应用打开浏览器（URL 注入）
- 用户在控制台手动登录/切换账号
- 从 /login 页面跳转到 /dashboard 等页面
- 切换标签页回到控制台页面
- 使用插件切换账号
- 点击"同步当前"按钮强制同步

## API 端点

桌面应用提供以下 HTTP API:

- `GET /health` - 健康检查
- `GET /accounts` - 获取所有账号信息 (需要 API Key)
- `POST /csrf` - 更新账号的 CSRF (需要 API Key)

## 注意事项

1. 确保桌面应用的 HTTP Server 已启用
2. 如果设置了 API Key，插件配置中也需要填写相同的 Key
3. 插件只在 `console.hayfrp.com` 域名下工作
4. 切换账号会刷新页面，未保存的数据会丢失

## 开发

如果需要修改插件代码:

1. 修改 `content.js`、`popup.js` 或 `background.js`
2. 在浏览器扩展管理页面点击"重新加载"
3. 刷新 `console.hayfrp.com` 页面以应用更改

## 图标

插件需要以下尺寸的图标:
- `icon16.png` - 16x16
- `icon48.png` - 48x48
- `icon128.png` - 128x128

可以使用任何图标生成工具创建这些图标。
