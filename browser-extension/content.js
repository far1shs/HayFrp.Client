// Content Script - 在 console.hayfrp.com 页面中运行

console.log('[HayFrp Helper] Content script loaded');

// 存储上次的 CSRF 值
let lastCsrf = '';
let syncInProgress = false;

// 从 URL hash 中读取 csrf 并注入
function injectCsrfFromUrl() {
  const hash = window.location.hash;
  if (hash.includes('csrf=')) {
    const match = hash.match(/csrf=([^&]+)/);
    if (match && match[1]) {
      const csrf = decodeURIComponent(match[1]);
      console.log('[HayFrp Helper] Found CSRF in URL, injecting...');
      
      // 注入到 localStorage
      localStorage.setItem('csrf', csrf);
      
      // 注入到 sessionStorage
      sessionStorage.setItem('csrf', csrf);
      
      // 注入到 cookie
      document.cookie = `csrf=${csrf}; domain=.hayfrp.com; path=/; secure; samesite=lax`;
      
      console.log('[HayFrp Helper] CSRF injected successfully');
      
      // 清除 URL 中的 csrf 参数
      const newHash = hash.replace(/[?&]?csrf=[^&]+&?/, '').replace(/^#&/, '#').replace(/#$/, '');
      window.history.replaceState(null, '', window.location.pathname + window.location.search + newHash);
      
      // 同步到服务器
      lastCsrf = csrf;
      syncCsrfToServer(csrf);
      
      // 延迟刷新页面以应用 csrf
      setTimeout(() => {
        console.log('[HayFrp Helper] Reloading page to apply CSRF...');
        window.location.reload();
      }, 500);
    }
  }
}

// 同步 CSRF 到服务器
async function syncCsrfToServer(csrf) {
  if (syncInProgress) {
    console.log('[HayFrp Helper] Sync already in progress, skipping...');
    return;
  }
  
  if (!csrf || csrf === lastCsrf) {
    console.log('[HayFrp Helper] CSRF unchanged, skipping sync');
    return;
  }
  
  syncInProgress = true;
  
  try {
    const { serverUrl, apiKey } = await chrome.storage.sync.get(['serverUrl', 'apiKey']);
    if (!serverUrl) {
      console.log('[HayFrp Helper] Server URL not configured, skipping sync');
      syncInProgress = false;
      return;
    }
    
    const headers = {
      'Content-Type': 'application/json'
    };
    
    if (apiKey) {
      headers['X-API-Key'] = apiKey;
    }
    
    console.log('[HayFrp Helper] Syncing CSRF to server:', csrf.substring(0, 10) + '...');
    
    const response = await fetch(`${serverUrl}/sync`, {
      method: 'POST',
      headers,
      body: JSON.stringify({ csrf })
    });
    
    if (response.ok) {
      const result = await response.json();
      console.log('[HayFrp Helper] CSRF synced to server:', result);
      lastCsrf = csrf;
      
      // 显示通知
      if (result.success && result.data) {
        console.log(`[HayFrp Helper] Current account: ${result.data}`);
        
        // 通知 popup 刷新账号列表
        chrome.runtime.sendMessage({
          action: 'accountSynced',
          accountName: result.data
        }).catch(() => {
          // Popup 可能未打开，忽略错误
        });
      }
    } else {
      const result = await response.json().catch(() => ({}));
      console.error('[HayFrp Helper] Failed to sync CSRF:', response.status, result);
    }
  } catch (err) {
    console.error('[HayFrp Helper] Sync error:', err);
  } finally {
    syncInProgress = false;
  }
}

// 检查并同步当前 CSRF
async function checkAndSyncCsrf() {
  const currentCsrf = localStorage.getItem('csrf');
  if (currentCsrf && currentCsrf !== lastCsrf) {
    console.log('[HayFrp Helper] Detected CSRF change, syncing...');
    await syncCsrfToServer(currentCsrf);
  }
}

// 监听 localStorage 变化（跨标签页）
window.addEventListener('storage', (e) => {
  if (e.key === 'csrf' && e.newValue) {
    console.log('[HayFrp Helper] localStorage csrf changed (cross-tab)');
    checkAndSyncCsrf();
  }
});

// 拦截 localStorage.setItem 来监听当前标签页的变化
const originalSetItem = localStorage.setItem;
localStorage.setItem = function(key, value) {
  const oldValue = localStorage.getItem(key);
  originalSetItem.apply(this, arguments);
  
  if (key === 'csrf' && value !== oldValue) {
    console.log('[HayFrp Helper] localStorage csrf changed (same tab)');
    // 延迟一下，确保值已经设置
    setTimeout(() => checkAndSyncCsrf(), 100);
  }
};

// 监听页面导航（URL 变化）
let lastUrl = location.href;
new MutationObserver(() => {
  const currentUrl = location.href;
  if (currentUrl !== lastUrl) {
    console.log('[HayFrp Helper] URL changed:', lastUrl, '->', currentUrl);
    lastUrl = currentUrl;
    
    // 检查是否从登录页跳转到其他页面
    if (currentUrl.includes('/dashboard') || 
        currentUrl.includes('/proxies') || 
        currentUrl.includes('/console')) {
      console.log('[HayFrp Helper] Navigated to main page, checking CSRF...');
      setTimeout(() => checkAndSyncCsrf(), 500);
    }
  }
}).observe(document, { subtree: true, childList: true });

// 监听页面可见性变化（切换标签页回来时检查）
document.addEventListener('visibilitychange', () => {
  if (!document.hidden) {
    console.log('[HayFrp Helper] Page became visible, checking CSRF...');
    setTimeout(() => checkAndSyncCsrf(), 500);
  }
});

// 监听页面加载完成
window.addEventListener('load', () => {
  console.log('[HayFrp Helper] Page loaded, checking CSRF...');
  const currentCsrf = localStorage.getItem('csrf');
  if (currentCsrf) {
    lastCsrf = currentCsrf;
    syncCsrfToServer(currentCsrf);
  }
});

// 定期检查（作为后备方案）
setInterval(() => {
  checkAndSyncCsrf();
}, 30000); // 每 30 秒检查一次

// 页面加载时执行
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', injectCsrfFromUrl);
} else {
  injectCsrfFromUrl();
}

// 监听来自 popup 的消息
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  console.log('[HayFrp Helper] Received message:', request);
  
  if (request.action === 'switchAccount') {
    const { csrf } = request;
    
    try {
      // 切换账号：更新 localStorage 中的 csrf
      localStorage.setItem('csrf', csrf);
      sessionStorage.setItem('csrf', csrf);
      document.cookie = `csrf=${csrf}; domain=.hayfrp.com; path=/; secure; samesite=lax`;
      
      console.log('[HayFrp Helper] Account switched, reloading...');
      
      // 立即响应成功
      sendResponse({ success: true });
      
      // 延迟刷新页面
      setTimeout(() => {
        window.location.reload();
      }, 100);
      
    } catch (err) {
      console.error('[HayFrp Helper] Switch account error:', err);
      sendResponse({ success: false, error: err.message });
    }
    
    return true; // 保持消息通道开放
  } else if (request.action === 'getCurrentCsrf') {
    // 获取当前页面的 csrf
    try {
      const csrf = localStorage.getItem('csrf') || '';
      sendResponse({ csrf });
    } catch (err) {
      sendResponse({ csrf: '', error: err.message });
    }
    return true;
  } else if (request.action === 'forceSync') {
    // 强制同步当前 CSRF
    (async () => {
      try {
        const csrf = localStorage.getItem('csrf');
        if (csrf) {
          await syncCsrfToServer(csrf);
          sendResponse({ success: true });
        } else {
          sendResponse({ success: false, error: 'No CSRF found' });
        }
      } catch (err) {
        sendResponse({ success: false, error: err.message });
      }
    })();
    return true; // 保持消息通道开放
  }
  
  return false;
});

// 初始化时检查
setTimeout(() => {
  const currentCsrf = localStorage.getItem('csrf');
  if (currentCsrf) {
    lastCsrf = currentCsrf;
    console.log('[HayFrp Helper] Initial CSRF:', currentCsrf.substring(0, 10) + '...');
  }
}, 1000);
