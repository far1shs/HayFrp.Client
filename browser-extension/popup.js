// Popup Script

const serverUrlInput = document.getElementById('serverUrl');
const apiKeyInput = document.getElementById('apiKey');
const saveConfigBtn = document.getElementById('saveConfig');
const accountListDiv = document.getElementById('accountList');
const refreshBtn = document.getElementById('refreshAccounts');
const forceSyncBtn = document.getElementById('forceSyncBtn');
const statusMessage = document.getElementById('statusMessage');

// 显示状态消息
function showStatus(message, type = 'info') {
  statusMessage.textContent = message;
  statusMessage.className = `status ${type}`;
  statusMessage.style.display = 'block';
  
  setTimeout(() => {
    statusMessage.style.display = 'none';
  }, 3000);
}

// 加载配置
async function loadConfig() {
  const { serverUrl, apiKey } = await chrome.storage.sync.get(['serverUrl', 'apiKey']);
  serverUrlInput.value = serverUrl || 'http://127.0.0.1:3737';
  apiKeyInput.value = apiKey || '';
}

// 保存配置
saveConfigBtn.addEventListener('click', async () => {
  const serverUrl = serverUrlInput.value.trim();
  const apiKey = apiKeyInput.value.trim();
  
  await chrome.storage.sync.set({ serverUrl, apiKey });
  showStatus('配置已保存', 'success');
  
  // 重新加载账号列表
  loadAccounts();
});

// 加载账号列表
async function loadAccounts() {
  accountListDiv.innerHTML = '<div class="loading">加载中...</div>';
  
  try {
    const { serverUrl, apiKey } = await chrome.storage.sync.get(['serverUrl', 'apiKey']);
    
    if (!serverUrl) {
      accountListDiv.innerHTML = '<div class="loading">请先配置服务器地址</div>';
      return;
    }
    
    const headers = {
      'Content-Type': 'application/json'
    };
    
    if (apiKey) {
      headers['X-API-Key'] = apiKey;
    }
    
    const response = await fetch(`${serverUrl}/accounts`, { headers });
    
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    
    const result = await response.json();
    
    if (!result.success) {
      throw new Error(result.message);
    }
    
    const accounts = result.data || [];
    
    if (accounts.length === 0) {
      accountListDiv.innerHTML = '<div class="loading">暂无账号</div>';
      return;
    }
    
    // 渲染账号列表（现在 csrf 已经是解密后的）
    accountListDiv.innerHTML = accounts.map(account => {
      const avatarHtml = account.avatar 
        ? `<img src="${account.avatar}" style="width: 32px; height: 32px; border-radius: 4px; margin-right: 8px;" />`
        : '';
      
      return `
        <div class="account-item" data-csrf="${account.csrf}">
          <div style="display: flex; align-items: center;">
            ${avatarHtml}
            <div>
              <div class="account-name">${account.name}</div>
              <div class="account-csrf">${account.csrf.substring(0, 20)}...</div>
            </div>
          </div>
        </div>
      `;
    }).join('');
    
    // 添加点击事件
    document.querySelectorAll('.account-item').forEach(item => {
      item.addEventListener('click', async () => {
        const csrf = item.dataset.csrf;
        await switchAccount(csrf);
      });
    });
    
  } catch (err) {
    console.error('Load accounts error:', err);
    accountListDiv.innerHTML = `<div class="loading">加载失败: ${err.message}</div>`;
    showStatus(`加载失败: ${err.message}`, 'error');
  }
}

// 切换账号
async function switchAccount(csrf) {
  try {
    // 获取当前活动标签页
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    
    if (!tab) {
      showStatus('无法获取当前标签页', 'error');
      return;
    }
    
    if (!tab.url || !tab.url.includes('console.hayfrp.com')) {
      showStatus('请在 HayFrp 控制台页面使用此功能', 'error');
      
      // 如果不在控制台页面，直接打开控制台并传入 csrf
      const consoleUrl = `https://console.hayfrp.com/proxies#csrf=${encodeURIComponent(csrf)}`;
      await chrome.tabs.create({ url: consoleUrl });
      showStatus('已在新标签页打开控制台', 'success');
      window.close();
      return;
    }
    
    // 在控制台页面，发送消息到 content script
    try {
      const response = await chrome.tabs.sendMessage(tab.id, {
        action: 'switchAccount',
        csrf: csrf
      });
      
      if (response && response.success) {
        showStatus('账号切换成功，页面即将刷新', 'success');
        setTimeout(() => window.close(), 1000);
      } else {
        showStatus('账号切换失败', 'error');
      }
    } catch (err) {
      // 如果 content script 未加载，直接刷新页面并传入 csrf
      console.log('Content script not loaded, using URL method');
      const currentUrl = new URL(tab.url);
      currentUrl.hash = `csrf=${encodeURIComponent(csrf)}`;
      await chrome.tabs.update(tab.id, { url: currentUrl.toString() });
      showStatus('正在切换账号...', 'success');
      setTimeout(() => window.close(), 1000);
    }
    
  } catch (err) {
    console.error('Switch account error:', err);
    showStatus(`切换失败: ${err.message}`, 'error');
  }
}

// 刷新按钮
refreshBtn.addEventListener('click', loadAccounts);

// 强制同步按钮
forceSyncBtn.addEventListener('click', async () => {
  try {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    
    if (!tab || !tab.url || !tab.url.includes('console.hayfrp.com')) {
      showStatus('请在 HayFrp 控制台页面使用此功能', 'error');
      return;
    }
    
    // 发送消息到 content script
    const response = await chrome.tabs.sendMessage(tab.id, {
      action: 'forceSync'
    });
    
    if (response && response.success) {
      showStatus('CSRF 已同步到服务器', 'success');
      // 刷新账号列表
      setTimeout(() => loadAccounts(), 500);
    } else {
      showStatus('同步失败: ' + (response?.error || '未知错误'), 'error');
    }
  } catch (err) {
    console.error('Force sync error:', err);
    showStatus('同步失败: ' + err.message, 'error');
  }
});

// 初始化
loadConfig();
loadAccounts();

// 监听来自 content script 的消息
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (request.action === 'accountSynced') {
    console.log('Account synced:', request.accountName);
    showStatus(`账号 ${request.accountName} 已同步`, 'success');
    // 自动刷新账号列表
    setTimeout(() => loadAccounts(), 500);
  }
});
